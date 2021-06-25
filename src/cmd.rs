pub mod new {
    pub const CMD_STR: &str = "new";
    pub const TEMPLATE_ARG: &str = "TEMPLATE";
    pub const NAME_ARG: &str = "NAME";
    pub const LOCATION_ARG: &str = "LOCATION";
}

pub mod make {
    use std::{
        collections::HashMap,
        error::Error,
        fmt::Debug,
        fs,
        path::{Path, PathBuf},
        str::FromStr,
    };

    use crate::{
        config::{Config, LoadedConfig},
        template::Template,
        ui, Verbosity,
    };
    use colored::Colorize;
    use read_input::prelude::*;

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

        let mut files_memo = HashMap::<PathBuf, bool>::new();
        let files_to_include = walkdir::WalkDir::new(&template_dir.path_buf)
            .min_depth(1)
            .into_iter()
            .flatten()
            .filter(|f| file_list.is_included_memoized(f.path(), &mut files_memo));

        // We now copy the files to the templates directory, and store a new template in memory.
        let target_base_dir = config.get_template_dir().join(&template_name);
        std::fs::create_dir(&target_base_dir).expect("Could not create template directory.");

        for file in files_to_include {
            if verbosity >= Verbosity::Very {
                println!("Copying: {}", file.path().to_string_lossy());
            }
            let target_file =
                target_base_dir.join(file.path().strip_prefix(&template_dir.path_buf).unwrap());
            let copy_result = if file.path().is_dir() {
                std::fs::create_dir(target_file).err()
            } else {
                std::fs::copy(file.into_path(), target_file).err()
            };
            if let Some(e) = copy_result {
                if verbosity >= Verbosity::Very {
                    println!("Some error occurred; cleaning up the templates directory first...");
                }
                std::fs::remove_dir_all(target_base_dir).ok();
                panic!("{}", e);
            };
        }

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
