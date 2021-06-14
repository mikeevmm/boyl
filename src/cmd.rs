pub mod new {
    pub const CMD_STR: &str = "new";
    pub const TEMPLATE_ARG: &str = "TEMPLATE";
    pub const NAME_ARG: &str = "NAME";
    pub const LOCATION_ARG: &str = "LOCATION";
}

pub mod make {
    use std::fmt::format;

    use colored::Colorize;

    pub const CMD_STR: &str = "make";

    pub fn make() {
        println!("This prompt will guide you through creating a new template in Boyl.");

        // Inspect the current directory; we will assume that the new template
        // is going to be based on the current working directory, and suggest
        // defaults to the prompted values accordingly.
        let working_dir =
            std::env::current_dir().expect("Failed to read the current working directory.");

        // * The default name is the working directory's top level name
        let name = {
            let default_name = working_dir
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let default_str = format!("[default: '{}']", &default_name).dimmed();
            let name_prompt = format!("Template name {}: ", &default_str);
            let from_prompt = rprompt::prompt_reply_stdout(&name_prompt)
                .unwrap()
                .trim()
                .to_string();
            if from_prompt.is_empty() {
                from_prompt
            } else {
                default_name
            }
        };

        // * The default project to copy is the current working directory
        let copy_dir_str = {
            let default_copy_dir = working_dir.display().to_string();
            let default_str = format!("[default: '{}']", &default_copy_dir).dimmed();
            let copy_dir_prompt = format!("Copy from {}:", &default_str);
            let from_prompt = rprompt::prompt_reply_stdout(&copy_dir_prompt)
                .unwrap()
                .trim()
                .to_string();
            if from_prompt.is_empty() {
                from_prompt
            } else {
                default_copy_dir
            }
        };

        // * Because it implies dropping to an external editor, we prompt
        //   the user whether they want to edit an .ignores file.
        let wants_ignores = {
            let prompt_str = format!("Edit an .ignores file {}? ", "[default: No]".dimmed());
            let from_prompt = rprompt::prompt_reply_stdout(&prompt_str).unwrap();
            crate::boolprompt::user_str_into_bool(&from_prompt, false)
        };
        //TODO
    }
}

pub mod list {
    use crate::config::LoadedConfig;
    use colored::Colorize;

    pub const CMD_STR: &str = "list";

    pub fn list(config: &LoadedConfig) {
        for template in config.config.templates.values() {
            println!(
                "\
    {}
      {}",
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
