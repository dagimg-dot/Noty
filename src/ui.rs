use std::path::Path;
use std::time::{Duration, SystemTime};

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use crate::config::home_dir;

pub const PRIMARY_KEY: &str = if cfg!(target_os = "macos") {
    "⌘"
} else {
    "Ctrl"
};

#[derive(Debug, Clone, Copy)]
pub enum AppIcon {
    Close,
    Create,
    Note,
    Notes,
    Search,
    Saved,
    Unsaved,
    Warning,
}

impl AppIcon {
    fn path(self) -> &'static str {
        match self {
            AppIcon::Close => "icons/close.svg",
            AppIcon::Create => "icons/plus.svg",
            AppIcon::Note => "icons/file-text.svg",
            AppIcon::Notes => "icons/book-open.svg",
            AppIcon::Search => "icons/search.svg",
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

pub fn kbd(label: impl Into<SharedString>, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .justify_center()
        .min_w(px(20.))
        .h(px(20.))
        .px(px(5.))
        .rounded(px(5.))
        .border_1()
        .border_b_2()
        .border_color(theme.border)
        .bg(theme.muted)
        .text_color(theme.muted_foreground)
        .text_size(px(11.))
        .font_weight(FontWeight::MEDIUM)
        .child(label.into())
}

pub fn shortcut_hint(keys: &[&'static str], label: &'static str, cx: &App) -> Div {
    shortcut_hint_groups(&[keys], label, cx)
}

/// Keycaps for one action with several shortcuts, like "↑ ↓ / Ctrl J K".
pub fn shortcut_hint_groups(groups: &[&[&'static str]], label: &'static str, cx: &App) -> Div {
    let muted = cx.theme().muted_foreground;
    let keycaps = groups.iter().enumerate().flat_map(|(ix, keys)| {
        let separator = (ix > 0).then(|| {
            div()
                .px(px(2.))
                .text_size(px(11.))
                .text_color(muted)
                .child("/")
                .into_any_element()
        });
        separator
            .into_iter()
            .chain(keys.iter().map(|key| kbd(*key, cx).into_any_element()))
            .collect::<Vec<_>>()
    });
    div()
        .flex()
        .items_center()
        .gap(px(4.))
        .children(keycaps)
        .child(
            div()
                .ml(px(4.))
                .text_size(px(12.))
                .text_color(muted)
                .child(label),
        )
}

pub fn relative_time(time: SystemTime) -> String {
    let seconds = SystemTime::now()
        .duration_since(time)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    match seconds {
        0..60 => "just now".into(),
        60..3_600 => format!("{}m ago", seconds / 60),
        3_600..86_400 => format!("{}h ago", seconds / 3_600),
        86_400..2_592_000 => format!("{}d ago", seconds / 86_400),
        _ => format!("{}mo ago", seconds / 2_592_000),
    }
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
    use std::time::{Duration, SystemTime};

    use super::{display_path, relative_time, word_count};
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

    #[test]
    fn relative_times_round_down_to_the_largest_unit() {
        let now = SystemTime::now();
        assert_eq!(relative_time(now), "just now");
        assert_eq!(relative_time(now - Duration::from_secs(5 * 60)), "5m ago");
        assert_eq!(
            relative_time(now - Duration::from_secs(3 * 3_600)),
            "3h ago"
        );
        assert_eq!(
            relative_time(now - Duration::from_secs(2 * 86_400)),
            "2d ago"
        );
    }
}
