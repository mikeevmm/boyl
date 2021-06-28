use crate::{config::{Config, LoadedConfig}, ui::{self, file::FileTreeUi}};
use colored::Colorize;

pub const CMD_STR: &str = "tree";
pub const TEMPLATE_ARG: &str = "TEMPLATE";

pub fn tree(config: &LoadedConfig, template_name: &str) {
    let template_key = Config::get_template_key(template_name);
    let template = match config.config.templates.get(&template_key) {
        Some(x) => x,
        None => {
            println!(
                "{}",
                format!("{} is not an existing template.", template_name).red()
            );
            println!(
                "{} {}{}",
                "You can list existing templates with".dimmed(),
                "boyl list".yellow(),
                ".".dimmed()
            );
            std::process::exit(exitcode::USAGE);
        }
    };

    let mut ui_state = FileTreeUi::new(&template.path);
    ui::run_ui(&mut ui_state);
}
