use colored::Colorize;
use shellexpand::LookupError;
use std::{env::VarError, io, path::PathBuf, str::FromStr};

#[derive(Clone, PartialEq, Eq)]
pub struct UserDir {
    pub path_buf: PathBuf,
}

pub enum UserPathErr {
    ShellExpandError(LookupError<VarError>),
    CanonicalizeError(io::Error),
    NotDirectory,
}

impl From<LookupError<VarError>> for UserPathErr {
    fn from(err: LookupError<VarError>) -> Self {
        Self::ShellExpandError(err)
    }
}

impl From<io::Error> for UserPathErr {
    fn from(err: io::Error) -> Self {
        Self::CanonicalizeError(err)
    }
}

impl std::fmt::Debug for UserDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.path_buf.fmt(f)
    }
}

impl FromStr for UserDir {
    type Err = UserPathErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let expanded = shellexpand::full(s)?;
        // <PathBuf as FromStr>::Err is infallible
        let path_buf = PathBuf::from_str(&expanded).unwrap().canonicalize()?;
        if !path_buf.is_dir() {
            return Err(UserPathErr::NotDirectory);
        }
        Ok(UserDir { path_buf })
    }
}

impl From<PathBuf> for UserDir {
    fn from(path_buf: PathBuf) -> Self {
        UserDir { path_buf }
    }
}

impl std::fmt::Display for UserDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path_buf.to_string_lossy())
    }
}

/// Tries to convert a given user path (as a string slice) to a `UserDir`.
/// If it fails, returns an error message.
pub fn to_user_path(path: &str) -> Result<UserDir, String> {
    UserDir::from_str(path).map_err(|e| match e {
        UserPathErr::ShellExpandError(e) => {
            format!(
                "{}\n{}",
                "Error resolving the given path:".red(),
                e.to_string().red()
            )
        }
        UserPathErr::CanonicalizeError(e) => match e.kind() {
            std::io::ErrorKind::NotFound => format!(
                "{} does not exist.\n{}",
                path,
                "Please note that the provided directory should \
                    be the parent directory to the new template instance."
                    .dimmed()
            ),
            std::io::ErrorKind::PermissionDenied => format!("Permission denied for {}", path),
            _ => format!("{}", e),
        },
        UserPathErr::NotDirectory => {
            format!("{} is not a directory.", path)
        }
    })
}
