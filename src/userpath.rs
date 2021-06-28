use std::{io, path::{self, PathBuf}};

/// Converts a user specified path (potentially using ~) to a
/// canonicalized PathBuf.
///
/// This function returns a `Result` as the `canonicalized()`
/// call can fail if, for example, the given path does not
/// exist.
pub fn user_path_to_path(path: &str) -> io::Result<PathBuf> {
    path::Path::new(&shellexpand::tilde(path).to_owned().to_string()).canonicalize()
}

/// Checks whether a user specified path (potentially using ~)
/// exists.
pub fn user_path_exists(path: &str) -> bool {
    user_path_to_path(path).map_or(false, |p| p.exists())
}
