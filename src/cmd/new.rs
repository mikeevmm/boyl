use crate::{
    config::{Config, LoadedConfig},
    userpath::user_path_to_path,
    walkdir,
};
use colored::Colorize;
use futures::StreamExt;
use std::io::ErrorKind;

pub const CMD_STR: &str = "new";
pub const TEMPLATE_ARG: &str = "TEMPLATE";
pub const NAME_ARG: &str = "NAME";
pub const LOCATION_ARG: &str = "LOCATION";

enum LocationErrorKind {
    NotExists,
    NotADir,
    Unknown(Box<dyn std::error::Error>),
}

fn location_error(kind: LocationErrorKind, location: &str) {
    println!("{}", "Cannot create new template:".red());
    match kind {
        LocationErrorKind::NotExists => {
            println!("{} does not exist.", location);
            println!(
                "{}",
                "Please note that the provided directory should \
            be the parent directory to the new template instance."
                    .dimmed()
            );
        }
        LocationErrorKind::NotADir => {
            println!("{} is not a directory.", location);
        }
        LocationErrorKind::Unknown(err) => {
            println!("{}", err);
        }
    }
}

pub fn new(config: &LoadedConfig, template: &str, name: Option<&str>, location_str: &str) {
    let template_key = Config::get_template_key(template);
    let template = match config.config.templates.get(&template_key) {
        Some(template) => template,
        None => {
            println!("{}", format!("{} does not exist.", template).red());
            println!(
                "To list existing templates, call {} or create a new one with {}.",
                "boyl list".yellow(),
                "boyl make".yellow(),
            );
            std::process::exit(exitcode::USAGE);
        }
    };
    let name = name.unwrap_or(&template.name);
    let location = match user_path_to_path(location_str) {
        Ok(location) => location,
        Err(err) => {
            match err.kind() {
                ErrorKind::NotFound => location_error(LocationErrorKind::NotExists, location_str),
                _ => location_error(LocationErrorKind::Unknown(Box::new(err)), location_str),
            };
            std::process::exit(exitcode::USAGE);
        }
    };
    if !location.is_dir() {
        location_error(LocationErrorKind::NotADir, location_str);
        std::process::exit(exitcode::USAGE);
    }

    let target_base_dir = location.join(name);
    if target_base_dir.exists() && target_base_dir.read_dir().unwrap().next().is_some() {
        println!("{}", "Cannot create new template:".red());
        println!(
            "{} already exists, and is not empty.",
            target_base_dir.to_string_lossy()
        );
        std::process::exit(exitcode::USAGE);
    }

    let tokio_runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();
    tokio_runtime.block_on({
        async {
            let files_to_include =
                Box::pin(walkdir::visit(&template.path).filter_map(|x| async move {
                    match x {
                        Ok(x) => Some(x),
                        Err(_) => None,
                    }
                }));
            crate::copy::recursive_copy(&template.path, &target_base_dir, files_to_include).await;
        }
    });

    println!(
        "{} {} {} {}",
        "Created new template".green(),
        template.name,
        "in".green(),
        target_base_dir.to_string_lossy()
    );
}
