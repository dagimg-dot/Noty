use std::fs;
use std::io::{self, Write as _};
use std::path::Path;
use std::time::SystemTime;

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
}
