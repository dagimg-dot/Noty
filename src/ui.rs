use std::path::Path;

use gpui_kit::*;

use crate::config::home_dir;

#[derive(Debug, Clone, Copy)]
pub enum AppIcon {
    Close,
    Notes,
    Saved,
    Unsaved,
    Warning,
}

impl AppIcon {
    fn path(self) -> &'static str {
        match self {
            AppIcon::Close => "icons/close.svg",
            AppIcon::Notes => "icons/book-open.svg",
            AppIcon::Saved => "icons/check.svg",
            AppIcon::Unsaved => "icons/asterisk.svg",
            AppIcon::Warning => "icons/triangle-alert.svg",
        }
    }
}

pub fn icon(icon: AppIcon, size: Pixels, color: Hsla) -> Svg {
    svg()
        .path(SharedString::new_static(icon.path()))
        .size(size)
        .flex_none()
        .text_color(color)
}

pub fn display_path(path: &Path) -> String {
    let home = home_dir();
    match path.strip_prefix(&home) {
        Ok(rest) if !home.as_os_str().is_empty() => Path::new("~").join(rest).display().to_string(),
        _ => path.display().to_string(),
    }
}

pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use super::{display_path, word_count};
    use crate::config::home_dir;

    #[test]
    fn paths_under_home_are_shortened() {
        assert_eq!(
            display_path(&home_dir().join("Documents/Noties")),
            "~/Documents/Noties"
        );
        assert_eq!(
            display_path(std::path::Path::new("/srv/notes")),
            "/srv/notes"
        );
    }

    #[test]
    fn words_are_whitespace_separated() {
        assert_eq!(word_count("# Title\n\nsome *notes* here"), 5);
        assert_eq!(word_count("   "), 0);
    }
}
