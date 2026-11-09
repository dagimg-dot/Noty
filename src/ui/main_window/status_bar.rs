use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{MainWindow, SaveState};
use crate::config::Config;
use crate::format;
use crate::ui::widgets;
use crate::ui::widgets::AppIcon;

impl MainWindow {
    pub(super) fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let (state_icon, state_label, state_color) = match self.save_state {
            SaveState::Saved => (AppIcon::Saved, "Saved", theme.muted_foreground),
            SaveState::Unsaved => (AppIcon::Unsaved, "Editing", theme.muted_foreground),
            SaveState::Conflict => (AppIcon::Warning, "Changed on disk", theme.warning),
            SaveState::Failed => (AppIcon::Warning, "Not saved", theme.danger),
        };
        let zoom = (self.font_size / Config::default_font_size() * 100.).round() as i32;
        let location = match &self.current {
            Some(note) => format::display_path(&note.path),
            None => format::display_path(self.store.dir()),
        };

        div()
            .h(px(26.))
            .px(px(12.))
            .flex()
            .items_center()
            .gap(px(14.))
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.status_bar)
            .text_size(px(12.))
            .text_color(theme.muted_foreground)
            .child(div().flex_1().min_w_0().truncate().child(location))
            .when(zoom != 100, |bar| bar.child(format!("{zoom}%")))
            .when(self.current.is_some(), |bar| {
                bar.child(format!(
                    "{} {}",
                    self.word_count,
                    if self.word_count == 1 {
                        "word"
                    } else {
                        "words"
                    }
                ))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_color(state_color)
                        .child(widgets::icon(state_icon, px(12.), state_color))
                        .child(state_label),
                )
            })
    }
}
