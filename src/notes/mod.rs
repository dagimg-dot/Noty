use std::path::{Path, PathBuf};
use std::time::SystemTime;

mod file;
pub mod saver;
mod store;

pub use file::{SaveOutcome, modified_time, read_note, save_note};
pub use store::NoteStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub name: String,
    pub path: PathBuf,
    pub modified: SystemTime,
}

/// The name Noty shows for a note file: the file name without its extension.
pub fn note_name(path: &Path) -> String {
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    match file_name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_owned(),
        _ => file_name.into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_names_drop_the_extension() {
        assert_eq!(note_name(Path::new("/n/Travel plans.md")), "Travel plans");
        assert_eq!(note_name(Path::new("/n/todo")), "todo");
    }
}
