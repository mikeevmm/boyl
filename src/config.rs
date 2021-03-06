use crate::template::Template;
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    fmt::Display,
    fs,
    hash::{Hash, Hasher},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

/// Given the base configuration folder path, returns
/// the path of the configuration JSON file.
fn get_json_path(config_path: &Path) -> PathBuf {
    config_path.join("config.json")
}

/// Gets the default directory for boyl's configuration files,
/// namely `(default config directory)/boyl`, where the default
/// configuration directory is given by the `dirs` crate.
///
/// As a side effect of this function, **if the default directory
/// does not exist, it will be created**.
pub fn default_config_dir() -> PathBuf {
    let default_dir = dirs::config_dir()
        .expect("`dirs` crate does not specify a config directory for this OS.")
        .join("boyl");
    if !default_dir.exists() {
        std::fs::create_dir_all(default_dir.clone())
            .expect("Failed to create the default configuration directory.");
    }
    default_dir
}

pub fn write_config_or_fail(config: &LoadedConfig) {
    if let Err(err) = config.write_config() {
        println!("{}", err);
        std::process::exit(exitcode::IOERR);
    }
}

pub type TemplateKey = u64;

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
    pub version: String,
    pub templates: BTreeMap<TemplateKey, Template>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            templates: BTreeMap::new(),
            version: super::VERSION.to_string(),
        }
    }
}

impl Config {
    pub fn get_template_key(template_name: &str) -> u64 {
        let mut hasher = DefaultHasher::default();
        template_name.hash(&mut hasher);
        hasher.finish()
    }

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
            return Err(LoadConfigError::NotAFile(json_path.display().to_string()));
        }
        let json_file = match fs::File::open(json_path.clone()) {
            Ok(f) => f,
            Err(x) => return Err(LoadConfigError::FileError(x)),
        };
        let reader = BufReader::new(json_file);
        serde_json::from_reader::<_, Config>(reader)
            .map_err(|e| LoadConfigError::BadDeserialization(e, json_path.display().to_string()))
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

pub enum DeleteTemplateError<'key> {
    NoTemplate(&'key TemplateKey),
    IoErr(std::io::Error),
}

/// Struct coupling the serializable, in-memory representation of the
/// program's configuration `Config`, with information about its file
/// representation.
pub struct LoadedConfig {
    pub config: Config,
    pub path: PathBuf,
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

    /// Get the template base directory, per this `LoadedConfig`'s base directory.
    ///
    /// As a side effect of this call, if this directory does not exist, it will
    /// be created.
    pub fn get_template_dir(&self) -> PathBuf {
        let dir = self.path.join("templates");
        if !dir.exists() {
            std::fs::create_dir(&dir).expect("Could not create templates directory.");
        }
        dir
    }

    /// Serialize the configuration object to disk, according to the path
    /// information in `LoadedConfig`.
    ///
    /// If the JSON file does not exist, it will be created.
    pub fn write_config(&self) -> Result<(), WriteConfigError> {
        let json_path = get_json_path(&self.path);
        if json_path.exists() && !json_path.is_file() {
            return Err(WriteConfigError::NotAFile(json_path.display().to_string()));
        }
        let json_file = match fs::File::create(json_path.clone()) {
            Ok(f) => f,
            Err(e) => return Err(WriteConfigError::FileError(e)),
        };
        let writer = BufWriter::new(json_file);
        serde_json::to_writer(writer, &self.config)
            .map_err(|e| WriteConfigError::BadSerialization(e, json_path.display().to_string()))
    }

    /// Deltes a template from the `Config` in memory, removing the corresponding saved
    /// directory in the templates directory.
    pub fn delete_template<'key>(
        &mut self,
        key: &'key TemplateKey,
    ) -> Result<(), DeleteTemplateError<'key>> {
        if !self.config.templates.contains_key(key) {
            Err(DeleteTemplateError::NoTemplate(key))
        } else if let Err(err) =
            std::fs::remove_dir_all(self.config.templates.remove(key).unwrap().path)
        {
            Err(DeleteTemplateError::IoErr(err))
        } else {
            Ok(())
        }
    }
}
