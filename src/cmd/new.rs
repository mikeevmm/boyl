use crate::{
    config::{Config, LoadedConfig},
    userpath::UserDir,
    walkdir,
};
use colored::Colorize;
use futures::StreamExt;

pub fn new(config: &LoadedConfig, template: &str, name: Option<&str>, location: Option<UserDir>) {
    let location = location
        .map(|d| d.path_buf)
        .unwrap_or_else(|| std::env::current_dir().expect("Could not read current directory."));
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

    let target_base_dir = location.join(name);
    if target_base_dir.exists() && target_base_dir.read_dir().unwrap().next().is_some() {
        println!("{}", "Cannot create new template:".red());
        println!(
            "{} already exists, and is not empty.",
            target_base_dir.to_string_lossy()
        );
        std::process::exit(exitcode::USAGE);
    }

    std::fs::create_dir(target_base_dir.clone()).expect("Could not create target base directory.");

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
        "{} {} {} {}.",
        "Created new template".green(),
        template.name,
        "in".green(),
        target_base_dir.to_string_lossy()
    );
}
