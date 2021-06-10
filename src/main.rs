#[macro_use]
extern crate serde;
extern crate clap;
extern crate dirs;
extern crate num_traits;
extern crate serde_json;
extern crate shellexpand;

use clap::{App, AppSettings, Arg};
use num_traits::PrimInt;
use std::{
    io,
    path::{self, PathBuf},
};

mod config;

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

fn main() {
    let matches = App::new("boyl")
        .version("1.0")
        .author("Miguel Murça <zvthryzhepn+rot13@gmail.com>")
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
        .subcommand(App::new("list").about("Lists the available projects."))
        .subcommand(
            App::new("make")
                .about("Interactively generates a new template from the current folder."),
        )
        .subcommand(
            App::new("new")
                .about("Creates a new boilerplate project.")
                .arg(
                    Arg::with_name("TEMPLATE")
                        .help("The project template to use")
                        .long_help(
                            "The project template to use. Use the `list` command \
                            to find what templates are available, or create a new \
                            template with `create`.",
                        )
                        .required(true),
                )
                .arg(
                    Arg::with_name("NAME")
                        .help("The name for the new project")
                        .long_help(
                            "The name for the new project. \
                            This will be the name of the created folder.",
                        )
                        .required(true),
                )
                .arg(
                    Arg::with_name("LOCATION")
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
    let config_dir = matches
        .value_of("config_dir")
        .map_or_else(default_config_dir, |user_path| {
            user_path_to_path(user_path).unwrap()
        });

    let config = match config::load_config(&config_dir) {
        Ok(config) => config.unwrap_or_else(config::Config::default),
        Err(err) => {
            clap::Error::with_description(&err.to_string(), clap::ErrorKind::InvalidValue).exit()
        }
    };

    println!("{:?}", config);

    match matches.subcommand() {
        ("new", Some(sub_matches)) => {}
        ("create", Some(sub_matches)) => {}
        ("list", Some(sub_matches)) => {}
        (name, _) => panic!("Unimplemented subcommand {}", name),
    }

    if let Err(err) = config::write_config(config, &config_dir) {
        clap::Error::with_description(&err.to_string(), clap::ErrorKind::InvalidValue).exit()
    }
}
