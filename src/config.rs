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

/// Configuration elements that persist between sessions;
/// this struct is deserialized and serialized from/to a
/// JSON file on program start/end.
///
/// This object should be agnostic to the location of the
/// configuration file, and should instead represent an
/// "in-memory" view of the program's configuration. For
/// applications that are focused on the configuration as
/// a file, [`LoadedConfig`] should be used. Furthermore,
/// it is expected that a `Config` struct is never created
/// explicitly, and rather derived from a `LoadedConfig`.
#[derive(Serialize, Deserialize)]
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
    /// Deserialize a `Config` object from an in-disk `JSON` representation.
    ///
    /// # Returns
    ///
    /// If the specified serialized JSON file (as given by `path`) exists, this
    /// function returns `Some(Config)`, containing the deserialized `Config`
    /// struct. If the file does not exist, `None` is returned.
    fn load_from_path(path: &Path) -> Result<Option<Config>, LoadConfigError> {
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

/// Struct coupling the serializable, in-memory representation of the
/// program's configuration `Config`, with information about its file
/// representation.
pub struct LoadedConfig {
    pub config: Config,
    path: PathBuf,
}

impl LoadedConfig {
    /// Load a configuration from a JSON path. The given path is expected
    /// to exist up until to the penultimate component.
    ///
    /// If the specified file does not exist, a default configuration is
    /// instantiated instead.
    pub fn load_from_path(path: PathBuf) -> Result<Self, LoadConfigError> {
        let config = Config::load_from_path(&path)?.unwrap_or_default();
        Ok(LoadedConfig { config, path })
    }

    /// Serialize the configuration object to disk, according to the path
    /// information in `LoadedConfig`.
    ///
    /// If the JSON file does not exist, it will be created.
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
        serde_json::to_writer(writer, &self.config).map_err(|e| {
            WriteConfigError::BadSerialization(e, json_path.to_string_lossy().to_string())
        })
    }
}
