use crate::userbool::UserBool;
use crate::{
    config::{Config, LoadedConfig},
    template::Template,
    ui::{self},
    walkdir,
};
use colored::Colorize;
use futures::StreamExt;
use parking_lot::RwLock;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use read_input::prelude::*;

const ERR_NAME_TAKEN: &str = "There is already a template of that name.";

pub fn make(
    config: &mut LoadedConfig,
    template_name: String,
    template_dir: PathBuf,
    template_description: Option<String>,
) {
    if config.config.templates.contains_key(&Config::get_template_key(&template_name)) {
        println!("{}", ERR_NAME_TAKEN.red());
        std::process::exit(exitcode::USAGE);
    }

    let file_list = {
        let mut ui_state = crate::ui::file::FilePickerUi::new(&template_dir);
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
        let base_path = template_dir.clone();
        let target_path = target_base_dir.clone();
        let files_list = Arc::new(file_list);
        let files_memo = Arc::new(RwLock::new(HashMap::<PathBuf, bool>::new()));
        async move {
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
