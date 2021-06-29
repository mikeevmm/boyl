#[macro_use]
extern crate serde;

use argh::FromArgs;
use colored::Colorize;
use config::LoadedConfig;
use std::{path::PathBuf, str::FromStr};
use userpath::{to_user_path, UserDir};

use crate::config::default_config_dir;

macro_rules! clone_move {
    (mut $x:ident) => {
        let mut $x = $x.clone();
    };
    ($x:ident) => {
        let $x = $x.clone();
    };
}

mod cmd;
mod config;
mod copy;
mod template;
#[allow(dead_code)]
mod ui;
mod userpath;
mod walkdir;

const VERSION: &str = "0.0.1";

#[derive(FromArgs)]
/// Quickly create boilerplate projects and templates.
struct Boyl {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Command {
    List(ListCommand),
    Tree(TreeCommand),
    Make(MakeCommand),
    New(NewCommand),
    Edit(EditCommand),
    Xoxo(XoxoCommand),
    Version(VersionCommand),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Lists the available templates.
#[argh(subcommand, name = "list")]
struct ListCommand {}

#[derive(FromArgs, PartialEq, Debug)]
/// Shows the tree structure of a template.
#[argh(subcommand, name = "tree")]
struct TreeCommand {
    #[argh(positional)]
    /// the project template to examine
    ///
    /// Should be one of the template names listed with `list`.
    template: String,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Interactively generates a new template.
#[argh(subcommand, name = "make")]
struct MakeCommand {}

#[derive(FromArgs, PartialEq, Debug)]
/// Creates a new project.
#[argh(subcommand, name = "new")]
struct NewCommand {
    #[argh(positional)]
    /// the project template to use
    ///
    /// Use the `list` command to find what templates are available,
    /// or create a new template with `make`.
    template: String,
    #[argh(option)]
    /// the name for the new project
    ///
    /// This will be the name of the created folder.
    /// By default takes the same name as the template.
    name: Option<String>,
    #[argh(option, from_str_fn(to_user_path))]
    /// where to create the new project
    ///
    /// Defaults to the current directory. This argument
    /// specifies the parent directory to the project, as a new
    /// folder will be created for the project.
    location: Option<userpath::UserDir>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Starts an interactive project management prompt.
#[argh(subcommand, name = "edit")]
struct EditCommand {}

#[derive(FromArgs, PartialEq, Debug)]
/// Print the current version.
#[argh(subcommand, name = "version")]
struct VersionCommand {}

#[derive(FromArgs, PartialEq, Debug)]
///
#[argh(subcommand, name = "xoxo")]
struct XoxoCommand {}

fn main() {
    let command: Boyl = argh::from_env();

    let config_path = std::env::var("BOYL_CONFIG").map_or_else(
        |_| default_config_dir(),
        |path| match to_user_path(&path) {
            Ok(path) => path.path_buf,
            Err(msg) => {
                println!("{}", msg);
                std::process::exit(exitcode::CONFIG);
            }
        },
    );

    let mut config = match config::LoadedConfig::load_from_path(config_path) {
        Ok(config) => config,
        Err(err) => {
            println!("{}", "Error loading configuration:".red());
            println!("{}", &err.to_string().red());
            std::process::exit(exitcode::USAGE);
        }
    };

    match command.command {
        Command::List(_) => cmd::list::list(&config),
        Command::Tree(tree) => cmd::tree::tree(&config, &tree.template),
        Command::Make(_) => {
            cmd::make::make(&mut config);
            config::write_config_or_fail(&config);
        }
        Command::New(new) => {
            cmd::new::new(&config, &new.template, new.name.as_deref(), new.location)
        }
        Command::Edit(_) => cmd::edit::edit(&config),
        Command::Xoxo(_) => cmd::xoxo::xoxo(),
        Command::Version(_) => cmd::version::version(),
    }

    std::process::exit(exitcode::OK)
}
