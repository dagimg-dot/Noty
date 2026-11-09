use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::*;

use super::MainWindow;
use crate::ui::keymap::ShowAbout;
use crate::ui::keymap::ShowShortcuts;
use crate::ui::widgets;
use crate::ui::widgets::PRIMARY_KEY;

const HOMEPAGE: &str = "https://github.com/dagimg-dot/noty";

impl MainWindow {
    pub(super) fn show_shortcuts(
        &mut self,
        _: &ShowShortcuts,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.open_dialog(cx, |dialog, _, cx| {
            dialog
                .title("Keyboard Shortcuts")
                .w(px(460.))
                .child(shortcut_list(cx))
        });
    }

    pub(super) fn show_about(
        &mut self,
        _: &ShowAbout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.open_dialog(cx, |dialog, _, cx| dialog.w(px(340.)).child(about(cx)));
    }
}

fn shortcut_list(cx: &App) -> Div {
    const SHORTCUTS: &[(&[&[&str]], &str)] = &[
        (
            &[&[PRIMARY_KEY, "P"], &["Alt", "S"], &["Esc"]],
            "Search notes",
        ),
        (&[&["↑", "↓"], &["Ctrl", "J", "K"]], "Move through the list"),
        (&[&["↵"]], "Open, or create a new note"),
        (&[&["⇧", "↵"]], "Create exactly what you typed"),
        (&[&["F2"], &["Ctrl", "R"]], "Rename note"),
        (&[&["Delete"], &["⇧", "Delete"]], "Delete note"),
        (&[&[PRIMARY_KEY, "S"]], "Save now, or keep your version"),
        (&[&[PRIMARY_KEY, "⇧", "R"]], "Reload from disk"),
        (
            &[
                &[PRIMARY_KEY, "="],
                &[PRIMARY_KEY, "-"],
                &[PRIMARY_KEY, "0"],
            ],
            "Text size",
        ),
        (&[&["Alt", "L"]], "Line numbers"),
        (&[&[PRIMARY_KEY, "Q"]], "Quit"),
    ];
    let theme = cx.theme();
    let rows = SHORTCUTS.iter().map(|(groups, label)| {
        let keycaps = groups.iter().enumerate().flat_map(|(ix, keys)| {
            let separator = (ix > 0).then(|| {
                div()
                    .px(px(2.))
                    .text_size(px(11.))
                    .text_color(theme.muted_foreground)
                    .child("/")
                    .into_any_element()
            });
            separator
                .into_iter()
                .chain(
                    keys.iter()
                        .map(|key| widgets::kbd(*key, cx).into_any_element()),
                )
                .collect::<Vec<_>>()
        });
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(16.))
            .child(div().text_size(px(13.)).child(*label))
            .child(div().flex().items_center().gap(px(4.)).children(keycaps))
    });
    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .pb(px(6.))
        .children(rows)
}

fn about(cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.))
        .py(px(10.))
        .child(widgets::app_logo(px(72.)))
        .child(
            div()
                .mt(px(6.))
                .text_size(px(22.))
                .font_weight(FontWeight::SEMIBOLD)
                .child("Noty"),
        )
        .child(
            div()
                .text_size(px(13.))
                .text_color(theme.muted_foreground)
                .child(format!("Version {}", env!("CARGO_PKG_VERSION"))),
        )
        .child(
            div()
                .mt(px(6.))
                .text_size(px(13.))
                .text_center()
                .child("A minimal keyboard driven note taking application"),
        )
        .child(
            div()
                .id("homepage")
                .cursor_pointer()
                .text_size(px(12.))
                .text_color(theme.link)
                .hover(|link| link.underline())
                .on_click(|_, _, cx| cx.open_url(HOMEPAGE))
                .child(HOMEPAGE.trim_start_matches("https://")),
        )
}
