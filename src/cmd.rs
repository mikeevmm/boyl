pub mod new {
    pub const CMD_STR: &str = "new";
    pub const TEMPLATE_ARG: &str = "TEMPLATE";
    pub const NAME_ARG: &str = "NAME";
    pub const LOCATION_ARG: &str = "LOCATION";
}

pub mod make {
    use std::{error::Error, fmt::Debug, path::PathBuf, str::FromStr};

    use crate::ui;
    use colored::Colorize;
    use read_input::prelude::*;

    pub const CMD_STR: &str = "make";

    const ERR_PATH: &str = "Cannot understand path.";
    const ERR_NO_EXIST: &str = "Path does not exist.";
    const ERR_NOT_DIR: &str = "Path is not a directory.";

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

    pub fn make() {
        let current_dir = std::env::current_dir().ok();

        let template_dir = {
            let template_dir_default = current_dir.map(UserPath::from);
            let prompt = match &template_dir_default {
                Some(default) => format!(
                    "Template directory {}:",
                    format!("[default: {}]", default.path_buf.to_string_lossy()).dimmed()
                ),
                None => "Template directory:".to_string(),
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
                    "Template name {}:",
                    format!("[default: {}]", default).dimmed()
                ),
                None => "Template name:".to_string(),
            };
            match template_name_default {
                Some(default) => input().repeat_msg(prompt).default(default).get(),
                None => input::<String>().repeat_msg(prompt).get(),
            }
        };

        let ignore_list = {
            let mut ui_state = crate::ui::file::IgnoreUi::new(&template_dir.path_buf);
            ui::run_ui(&mut ui_state);
        };
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
