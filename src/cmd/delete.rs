use colored::Colorize;
use read_input::prelude::*;

use crate::{
    config::{Config, LoadedConfig},
    userbool::UserBool,
};

pub fn delete(config: &mut LoadedConfig, to_delete_name: &str) {
    let template_key = Config::get_template_key(to_delete_name);
    if !config.config.templates.contains_key(&template_key) {
        println!(
            "{}{}",
            to_delete_name.yellow(),
            " is not an existing template.".red()
        );
        println!(
            "{}{}{}",
            "Call ".dimmed(),
            "boyl list".yellow(),
            " to list existing templates.".dimmed()
        );
        std::process::exit(exitcode::USAGE);
    }
    let confirm = input()
        .msg(&format!(
            "Are you sure you want to delete '{}'? {} ",
            to_delete_name,
            "[y/N]".dimmed()
        ))
        .default(UserBool { value: false })
        .get()
        .into();
    if confirm {
        // Delete both the entry in the config and the folder
        std::fs::remove_dir_all(&config.config.templates.get(&template_key).unwrap().path).unwrap();
        config.config.templates.remove(&template_key);

        println!("Template {} deleted.", to_delete_name);
    } else {
        println!("Aborted.")
    }
}
