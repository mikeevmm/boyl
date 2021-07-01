use crate::userbool::UserBool;
use crate::{
    config::{Config, LoadedConfig},
    template::Template,
    ui::{self},
    userpath::UserDir,
    walkdir,
};
use colored::Colorize;
use futures::StreamExt;
use parking_lot::RwLock;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use read_input::prelude::*;

const ERR_NAME_TAKEN: &str = "There is already a template of that name.";

pub fn make(config: &mut LoadedConfig) {
    let current_dir = std::env::current_dir().ok();

    let template_dir = {
        let template_dir_default = current_dir.map(UserDir::from);
        let prompt = match &template_dir_default {
            Some(default) => format!(
                "Template directory {}: ",
                format!("[default: {}]", default.path_buf.to_string_lossy()).dimmed()
            ),
            None => "Template directory: ".to_string(),
        };
        match template_dir_default {
            Some(default) => input()
                .repeat_msg(prompt)
                .default(default)
                .err_match(|err| Some(err.to_string().red().to_string()))
                .get(),
            None => input::<UserDir>()
                .repeat_msg(prompt)
                .err_match(|err| Some(err.to_string().red().to_string()))
                .get(),
        }
    };

    let template_name = {
        let template_name_default = template_dir
            .path_buf
            .file_name()
            .map(|s| s.to_string_lossy().to_string());
        let prompt = match &template_name_default {
            Some(default) => format!(
                "Template name {}: ",
                format!("[default: {}]", default).dimmed()
            ),
            None => "Template name: ".to_string(),
        };
        let mut answer;
        loop {
            answer = match template_name_default.clone() {
                Some(default) => input().msg(prompt.clone()).default(default).get(),
                None => input::<String>().msg(prompt.clone()).get(),
            };

            // A bit hacky: `rprompt` validation funcitons require 'static lifetime,
            // which cannot be satisfied for this check.
            if config
                .config
                .templates
                .values()
                .map(|t| &t.name)
                .any(|n| *n == answer)
            {
                println!("{}", ERR_NAME_TAKEN.red());
            } else {
                break;
            }
        }
        answer
    };

    let template_description = {
        let user = input::<String>()
            .msg(format!(
                "Template description {}: ",
                "(Leave empty for none)".dimmed()
            ))
            .get();
        if user.is_empty() {
            None
        } else {
            Some(user)
        }
    };

    let file_list = {
        let mut ui_state = crate::ui::file::FilePickerUi::new(&template_dir.path_buf);
        ui::run_ui(&mut ui_state);

        if ui_state.aborted {
            std::process::exit(exitcode::USAGE);
        }
        ui_state.file_list
    };

    // We now copy the files to the templates directory, and store a new template in memory.
    let target_base_dir = config.get_template_dir().join(&template_name);

    if target_base_dir.exists() {
        println!(
            "{}",
            "The template base directory already exists.\n\
        This may be because you previously aborted the creation of a template of \
        the same name."
                .red()
        );
        let erase_and_continue = input::<UserBool>()
            .repeat_msg(
                format!(
                    "Do you wish to delete the existing directory and continue? {} ",
                    "[y/N]".dimmed()
                )
                .yellow(),
            )
            .default(false.into())
            .get();

        match erase_and_continue.value {
            true => {
                std::fs::remove_dir_all(&target_base_dir)
                    .expect("Could not remove the existing directory.");
                std::fs::create_dir(&target_base_dir)
                    .expect("Could not create template directory.");
            }
            false => {
                println!("Aborting.");
                std::process::exit(exitcode::CONFIG);
            }
        }
    }

    if let Err(err) = std::fs::create_dir(&target_base_dir) {
        println!(
            "Could not create the template base directory, with error: {}",
            err
        );
        std::process::exit(exitcode::IOERR);
    }

    let tokio_runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();
    tokio_runtime.block_on({
        let base_path = template_dir.path_buf.clone();
        let target_path = target_base_dir.clone();
        let files_list = Arc::new(file_list);
        let files_memo = Arc::new(RwLock::new(HashMap::<PathBuf, bool>::new()));
        async move {
            // files_list.is_included_memoized_async(&f.path(), files_memo.clone());
            let files_to_include = Box::pin(walkdir::visit(&base_path).filter_map({
                clone_move!(files_list);
                clone_move!(files_memo);
                move |x| {
                    clone_move!(files_list);
                    clone_move!(files_memo);
                    async move {
                        match x {
                            Ok(x) => {
                                if files_list.is_included_memoized_async(&x.path(), files_memo) {
                                    Some(x)
                                } else {
                                    None
                                }
                            }
                            Err(e) => {
                                println!("Ignoring file: {}", e);
                                None
                            }
                        }
                    }
                }
            }));
            crate::copy::recursive_copy(&base_path, &target_path, files_to_include).await;
        }
    });

    println!("New template {} was created.", template_name.bold());
    println!(
        "{} {} {}",
        "Call".dimmed(),
        format!("boyl new {}", template_name).green(),
        "to create a new instance of this template.".dimmed()
    );

    let new_template = Template {
        name: template_name,
        description: template_description,
        path: target_base_dir,
    };
    let new_template_key = Config::get_template_key(&new_template.name);
    config
        .config
        .templates
        .insert(new_template_key, new_template);
}
