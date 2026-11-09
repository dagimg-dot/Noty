use gpui_kit::*;

use super::MainWindow;
use crate::config::Appearance;
use crate::config::Config;
use crate::config::MAX_FONT_SIZE;
use crate::config::MIN_FONT_SIZE;
use crate::theme::Variant;
use crate::ui::keymap::FollowSystemAppearance;
use crate::ui::keymap::SelectTheme;
use crate::ui::keymap::ToggleCloseButton;
use crate::ui::keymap::ToggleLineNumbers;
use crate::ui::keymap::UseDarkAppearance;
use crate::ui::keymap::UseLightAppearance;
use crate::ui::keymap::ZoomIn;
use crate::ui::keymap::ZoomOut;
use crate::ui::keymap::ZoomReset;
use crate::ui::theme_bridge;

impl MainWindow {
    fn showing_variant(&self, window: &Window) -> Variant {
        match self.config.appearance() {
            Appearance::Light => Variant::Light,
            Appearance::Dark => Variant::Dark,
            Appearance::System => match window.appearance() {
                WindowAppearance::Dark | WindowAppearance::VibrantDark => Variant::Dark,
                WindowAppearance::Light | WindowAppearance::VibrantLight => Variant::Light,
            },
        }
    }

    /// Uses the theme for its variant and, when the window shows the other
    /// variant, switches the appearance so the choice is visible right away.
    pub(super) fn select_theme(
        &mut self,
        action: &SelectTheme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(variant) = self.themes.get(&action.id).map(|theme| theme.variant()) else {
            return;
        };
        self.config.set_theme_id(variant, &action.id);
        if self.showing_variant(window) != variant {
            self.config.set_appearance(match variant {
                Variant::Light => Appearance::Light,
                Variant::Dark => Appearance::Dark,
            });
        }
        self.settings_changed(window, cx);
    }

    fn set_appearance(
        &mut self,
        appearance: Appearance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.config.set_appearance(appearance);
        self.settings_changed(window, cx);
    }

    pub(super) fn follow_system_appearance(
        &mut self,
        _: &FollowSystemAppearance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_appearance(Appearance::System, window, cx);
    }

    pub(super) fn use_light_appearance(
        &mut self,
        _: &UseLightAppearance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_appearance(Appearance::Light, window, cx);
    }

    pub(super) fn use_dark_appearance(
        &mut self,
        _: &UseDarkAppearance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_appearance(Appearance::Dark, window, cx);
    }

    fn settings_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.persist_config();
        theme_bridge::apply(&self.themes, &self.config, Some(window), cx);
        cx.notify();
    }

    pub(super) fn toggle_line_numbers(
        &mut self,
        _: &ToggleLineNumbers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let show = !self.config.show_line_numbers();
        self.config.set_show_line_numbers(show);
        self.editor
            .update(cx, |editor, cx| editor.set_line_number(show, window, cx));
        self.persist_config();
        cx.notify();
    }

    pub(super) fn toggle_close_button(
        &mut self,
        _: &ToggleCloseButton,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let show = !self.config.show_close_button();
        self.config.set_show_close_button(show);
        self.persist_config();
        cx.notify();
    }

    fn zoom(&mut self, points: f32, cx: &mut Context<Self>) {
        self.font_size = points.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        self.config.set_font_size(self.font_size);
        self.persist_config();
        cx.notify();
    }

    pub(super) fn zoom_in(&mut self, _: &ZoomIn, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom(self.font_size + 1., cx);
    }

    pub(super) fn zoom_out(&mut self, _: &ZoomOut, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom(self.font_size - 1., cx);
    }

    pub(super) fn zoom_reset(&mut self, _: &ZoomReset, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom(Config::default_font_size(), cx);
    }

    pub(super) fn persist_config(&self) {
        if let Err(error) = self.config.save() {
            eprintln!("noty: could not save the settings: {error}");
        }
    }
}
