use crate::template::Template;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs,
    io::{self, BufReader, BufWriter},
    path::{Path, PathBuf},
};

/// Given the base configuration folder path, returns
/// the path of the configuration JSON file.
fn get_json_path(config_path: &Path) -> PathBuf {
    config_path.join("config.json")
}

/// Given the base configuration folder path, returns
/// the path of the templates folder.
fn get_template_dir(config_path: &Path) -> PathBuf {
    config_path.join("templates")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    version: String,
    templates: Vec<Template>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            templates: vec![],
            version: super::VERSION.to_string(),
        }
    }
}

impl Config {
    fn load_config(path: &Path) -> Result<Option<Config>, LoadConfigError> {
        let json_path = get_json_path(path);
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

pub enum TemplateScanError {
    NotAFolder(String),
    ReadDirError(io::Error, String),
    DirEntryError(io::Error, String),
}

impl Display for TemplateScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateScanError::NotAFolder(path) => {
                write!(
                    f,
                    "The templates path ('{}') exists, but is not a directory!",
                    path
                )
            }
            TemplateScanError::ReadDirError(e, path) => {
                write!(
                    f,
                    "Could not read the contentes of the templates directory ('{}'): \
                    '{}'",
                    path, e
                )
            }
            TemplateScanError::DirEntryError(e, path) => {
                write!(
                    f,
                    "Could not read content of the templates directory ('{}'): '{}'",
                    path, e
                )
            }
        }
    }
}

pub struct LoadedConfig {
    pub config: Config,
    path: PathBuf,
}

impl LoadedConfig {
    pub fn load_config(path: PathBuf) -> Result<Self, LoadConfigError> {
        let config = Config::load_config(&path)?.unwrap_or_default();
        Ok(LoadedConfig { config, path })
    }

    pub fn write_config(&self) -> Result<(), WriteConfigError> {
        let json_path = get_json_path(&self.path);
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
        serde_json::to_writer(writer, &self.path).map_err(|e| {
            WriteConfigError::BadSerialization(e, json_path.to_string_lossy().to_string())
        })
    }

    pub fn scan_templates(&mut self) -> Result<(), TemplateScanError> {
        let templates_dir = get_template_dir(&self.path);
        if templates_dir.exists() && !templates_dir.is_dir() {
            return Err(TemplateScanError::NotAFolder(
                templates_dir.to_string_lossy().to_string(),
            ));
        }
        if !templates_dir.exists() {
            return Ok(());
        }
        let items = match fs::read_dir(templates_dir.clone()) {
            Ok(items) => items,
            Err(err) => {
                return Err(TemplateScanError::ReadDirError(
                    err,
                    templates_dir.to_string_lossy().to_string(),
                ))
            }
        };
        for item in items {
            match item {
                Ok(item) => {
                    let item = item.path();
                    if item.is_file() {
                        continue;
                    }
                    let item_name = item.file_name().unwrap().to_string_lossy();

                    // TODO!
                }
                Err(err) => {
                    return Err(TemplateScanError::DirEntryError(
                        err,
                        templates_dir.to_string_lossy().to_string(),
                    ))
                }
            }
        }
        Ok(())
    }
}
