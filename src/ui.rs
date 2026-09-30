use std::path::Path;

use gpui_kit::*;

use crate::config::home_dir;

#[derive(Debug, Clone, Copy)]
pub enum AppIcon {
    Close,
    Notes,
}

impl AppIcon {
    fn path(self) -> &'static str {
        match self {
            AppIcon::Close => "icons/close.svg",
            AppIcon::Notes => "icons/book-open.svg",
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

#[cfg(test)]
mod tests {
    use super::display_path;
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
}
