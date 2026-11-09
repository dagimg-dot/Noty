use std::sync::Arc;

use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName};
use gpui_kit::*;

pub const PRIMARY_KEY: &str = if cfg!(target_os = "macos") {
    "⌘"
} else {
    "Ctrl"
};

const APP_ICON: &[u8] = include_bytes!("../data/icons/hicolor/scalable/apps/com.dagimg.noty.svg");

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

#[derive(Debug, Clone, Copy)]
pub enum Tone {
    Info,
    Success,
    Warning,
    Error,
}

pub fn notification(tone: Tone, message: impl Into<SharedString>) -> Notification {
    let message = message.into();
    Notification::new().content(move |_, _, cx| {
        let theme = cx.theme();
        let (icon, color) = match tone {
            Tone::Info => (IconName::Info, theme.info),
            Tone::Success => (IconName::CircleCheck, theme.success),
            Tone::Warning => (IconName::TriangleAlert, theme.warning),
            Tone::Error => (IconName::CircleX, theme.danger),
        };
        div()
            .flex()
            .items_center()
            .gap(px(12.))
            .child(div().flex_none().child(Icon::new(icon).text_color(color)))
            .child(div().flex_1().min_w_0().text_sm().child(message.clone()))
            .into_any_element()
    })
}

pub fn app_logo(size: Pixels) -> Img {
    img(Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        APP_ICON.to_vec(),
    )))
    .size(size)
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
