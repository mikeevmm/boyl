#[macro_use]
extern crate serde;
extern crate clap;
extern crate dirs;
extern crate num_traits;
extern crate read_input;
extern crate serde_json;
extern crate shellexpand;
extern crate tokio;

use clap::{App, AppSettings, Arg};
use config::LoadedConfig;
use num_traits::PrimInt;
use std::{
    io,
    path::{self, PathBuf},
};

mod cmd;
mod config;
mod template;
#[allow(dead_code)]
mod ui;

const VERSION: &str = "0.0.1";

#[derive(Debug)]
enum Verbosity {
    None,
    Some,
    Very,
}

impl<X> From<X> for Verbosity
where
    X: PrimInt,
{
    fn from(value: X) -> Self {
        if value.lt(&X::from(1).unwrap()) {
            Verbosity::None
        } else if value.lt(&X::from(2).unwrap()) {
            Verbosity::Some
        } else {
            Verbosity::Very
        }
    }
}

/// Converts a user specified path (potentially using ~) to a
/// canonicalized PathBuf.
///
/// This function returns a `Result` as the `canonicalized()`
/// call can fail if, for example, the given path does not
/// exist.
fn user_path_to_path(path: &str) -> io::Result<PathBuf> {
    path::Path::new(&shellexpand::tilde(path).to_owned().to_string()).canonicalize()
}

/// Checks whether a user specified path (potentially using ~)
/// exists.
fn user_path_exists(path: &str) -> bool {
    user_path_to_path(path).map_or(false, |p| p.exists())
}

/// Gets the default directory for boyl's configuration files,
/// namely `(default config directory)/boyl`, where the default
/// configuration directory is given by the `dirs` crate.
///
/// As a side effect of this function, **if the default directory
/// does not exist, it will be created**.
fn default_config_dir() -> PathBuf {
    let default_dir = dirs::config_dir()
        .expect("`dirs` crate does not specify a config directory for this OS.")
        .join("boyl");
    if !default_dir.exists() {
        std::fs::create_dir_all(default_dir.clone())
            .expect("Failed to create the default configuration directory.");
    }
    default_dir
}

fn write_config_or_fail(config: &LoadedConfig) {
    if let Err(err) = config.write_config() {
        clap::Error::with_description(&err.to_string(), clap::ErrorKind::InvalidValue).exit()
    }
}

fn main() {
    let matches = App::new("boyl")
        .version(VERSION)
        .author("Miguel Murça <zvthryzhepn+obly+rot13@gmail.com>")
        .about("Quickly create boilerplate projects and templates.")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("v")
                .short("-v")
                .multiple(true)
                .help("Sets the level of verbosity")
                .global(true),
        )
        .arg(
            Arg::with_name("config_dir")
                .hidden(true)
                .env("BOYL_CONFIG")
                .validator(|dir_str| {
                    if user_path_exists(&dir_str) {
                        Ok(())
                    } else {
                        Err(format!(
                            "The specified configuration directory (\"{}\") does not exist.",
                            &dir_str
                        ))
                    }
                }),
        )
        .subcommand(App::new(cmd::list::CMD_STR).about("Lists the available templates."))
        .subcommand(
            App::new(cmd::tree::CMD_STR)
                .about("Shows the tree structure of a template.")
                .arg(
                    Arg::with_name(cmd::new::TEMPLATE_ARG)
                        .help("The project template to examine")
                        .long_help(
                            "The project template to examine. Should be one \
                            of the template names listed with `list`.",
                        )
                        .required(true),
                ),
        )
        .subcommand(
            App::new(cmd::make::CMD_STR)
                .about("Interactively generates a new template from the current folder."),
        )
        .subcommand(
            App::new(cmd::new::CMD_STR)
                .about("Creates a new project.")
                .arg(
                    Arg::with_name(cmd::new::TEMPLATE_ARG)
                        .help("The project template to use")
                        .long_help(
                            "The project template to use. Use the `list` command \
                            to find what templates are available, or create a new \
                            template with `create`.",
                        )
                        .required(true),
                )
                .arg(
                    Arg::with_name(cmd::new::NAME_ARG)
                        .help("The name for the new project")
                        .long_help(
                            "The name for the new project. \
                            This will be the name of the created folder.",
                        )
                        .required(true),
                )
                .arg(
                    Arg::with_name(cmd::new::LOCATION_ARG)
                        .default_value(".")
                        .help("Where to create the new project")
                        .long_help(
                            "Where to create the specified boilerplate \
                            project. Defaults to the current directory. This argument \
                            specifies the *parent* directory to the project, as a new \
                            folder will be created for the project.",
                        )
                        .validator(|arg_str| {
                            if user_path_exists(&arg_str) {
                                Ok(())
                            } else {
                                Err(format!("{} does not exist.", arg_str))
                            }
                        }),
                ),
        )
        .subcommand(App::new("edit").about("Starts an interactive project management prompt."))
        .get_matches();

    let verbosity = Verbosity::from(matches.occurrences_of("v"));
    let config_path = matches
        .value_of("config_dir")
        .map_or_else(default_config_dir, |user_path| {
            user_path_to_path(user_path).unwrap()
        });

    let mut config = match config::LoadedConfig::load_from_path(config_path) {
        Ok(config) => config,
        Err(err) => {
            clap::Error::with_description(&err.to_string(), clap::ErrorKind::InvalidValue).exit()
        }
    };

    match matches.subcommand() {
        (cmd::new::CMD_STR, Some(sub_matches)) => {
            todo!()
        }
        (cmd::make::CMD_STR, Some(_)) => {
            cmd::make::make();
            write_config_or_fail(&config);
        }
        (cmd::list::CMD_STR, Some(_)) => {
            cmd::list::list(&config);
        }
        (cmd::tree::CMD_STR, Some(sub_matches)) => {
            todo!()
        }
        (name, _) => panic!("Unimplemented subcommand {}", name),
    }
}
