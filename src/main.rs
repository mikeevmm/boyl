use std::{fs, path};

use clap::{App, AppSettings, Arg};

use crate::verbosity::Verbosity;

mod verbosity;
mod config;

fn path_exists(path: &str) -> bool {
    path::Path::new(&shellexpand::tilde(path).to_owned().to_string())
        .canonicalize()
        .map_or(false, |p| p.exists())
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
            Arg::with_name("directory")
            .hidden(true)
            .env("BOYL_DIRECTORY")
            .default_value(config::default_config_dir())
        )
        .subcommand(App::new("list").about("Lists the available projects."))
        .subcommand(App::new("make").about("Generates a new template from the current folder."))
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
                            if path_exists(&arg_str) {
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

    match matches.subcommand() {
        ("new", Some(sub_matches)) => {}
        ("create", Some(sub_matches)) => {}
        ("list", Some(sub_matches)) => {}
        (name, _) => panic!("Unimplemented subcommand {}", name),
    }
}
