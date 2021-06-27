pub mod new {
    pub const CMD_STR: &str = "new";
    pub const TEMPLATE_ARG: &str = "TEMPLATE";
    pub const NAME_ARG: &str = "NAME";
    pub const LOCATION_ARG: &str = "LOCATION";
}

pub mod make {
    use std::{
        collections::{HashMap, VecDeque},
        error::Error,
        fmt::Debug,
        path::PathBuf,
        str::FromStr,
        sync::Arc,
    };

    use crate::{
        config::LoadedConfig,
        template::Template,
        ui::{self, spinner::Spinner},
        Verbosity,
    };
    use colored::Colorize;
    use parking_lot::Mutex;
    use read_input::prelude::*;
    use termion::terminal_size;
    use tokio::sync::mpsc;

    pub const CMD_STR: &str = "make";

    const ERR_PATH: &str = "Cannot understand path.";
    const ERR_NO_EXIST: &str = "Path does not exist.";
    const ERR_NOT_DIR: &str = "Path is not a directory.";
    const ERR_NAME_TAKEN: &str = "There is already a template of that name.";

    #[derive(Clone)]
    struct UserPath {
        path_buf: PathBuf,
    }

    impl Debug for UserPath {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.path_buf.fmt(f)
        }
    }

    impl FromStr for UserPath {
        type Err = Box<dyn Error>;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let expanded = shellexpand::full(s)?;
            let path_buf = PathBuf::from_str(&expanded)?;
            Ok(UserPath { path_buf })
        }
    }

    impl From<PathBuf> for UserPath {
        fn from(path_buf: PathBuf) -> Self {
            UserPath { path_buf }
        }
    }

    struct UserBool {
        value: bool,
    }

    impl From<bool> for UserBool {
        fn from(value: bool) -> Self {
            UserBool { value }
        }
    }

    impl FromStr for UserBool {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let s = s.to_lowercase();
            if s == "n" || s == "no" || s == "false" {
                Ok(false.into())
            } else if s == "y" || s == "yes" || s == "true" {
                Ok(true.into())
            } else {
                Err(format!("Cannot understand {}", s))
            }
        }
    }

    pub fn make(config: &mut LoadedConfig, verbosity: Verbosity) {
        let current_dir = std::env::current_dir().ok();

        let template_dir = {
            let template_dir_default = current_dir.map(UserPath::from);
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
                    .err(ERR_PATH.red())
                    .add_err_test(|p| p.path_buf.exists(), ERR_NO_EXIST.red())
                    .add_err_test(|p| p.path_buf.is_dir(), ERR_NOT_DIR.red())
                    .get(),
                None => input::<UserPath>()
                    .repeat_msg(prompt)
                    .err(ERR_PATH.red())
                    .add_err_test(|p| p.path_buf.exists(), ERR_NO_EXIST.red())
                    .add_err_test(|p| p.path_buf.is_dir(), ERR_NOT_DIR.red())
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
                return;
            }
            ui_state.file_list
        };

        // We now copy the files to the templates directory, and store a new template in memory.
        // Copying is done on a Tokio runtime, to make use of all threads without manual thread
        // management.
        let target_base_dir = config.get_template_dir().join(&template_name);
        if let Err(err) = std::fs::create_dir(&target_base_dir) {
            match err.kind() {
                std::io::ErrorKind::AlreadyExists => {
                    println!(
                        "The template base directory already exists.\n\
                    This may be because you previously aborted the creation of a template of \
                    the same name."
                    );
                    let erase_and_continue = input::<UserBool>()
                        .repeat_msg(format!(
                            "Do you wish to delete the existing directory and continue? {} ",
                            "[y/N]".dimmed()
                        ))
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
                            return;
                        }
                    }
                }
                _ => panic!(
                    "Could not create the template base directory, with error: {}",
                    err
                ),
            }
        }

        let tokio_runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();
        tokio_runtime.block_on({
            let target_base_dir = target_base_dir.clone();
            async {
                let mut files_memo = HashMap::<PathBuf, bool>::new();
                let files_to_include = walkdir::WalkDir::new(&template_dir.path_buf)
                    .min_depth(1)
                    .into_iter()
                    .flatten()
                    .filter(|f| file_list.is_included_memoized(f.path(), &mut files_memo));

                let copy_queue = Arc::new(Mutex::new(VecDeque::<(PathBuf, PathBuf)>::new()));
                let mut copy_handlers = vec![];
                // TODO: Make this a less arbitrary number
                for _copy_handler_i in 0..20 {
                    let copy_queue = copy_queue.clone();
                    let copy_handler = tokio::spawn(async move {
                        while let Some((from, to)) = {
                            let mut lock = copy_queue.lock();
                            let value = lock.pop_back();
                            drop(lock);
                            value
                        } {
                            if from.is_dir() {
                                tokio::fs::create_dir_all(to).await.unwrap();
                            } else {
                                tokio::fs::create_dir_all(to.as_path().parent().unwrap())
                                    .await
                                    .unwrap();
                                tokio::fs::copy(from, to).await.unwrap();
                            };
                        }
                    });
                    copy_handlers.push(copy_handler);
                }

                let mut spinner = Spinner::new();
                for file in files_to_include {
                    let spinner_symbol = spinner.tick();

                    let base_file = file.path().strip_prefix(&template_dir.path_buf).unwrap();
                    let display_path = &base_file.to_string_lossy();
                    let term_width = terminal_size().map(|(w, _)| w).unwrap_or(5) as usize;
                    let max_path_width = term_width.saturating_sub(5);
                    let space_width = term_width
                        .saturating_sub(std::cmp::min(max_path_width, display_path.len()) + 5);

                    print!("{}{} ", spinner_symbol, " ".repeat(space_width / 2));
                    if display_path.len() > max_path_width {
                        print!("{}", &display_path[display_path.len() - max_path_width..]);
                    } else {
                        print!("{}", display_path,);
                    }
                    print!(
                        " {}{}\r",
                        " ".repeat(space_width - space_width / 2),
                        spinner_symbol
                    );

                    let target_file = target_base_dir.join(base_file);

                    (*copy_queue.lock()).push_front((file.into_path(), target_file));
                }

                for copy_handler in copy_handlers {
                    if let Err(e) = copy_handler.await {
                        println!(
                            "Some error occurred; cleaning up the templates directory first..."
                        );
                        std::fs::remove_dir_all(target_base_dir).ok();
                        panic!("{}", e);
                    };
                }
            }
        });

        let new_template = Template {
            name: template_name,
            description: template_description,
            directory: target_base_dir,
        };
        config.config.insert_template(new_template).unwrap();
    }
}

pub mod list {
    use crate::config::LoadedConfig;
    use colored::Colorize;

    pub const CMD_STR: &str = "list";

    pub fn list(config: &LoadedConfig) {
        for template in config.config.templates.values() {
            println!(
                "{}\n  {}",
                template.name.bold(),
                template
                    .description
                    .as_ref()
                    .unwrap_or(&"No description.".italic().to_string()),
            );
        }
    }
}

pub mod tree {
    pub const CMD_STR: &str = "tree";
}
