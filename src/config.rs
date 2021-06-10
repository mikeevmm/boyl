use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    templates: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config { templates: vec![] }
    }
}

pub enum LoadConfigError {
    NotAFile(String),
    FileError(std::io::Error),
    BadDeserialization(serde_json::Error, String),
}

impl Display for LoadConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadConfigError::NotAFile(path) => write!(
                f,
                "Configuration JSON path ({}) exists, but is not a file!",
                path
            ),
            LoadConfigError::FileError(e) => {
                write!(
                    f,
                    "Error opening the configuration JSON file for reading: {}",
                    e
                )
            }
            LoadConfigError::BadDeserialization(e, path) => {
                write!(
                    f,
                    "Error parsing the configuration JSON file: {}\n\
                    You can attempt to fix the file manually, or delete it \
                    (you will lose your configuration).\n\
                    The configuration file can be found in '{}'",
                    e, path
                )
            }
        }
    }
}

pub enum WriteConfigError {
    NotAFile(String),
    FileError(std::io::Error),
    BadSerialization(serde_json::Error, String),
}

impl Display for WriteConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteConfigError::NotAFile(path) => write!(
                f,
                "Configuration JSON path ('{}') exists, but is not a file!",
                path
            ),
            WriteConfigError::FileError(e) => write!(
                f,
                "Error opening the configuration JSON file for writing: '{}'",
                e
            ),
            WriteConfigError::BadSerialization(e, path) => {
                write!(
                    f,
                    "Error writing the memory configurations to file: '{}'\n\
                    This session's changes have not been saved, and it is \
                    possible that your configuration file has become corrupted. \
                    You can attempt to fix the file manually, or delete it \
                    (you will lose your configuration).\n\
                    The configuration file can be found in '{}'.",
                    e, path
                )
            }
        }
    }
}

fn get_json_path(config_path: &Path) -> PathBuf {
    config_path.join("config.json")
}

pub fn load_config(config_path: &Path) -> Result<Option<Config>, LoadConfigError> {
    let json_path = get_json_path(config_path);
    if !json_path.exists() {
        return Ok(None);
    }
    if !json_path.is_file() {
        return Err(LoadConfigError::NotAFile(
            json_path.to_string_lossy().to_string(),
        ));
    }
    let json_file = match fs::File::open(json_path.clone()) {
        Ok(f) => f,
        Err(x) => return Err(LoadConfigError::FileError(x)),
    };
    let reader = BufReader::new(json_file);
    serde_json::from_reader::<_, Config>(reader)
        .map_err(|e| {
            LoadConfigError::BadDeserialization(e, json_path.to_string_lossy().to_string())
        })
        .map(Some)
}

pub fn write_config(config: Config, config_path: &Path) -> Result<(), WriteConfigError> {
    let json_path = get_json_path(config_path);
    if json_path.exists() && !json_path.is_file() {
        return Err(WriteConfigError::NotAFile(
            json_path.to_string_lossy().to_string(),
        ));
    }
    let json_file = match fs::File::create(json_path.clone()) {
        Ok(f) => f,
        Err(e) => return Err(WriteConfigError::FileError(e)),
    };
    let writer = BufWriter::new(json_file);
    serde_json::to_writer(writer, &config)
        .map_err(|e| WriteConfigError::BadSerialization(e, json_path.to_string_lossy().to_string()))
}
