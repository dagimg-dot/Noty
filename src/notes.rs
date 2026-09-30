use std::cmp::Reverse;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Result, bail};

use crate::config::{Config, SortMethod};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub name: String,
    pub path: PathBuf,
    pub modified: SystemTime,
}

#[derive(Debug, Clone)]
pub struct NoteStore {
    dir: PathBuf,
    recurse: bool,
    use_extension: bool,
    sort: SortMethod,
    notes: Vec<Note>,
}

impl NoteStore {
    pub fn new(dir: PathBuf, recurse: bool, use_extension: bool, sort: SortMethod) -> Self {
        Self {
            dir,
            recurse,
            use_extension,
            sort,
            notes: Vec::new(),
        }
    }

    pub fn from_config(config: &Config) -> Self {
        Self::new(
            config.notes_dir(),
            config.recurse_subfolders(),
            config.use_file_extension(),
            config.sort_method(),
        )
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    /// Re-reads the notes folder, creating it on first use. Like the Python
    /// app, notes are visible files with no extension or a `.md` one.
    pub fn scan(&mut self) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let mut notes = Vec::new();
        collect_notes(&self.dir, self.recurse, &mut notes)?;
        match self.sort {
            SortMethod::Name => notes.sort_by_key(|note| note.name.to_lowercase()),
            SortMethod::Modified => notes.sort_by_key(|note| Reverse(note.modified)),
        }
        self.notes = notes;
        Ok(())
    }

    pub fn create(&mut self, name: &str) -> Result<PathBuf> {
        let name = valid_name(name)?;
        let path = self.dir.join(file_name(name, self.use_extension));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                bail!("A note named “{name}” already exists")
            }
            Err(err) => return Err(err.into()),
        }
        self.scan()?;
        Ok(path)
    }

    /// Renames the note at `path`, keeping it in the same folder and keeping
    /// its `.md` extension if it has one.
    pub fn rename(&mut self, path: &Path, new_name: &str) -> Result<PathBuf> {
        let name = valid_name(new_name)?;
        let new_path = path.with_file_name(file_name(name, path.extension().is_some()));
        if new_path.exists() && !same_file::is_same_file(path, &new_path)? {
            bail!("A note named “{name}” already exists");
        }
        fs::rename(path, &new_path)?;
        self.scan()?;
        Ok(new_path)
    }

    pub fn delete(&mut self, path: &Path) -> Result<()> {
        match fs::remove_file(path) {
            Err(err) if err.kind() != io::ErrorKind::NotFound => return Err(err.into()),
            _ => {}
        }
        self.scan()?;
        Ok(())
    }
}

fn file_name(name: &str, markdown: bool) -> String {
    if markdown || name.contains('.') {
        format!("{name}.md")
    } else {
        name.to_owned()
    }
}

fn valid_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\', '\0']) {
        bail!("“{name}” can't be used as a note name");
    }
    Ok(name)
}

/// The name Noty shows for a note file: the file name without its extension.
pub fn note_name(path: &Path) -> String {
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    match file_name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_owned(),
        _ => file_name.into_owned(),
    }
}

fn is_note_file(file_name: &str) -> bool {
    !file_name.starts_with('.')
        && (!file_name.contains('.') || file_name.to_lowercase().ends_with(".md"))
}

fn collect_notes(dir: &Path, recurse: bool, notes: &mut Vec<Note>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if file_name.starts_with('.') {
            continue;
        }
        if entry.file_type()?.is_dir() {
            if recurse {
                collect_notes(&path, recurse, notes)?;
            }
            continue;
        }
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.is_file() && is_note_file(&file_name) {
            notes.push(Note {
                name: note_name(&path),
                modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                path,
            });
        }
    }
    Ok(())
}

pub fn read_note(path: &Path) -> io::Result<(String, SystemTime)> {
    let content = fs::read_to_string(path)?;
    Ok((content, modified_time(path)?))
}

pub fn modified_time(path: &Path) -> io::Result<SystemTime> {
    fs::metadata(path)?.modified()
}

#[derive(Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved(SystemTime),
    Conflict,
}

/// Writes `content` to the note at `path`.
///
/// With `expected_modified`, the write is refused with
/// [`SaveOutcome::Conflict`] when the file changed or disappeared since it
/// was loaded, so edits made in other apps are never overwritten silently.
pub fn save_note(
    path: &Path,
    content: &str,
    expected_modified: Option<SystemTime>,
) -> io::Result<SaveOutcome> {
    let link_metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => Some(metadata),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => return Err(err),
    };
    if let Some(expected) = expected_modified {
        let unchanged = link_metadata.is_some() && modified_time(path)? == expected;
        if !unchanged {
            return Ok(SaveOutcome::Conflict);
        }
    }
    match &link_metadata {
        Some(metadata) if metadata.file_type().is_symlink() => fs::write(path, content)?,
        _ => replace_atomically(path, content, link_metadata.as_ref())?,
    }
    Ok(SaveOutcome::Saved(modified_time(path)?))
}

fn replace_atomically(
    path: &Path,
    content: &str,
    original: Option<&fs::Metadata>,
) -> io::Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let temp = dir.join(format!(".{file_name}.noty-{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = fs::File::create(&temp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        if let Some(original) = original {
            fs::set_permissions(&temp, original.permissions())?;
        }
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn store(dir: &Path) -> NoteStore {
        NoteStore::new(dir.to_path_buf(), false, false, SortMethod::Name)
    }

    fn names(store: &NoteStore) -> Vec<&str> {
        store
            .notes()
            .iter()
            .map(|note| note.name.as_str())
            .collect()
    }

    #[test]
    fn scan_lists_visible_plain_and_markdown_files() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["todo", "Ideas.md", "photo.png", ".hidden", "b-list"] {
            fs::write(dir.path().join(name), "x").unwrap();
        }
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/nested"), "x").unwrap();

        let mut notes = store(dir.path());
        notes.scan().unwrap();
        assert_eq!(names(&notes), ["b-list", "Ideas", "todo"]);

        let mut recursive = NoteStore::new(dir.path().into(), true, false, SortMethod::Name);
        recursive.scan().unwrap();
        assert_eq!(names(&recursive), ["b-list", "Ideas", "nested", "todo"]);
    }

    #[test]
    fn scan_can_sort_by_last_change() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["older", "newer"] {
            fs::write(dir.path().join(name), name).unwrap();
            std::thread::sleep(Duration::from_millis(20));
        }
        let mut notes = NoteStore::new(dir.path().into(), false, false, SortMethod::Modified);
        notes.scan().unwrap();
        assert_eq!(names(&notes), ["newer", "older"]);
    }

    #[test]
    fn scan_creates_a_missing_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut notes = store(&dir.path().join("Noties"));
        notes.scan().unwrap();
        assert!(dir.path().join("Noties").is_dir());
        assert!(notes.notes().is_empty());
    }

    #[test]
    fn create_rejects_unsafe_names_and_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let mut notes = store(dir.path());
        assert!(notes.create("../escape").is_err());
        assert!(notes.create(".hidden").is_err());
        assert!(notes.create("   ").is_err());

        let path = notes.create(" Groceries ").unwrap();
        assert_eq!(path, dir.path().join("Groceries"));
        assert_eq!(names(&notes), ["Groceries"]);
        let err = notes.create("Groceries").unwrap_err();
        assert_eq!(err.to_string(), "A note named “Groceries” already exists");
    }

    #[test]
    fn create_adds_the_markdown_extension_when_configured() {
        let dir = tempfile::tempdir().unwrap();
        let mut notes = NoteStore::new(dir.path().into(), false, true, SortMethod::Name);
        assert_eq!(notes.create("Plan").unwrap(), dir.path().join("Plan.md"));
        assert_eq!(names(&notes), ["Plan"]);
    }

    #[test]
    fn names_with_dots_stay_visible() {
        let dir = tempfile::tempdir().unwrap();
        let mut notes = store(dir.path());
        assert_eq!(notes.create("v1.2").unwrap(), dir.path().join("v1.2.md"));
        assert_eq!(names(&notes), ["v1.2"]);
    }

    #[test]
    fn rename_keeps_the_folder_and_extension() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Plan.md"), "x").unwrap();
        fs::write(dir.path().join("todo"), "y").unwrap();
        let mut notes = store(dir.path());

        let renamed = notes
            .rename(&dir.path().join("Plan.md"), "Roadmap")
            .unwrap();
        assert_eq!(renamed, dir.path().join("Roadmap.md"));
        let renamed = notes.rename(&dir.path().join("todo"), "Errands").unwrap();
        assert_eq!(renamed, dir.path().join("Errands"));
        assert_eq!(names(&notes), ["Errands", "Roadmap"]);
        assert_eq!(fs::read_to_string(dir.path().join("Errands")).unwrap(), "y");
    }

    #[test]
    fn rename_refuses_to_replace_another_note() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a"), "a").unwrap();
        fs::write(dir.path().join("b"), "b").unwrap();
        let mut notes = store(dir.path());

        let err = notes.rename(&dir.path().join("a"), "b").unwrap_err();
        assert_eq!(err.to_string(), "A note named “b” already exists");
        assert_eq!(fs::read_to_string(dir.path().join("b")).unwrap(), "b");
        assert!(notes.rename(&dir.path().join("a"), "a/b").is_err());
    }

    #[test]
    fn rename_can_change_only_the_case() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("todo"), "x").unwrap();
        let mut notes = store(dir.path());
        notes.rename(&dir.path().join("todo"), "Todo").unwrap();
        assert_eq!(names(&notes), ["Todo"]);
    }

    #[test]
    fn delete_removes_the_file_and_tolerates_missing_ones() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("gone"), "x").unwrap();
        fs::write(dir.path().join("kept"), "y").unwrap();
        let mut notes = store(dir.path());

        notes.delete(&dir.path().join("gone")).unwrap();
        assert!(!dir.path().join("gone").exists());
        assert_eq!(names(&notes), ["kept"]);
        notes.delete(&dir.path().join("gone")).unwrap();
    }

    #[test]
    fn save_refuses_to_overwrite_changes_made_elsewhere() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note");
        fs::write(&path, "original").unwrap();
        let (_, loaded) = read_note(&path).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        fs::write(&path, "theirs").unwrap();

        assert_eq!(
            save_note(&path, "mine", Some(loaded)).unwrap(),
            SaveOutcome::Conflict
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "theirs");

        assert!(matches!(
            save_note(&path, "mine", None).unwrap(),
            SaveOutcome::Saved(_)
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), "mine");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn save_treats_a_deleted_note_as_a_conflict_until_forced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note");
        fs::write(&path, "original").unwrap();
        let (_, loaded) = read_note(&path).unwrap();
        fs::remove_file(&path).unwrap();

        assert_eq!(
            save_note(&path, "mine", Some(loaded)).unwrap(),
            SaveOutcome::Conflict
        );
        assert!(!path.exists());
        assert!(matches!(
            save_note(&path, "mine", None).unwrap(),
            SaveOutcome::Saved(_)
        ));
        assert_eq!(fs::read_to_string(&path).unwrap(), "mine");
    }

    #[cfg(unix)]
    #[test]
    fn save_writes_through_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("real");
        let link = dir.path().join("link");
        fs::write(&target, "a").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        save_note(&link, "b", None).unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&target).unwrap(), "b");
    }

    #[test]
    fn note_names_drop_the_extension() {
        assert_eq!(note_name(Path::new("/n/Travel plans.md")), "Travel plans");
        assert_eq!(note_name(Path::new("/n/todo")), "todo");
    }
}
