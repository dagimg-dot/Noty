use gpui_kit::component::notification::Notification;
use gpui_kit::component::{ActiveTheme as _, TitleBar, WindowExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::actions::Quit;
use crate::appearance;
use crate::config::Config;
use crate::notes::NoteStore;
use crate::theme::ThemeLibrary;
use crate::ui::{self, AppIcon};

pub struct Workspace {
    config: Config,
    themes: ThemeLibrary,
    store: NoteStore,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    pub fn new(
        config: Config,
        themes: ThemeLibrary,
        store: NoteStore,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscriptions = vec![cx.observe_window_appearance(window, |this, window, cx| {
            appearance::apply(&this.themes, &this.config, Some(window), cx);
        })];
        cx.defer_in(window, |this, window, cx| {
            window.focus(&this.focus_handle, cx);
            this.rescan(window, cx);
        });
        Self {
            config,
            themes,
            store,
            focus_handle: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }

    fn rescan(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = self.store.scan() {
            window.push_notification(
                Notification::error(format!(
                    "Couldn't read {}: {error}",
                    ui::display_path(self.store.dir())
                )),
                cx,
            );
        }
        cx.notify();
    }

    fn quit(&mut self, _: &Quit, _: &mut Window, cx: &mut Context<Self>) {
        cx.quit();
    }

    fn render_title_bar(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let show_close =
            cfg!(target_os = "linux") && matches!(window.window_decorations(), Decorations::Server);
        let close = div()
            .id("close-window")
            .flex_none()
            .size(px(24.))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.muted)
            .hover(|style| style.bg(theme.accent))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(|_, window, _| window.remove_window())
            .child(ui::icon(AppIcon::Close, px(12.), theme.muted_foreground));

        TitleBar::new().child(
            div()
                .size_full()
                .flex()
                .items_center()
                .pr(px(6.))
                .when(show_close, |bar| bar.child(close))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .justify_center()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.muted_foreground)
                        .child("Noty"),
                ),
        )
    }

    fn render_welcome(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let count = self.store.notes().len();
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .px(px(24.))
            .child(ui::icon(AppIcon::Notes, px(30.), theme.muted_foreground))
            .child(
                div()
                    .mt(px(4.))
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .child("No note open"),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(theme.muted_foreground)
                    .text_center()
                    .child(format!(
                        "{count} {} in {}",
                        if count == 1 { "note" } else { "notes" },
                        ui::display_path(self.store.dir())
                    )),
            )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let theme = cx.theme();
        div()
            .h(px(26.))
            .px(px(12.))
            .flex()
            .items_center()
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.status_bar)
            .text_size(px(12.))
            .text_color(theme.muted_foreground)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(ui::display_path(self.store.dir())),
            )
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (background, foreground, font_family) = (
            theme.background,
            theme.foreground,
            theme.font_family.clone(),
        );
        let title_bar = self.render_title_bar(window, cx);
        let welcome = self.render_welcome(cx);
        let status_bar = self.render_status_bar(cx);

        div()
            .id("workspace")
            .key_context("NotyWorkspace")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::quit))
            .size_full()
            .flex()
            .flex_col()
            .bg(background)
            .text_color(foreground)
            .font_family(font_family)
            .child(title_bar)
            .child(div().flex_1().min_h_0().child(welcome))
            .child(status_bar)
    }
}
