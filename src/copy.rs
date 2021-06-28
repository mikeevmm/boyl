use crate::ui::spinner::Spinner;
use futures::{Stream, StreamExt};
use std::path::Path;
use termion::terminal_size;
use tokio::fs::DirEntry;

async fn copy_from_to(from: &Path, to: &Path) -> Result<(), tokio::io::Error> {
    if from.is_dir() {
        if !to.exists() {
            tokio::fs::create_dir(to).await?;
        }
    } else {
        let parent = to.parent().unwrap();
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::copy(from, to).await?;
    }
    Ok(())
}

/// Copies files within `from_base_dir` (as given by the `files` iterator)
/// into a new `to_base_dir` directory.
pub async fn recursive_copy(
    from_base_dir: &'_ Path,
    to_base_dir: &'_ Path,
    mut files: impl Stream<Item = DirEntry> + Unpin,
) {
    let mut spinner = Spinner::new();
    let terminal_width = terminal_size().map(|(w, _)| w).unwrap_or(0);
    while let Some(file) = files.next().await {
        let file = file.path();
        if file == from_base_dir {
            continue;
        }
        let base_file = file.strip_prefix(from_base_dir).unwrap();

        let file_name = file.to_string_lossy();
        let file_name = &file_name[file_name
            .len()
            .saturating_sub(terminal_width.saturating_sub(8) as usize)..];
        let whitespace = " ".repeat((terminal_width as usize).saturating_sub(file_name.len() + 10));
        let spinner_symbol = spinner.tick();
        print!("{} {}{} {}\r", spinner_symbol, file_name, whitespace, spinner_symbol);

        let target_file = to_base_dir.join(base_file);

        if let Err(e) = copy_from_to(&file, &target_file).await {
            println!("Some error occurred; cleaning up the templates directory first...");
            std::fs::remove_dir_all(to_base_dir).ok();
            panic!("{}", e);
        }
    }
    println!("{}\r", " ".repeat(terminal_width as usize));
}
