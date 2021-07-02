use colored::Colorize;
use shellexpand::LookupError;
use std::{env::VarError, fmt::Display, io, path::PathBuf, str::FromStr};

#[derive(Clone, PartialEq, Eq)]
pub struct UserDir {
    pub path_buf: PathBuf,
}

pub enum UserDirErr {
    ShellExpandError(LookupError<VarError>),
    CanonicalizeError(io::Error),
    NotDirectory,
}

impl Display for UserDirErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            UserDirErr::ShellExpandError(e) => e.fmt(f),
            UserDirErr::CanonicalizeError(e) => match e.kind() {
                std::io::ErrorKind::NotFound => write!(
                    f,
                    "Path does not exist.\n{}",
                    "Please note that the provided directory should \
                        be the parent directory to the new template instance."
                        .dimmed()
                ),
                std::io::ErrorKind::PermissionDenied => write!(f, "Permission denied for path."),
                _ => e.fmt(f),
            },
            UserDirErr::NotDirectory => write!(f, "Path is not a directory."),
        }
    }
}

impl From<LookupError<VarError>> for UserDirErr {
    fn from(err: LookupError<VarError>) -> Self {
        Self::ShellExpandError(err)
    }
}

impl From<io::Error> for UserDirErr {
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
    type Err = UserDirErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let expanded = shellexpand::full(s)?;
        // <PathBuf as FromStr>::Err is infallible
        let path_buf = PathBuf::from_str(&expanded).unwrap().canonicalize()?;
        if !path_buf.is_dir() {
            return Err(UserDirErr::NotDirectory);
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
    UserDir::from_str(path).map_err(|e| e.to_string())
}
