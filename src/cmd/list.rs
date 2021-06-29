use crate::config::LoadedConfig;
use colored::Colorize;

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
