use std::rc::Rc;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::Icon;
use gpui_kit::component::IconName;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::TitleBar;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::DropdownMenu as _;
use gpui_kit::component::menu::PopupMenu;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{MainWindow, SaveState};
use crate::config::Appearance;
use crate::theme::Variant;
use crate::ui::keymap::Confirm;
use crate::ui::keymap::ConfirmCreate;
use crate::ui::keymap::DeleteNote;
use crate::ui::keymap::Dismiss;
use crate::ui::keymap::FollowSystemAppearance;
use crate::ui::keymap::Quit;
use crate::ui::keymap::RenameNote;
use crate::ui::keymap::SelectNext;
use crate::ui::keymap::SelectPrev;
use crate::ui::keymap::SelectTheme;
use crate::ui::keymap::ShowAbout;
use crate::ui::keymap::ShowShortcuts;
use crate::ui::keymap::ToggleCloseButton;
use crate::ui::keymap::ToggleLineNumbers;
use crate::ui::keymap::UseDarkAppearance;
use crate::ui::keymap::UseLightAppearance;
use crate::ui::keymap::ZoomIn;
use crate::ui::keymap::ZoomOut;
use crate::ui::keymap::ZoomReset;
use crate::ui::widgets;
use crate::ui::widgets::AppIcon;

struct ThemeChoice {
    id: SharedString,
    name: SharedString,
    variant: Variant,
}

impl MainWindow {
    pub(super) fn render_title_bar(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let menu = self.render_menu_button(cx);
        let theme = cx.theme();
        let this = cx.entity().downgrade();
        let show_close = self.config.show_close_button()
            && cfg!(target_os = "linux")
            && matches!(window.window_decorations(), Decorations::Server);
        let dirty = !self.search_open
            && matches!(self.save_state, SaveState::Unsaved | SaveState::Conflict);
        let query = self.search.read(cx).query().clone();
        let search_context = if self.search.read(cx).is_navigating() {
            "NotySearch navigating"
        } else {
            "NotySearch"
        };
        let close = div()
            .id("close-window")
            .flex_none()
            .size(px(34.))
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.input)
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.muted)
            .hover(|style| style.bg(theme.accent))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(move |_, window, cx| {
                this.update(cx, |this, cx| this.save_session(window, cx))
                    .ok();
                window.remove_window();
            })
            .child(widgets::icon(
                AppIcon::Close,
                px(14.),
                theme.muted_foreground,
            ));

        let unsaved_dot = div()
            .size(px(7.))
            .mr(px(4.))
            .rounded_full()
            .bg(theme.primary);
        let field = Input::new(&query)
            .small()
            .cleanable(true)
            .focus_bordered(false)
            .prefix(widgets::icon(
                AppIcon::Search,
                px(14.),
                theme.muted_foreground,
            ))
            .when(dirty, |field| field.suffix(unsaved_dot));
        let search_field = div()
            .key_context(search_context)
            .flex_1()
            .min_w_0()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_action(cx.listener(|this, action: &SelectNext, window, cx| {
                this.search
                    .update(cx, |search, cx| search.select_next(action, window, cx))
            }))
            .on_action(cx.listener(|this, action: &SelectPrev, window, cx| {
                this.search
                    .update(cx, |search, cx| search.select_prev(action, window, cx))
            }))
            .on_action(cx.listener(|this, action: &Confirm, window, cx| {
                this.search
                    .update(cx, |search, cx| search.confirm(action, window, cx))
            }))
            .on_action(cx.listener(|this, action: &ConfirmCreate, window, cx| {
                this.search
                    .update(cx, |search, cx| search.confirm_create(action, window, cx))
            }))
            .on_action(cx.listener(|this, action: &Dismiss, window, cx| {
                this.search
                    .update(cx, |search, cx| search.dismiss(action, window, cx))
            }))
            .child(Styled::h(field, px(34.)));

        TitleBar::new().h(px(47.)).pl(px(6.)).child(
            div()
                .size_full()
                .flex()
                .items_center()
                .gap(px(6.))
                .pr(px(6.))
                .when(show_close, |bar| bar.child(close))
                .child(search_field)
                .child(menu),
        )
    }

    fn render_menu_button(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let has_note = self.current.is_some();
        let line_numbers = self.config.show_line_numbers();
        let close_button = self.config.show_close_button();
        let appearance = self.config.appearance();
        let light = SharedString::from(self.config.theme_id(Variant::Light));
        let dark = SharedString::from(self.config.theme_id(Variant::Dark));
        let themes: Rc<Vec<ThemeChoice>> = Rc::new(
            self.themes
                .themes()
                .iter()
                .map(|theme| ThemeChoice {
                    id: theme.id().to_owned().into(),
                    name: theme.name().to_owned().into(),
                    variant: theme.variant(),
                })
                .collect(),
        );

        div()
            .flex_none()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    cx.stop_propagation();
                    this.focus_editor(window, cx);
                }),
            )
            .child(
                Button::new("main-menu")
                    .outline()
                    .small()
                    .size(px(34.))
                    .icon(Icon::new(IconName::Ellipsis))
                    .tooltip("Main Menu")
                    .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, window, cx| {
                        let menu = if has_note {
                            menu.menu("Rename Note…", Box::new(RenameNote))
                                .menu("Delete Note…", Box::new(DeleteNote))
                                .separator()
                        } else {
                            menu
                        };
                        let (themes, light, dark) = (themes.clone(), light.clone(), dark.clone());
                        menu.submenu("Theme", window, cx, move |menu, _, _| {
                            theme_menu(menu, &themes, &light, &dark)
                        })
                        .submenu("Appearance", window, cx, move |menu, _, _| {
                            menu.menu_with_check(
                                "Follow System",
                                appearance == Appearance::System,
                                Box::new(FollowSystemAppearance),
                            )
                            .menu_with_check(
                                "Light",
                                appearance == Appearance::Light,
                                Box::new(UseLightAppearance),
                            )
                            .menu_with_check(
                                "Dark",
                                appearance == Appearance::Dark,
                                Box::new(UseDarkAppearance),
                            )
                        })
                        .menu_with_check("Line Numbers", line_numbers, Box::new(ToggleLineNumbers))
                        .menu_with_check("Close Button", close_button, Box::new(ToggleCloseButton))
                        .separator()
                        .menu("Bigger Text", Box::new(ZoomIn))
                        .menu("Smaller Text", Box::new(ZoomOut))
                        .menu("Actual Size", Box::new(ZoomReset))
                        .separator()
                        .menu("Keyboard Shortcuts", Box::new(ShowShortcuts))
                        .menu("About Noty", Box::new(ShowAbout))
                        .menu("Quit", Box::new(Quit))
                    }),
            )
    }
}

fn theme_menu(menu: PopupMenu, themes: &[ThemeChoice], light: &str, dark: &str) -> PopupMenu {
    [
        (Variant::Light, "Light", light),
        (Variant::Dark, "Dark", dark),
    ]
    .into_iter()
    .enumerate()
    .fold(menu, |menu, (ix, (variant, label, selected))| {
        let menu = if ix > 0 { menu.separator() } else { menu };
        themes.iter().filter(|theme| theme.variant == variant).fold(
            menu.label(label),
            |menu, theme| {
                menu.menu_with_check(
                    theme.name.clone(),
                    theme.id == selected,
                    Box::new(SelectTheme {
                        id: theme.id.clone(),
                    }),
                )
            },
        )
    })
}
