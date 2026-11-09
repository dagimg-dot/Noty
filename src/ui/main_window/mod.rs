use std::path::PathBuf;
use std::time::SystemTime;

use gpui_kit::component::input::{Editor, EditorState, InputEvent, InputState, Rope, TabSize};
use gpui_kit::component::{ActiveTheme as _, WindowExt as _};
use gpui_kit::*;

use crate::config::Config;
use crate::format;
use crate::notes::NoteStore;
use crate::notes::saver::Saver;
use crate::theme::ThemeLibrary;
use crate::ui::keymap::{FocusSearch, Quit};
use crate::ui::search::{Search, SearchEvent};
use crate::ui::theme_bridge;
use crate::ui::widgets::{self, AppIcon, PRIMARY_KEY, Tone};

mod dialogs;
mod note_actions;
mod saving;
mod settings;
mod status_bar;
mod title_bar;

const TEXT_COLUMN: f32 = 760.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SaveState {
    Saved,
    Unsaved,
    Conflict,
    Failed,
}

struct OpenNote {
    path: PathBuf,
    name: SharedString,
    disk_modified: SystemTime,
    saved: Rope,
}

struct Renaming {
    path: PathBuf,
    input: Entity<InputState>,
    error: Option<SharedString>,
}

pub struct MainWindow {
    config: Config,
    themes: ThemeLibrary,
    store: NoteStore,
    editor: Entity<EditorState>,
    search: Entity<Search>,
    search_open: bool,
    current: Option<OpenNote>,
    save_state: SaveState,
    word_count: usize,
    font_size: f32,
    focus_handle: FocusHandle,
    autosave: Option<Task<()>>,
    saver: Saver,
    recount: Option<Task<()>>,
    renaming: Option<Renaming>,
    _subscriptions: Vec<Subscription>,
}

impl MainWindow {
    pub fn new(
        config: Config,
        themes: ThemeLibrary,
        store: NoteStore,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("markdown")
                .line_number(config.show_line_numbers())
                .soft_wrap(true)
                .folding(false)
                .indent_guides(false)
                .searchable(false)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .placeholder("Start writing…")
        });
        let search = cx.new(|cx| Search::new(window, cx));
        let query = search.read(cx).query().clone();
        let subscriptions = vec![
            cx.subscribe_in(&editor, window, Self::on_editor_event),
            cx.subscribe_in(&search, window, Self::on_search_event),
            cx.subscribe_in(&query, window, Self::on_query_event),
            cx.observe_window_activation(window, |this, window, cx| {
                if window.is_window_active() {
                    this.check_disk(window, cx);
                } else {
                    this.save_in_background(window, cx);
                }
            }),
            cx.observe_window_appearance(window, |this, window, cx| {
                theme_bridge::apply(&this.themes, &this.config, Some(window), cx);
            }),
        ];
        let this = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            this.update(cx, |this, cx| this.save_session(window, cx))
                .ok();
            true
        });
        cx.defer_in(window, Self::start);

        let font_size = config.font_size();
        Self {
            config,
            themes,
            store,
            editor,
            search,
            search_open: false,
            current: None,
            save_state: SaveState::Saved,
            word_count: 0,
            font_size,
            focus_handle: cx.focus_handle(),
            autosave: None,
            saver: Saver::default(),
            recount: None,
            renaming: None,
            _subscriptions: subscriptions,
        }
    }

    fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let last_note = self
            .config
            .last_opened_file()
            .filter(|path| path.starts_with(self.store.dir()) && path.is_file());
        if let Some(path) = last_note {
            self.open(&path, window, cx);
        }
        if self.current.is_none() {
            self.focus_search(window, cx);
        }
    }

    fn rescan(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = self.store.scan() {
            window.push_notification(
                widgets::notification(
                    Tone::Error,
                    format!(
                        "Couldn't read {}: {error}",
                        format::display_path(self.store.dir())
                    ),
                ),
                cx,
            );
        }
        cx.notify();
    }

    fn on_editor_event(
        &mut self,
        _: &Entity<EditorState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => self.on_edit(window, cx),
            InputEvent::Blur => self.save_in_background(window, cx),
            _ => {}
        }
    }

    fn on_query_event(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Focus => self.begin_search(window, cx),
            InputEvent::Blur => cx.defer_in(window, |this, window, cx| {
                if this.editor.read(cx).focus_handle(cx).is_focused(window) {
                    this.end_search(window, cx);
                }
            }),
            _ => {}
        }
    }

    fn on_search_event(
        &mut self,
        _: &Entity<Search>,
        event: &SearchEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            SearchEvent::Open(path) => self.open(&path.clone(), window, cx),
            SearchEvent::Create(name) => match self.store.create(name) {
                Ok(path) => {
                    self.open(&path, window, cx);
                    window.push_notification(
                        widgets::notification(Tone::Success, format!("Created “{}”", name.trim())),
                        cx,
                    );
                }
                Err(error) => window
                    .push_notification(widgets::notification(Tone::Error, error.to_string()), cx),
            },
            SearchEvent::Dismissed => self.focus_editor(window, cx),
        }
    }

    fn begin_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_open {
            return;
        }
        self.save_in_background(window, cx);
        self.rescan(window, cx);
        let notes = self.store.notes().to_vec();
        self.search
            .update(cx, |search, cx| search.begin(notes, window, cx));
        self.search_open = true;
        cx.notify();
    }

    fn end_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.search_open {
            return;
        }
        self.search_open = false;
        let name = self.current.as_ref().map(|note| note.name.clone());
        self.search
            .update(cx, |search, cx| search.show_current(name, window, cx));
        cx.notify();
    }

    /// Focuses the search field and opens the list. The field's own focus
    /// event only fires between rendered frames, so this does not wait
    /// for it.
    fn focus_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search
            .update(cx, |search, cx| search.focus(window, cx));
        self.begin_search(window, cx);
    }

    fn focus_search_action(
        &mut self,
        _: &FocusSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.search_open && self.current.is_some() {
            self.focus_editor(window, cx);
        } else {
            self.focus_search(window, cx);
        }
    }

    /// Saves the open note and remembers it, along with the window size,
    /// for the next launch.
    fn save_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save(false, window, cx);
        let size = window.window_bounds().get_bounds().size;
        self.config
            .set_window_size(size.width.as_f32(), size.height.as_f32());
        self.config
            .set_last_opened_file(self.current.as_ref().map(|note| note.path.as_path()));
        self.persist_config();
    }

    fn quit(&mut self, _: &Quit, window: &mut Window, cx: &mut Context<Self>) {
        self.save_session(window, cx);
        cx.quit();
    }

    fn render_editor(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let font = cx.theme().mono_font_family.clone();
        let width = window.viewport_size().width;
        let side = ((width - px(TEXT_COLUMN)) / 2.).max(px(20.));
        div()
            .key_context("NotyEditor")
            .size_full()
            .pl(side)
            .pr(side)
            .pt(px(12.))
            .child(
                Editor::new(&self.editor)
                    .appearance(false)
                    .bordered(false)
                    .h_full()
                    .font_family(font)
                    .text_size(px(self.font_size * 4. / 3.))
                    .line_height(relative(1.6)),
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
            .child(widgets::icon(
                AppIcon::Notes,
                px(30.),
                theme.muted_foreground,
            ))
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
                        format::display_path(self.store.dir())
                    )),
            )
            .child(
                div()
                    .mt(px(14.))
                    .flex()
                    .flex_col()
                    .gap(px(9.))
                    .child(widgets::shortcut_hint_groups(
                        &[&[PRIMARY_KEY, "P"], &["Alt", "S"]],
                        "Search or create",
                        cx,
                    ))
                    .child(widgets::shortcut_hint_groups(
                        &[&["↑", "↓"], &["Ctrl", "J", "K"]],
                        "Move through notes",
                        cx,
                    ))
                    .child(widgets::shortcut_hint(&["↵"], "Open, or create if new", cx)),
            )
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (background, foreground, font_family) = (
            theme.background,
            theme.foreground,
            theme.font_family.clone(),
        );
        let body = if self.current.is_some() {
            self.render_editor(window, cx).into_any_element()
        } else {
            self.render_welcome(cx).into_any_element()
        };
        let title_bar = self.render_title_bar(window, cx);
        let status_bar = self.render_status_bar(cx);

        div()
            .id("main-window")
            .key_context("NotyMainWindow")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::focus_search_action))
            .on_action(cx.listener(Self::save_action))
            .on_action(cx.listener(Self::reload_action))
            .on_action(cx.listener(Self::rename_action))
            .on_action(cx.listener(Self::delete_action))
            .on_action(cx.listener(Self::select_theme))
            .on_action(cx.listener(Self::follow_system_appearance))
            .on_action(cx.listener(Self::use_light_appearance))
            .on_action(cx.listener(Self::use_dark_appearance))
            .on_action(cx.listener(Self::toggle_line_numbers))
            .on_action(cx.listener(Self::toggle_close_button))
            .on_action(cx.listener(Self::show_shortcuts))
            .on_action(cx.listener(Self::show_about))
            .on_action(cx.listener(Self::zoom_in))
            .on_action(cx.listener(Self::zoom_out))
            .on_action(cx.listener(Self::zoom_reset))
            .on_action(cx.listener(Self::quit))
            .size_full()
            .flex()
            .flex_col()
            .bg(background)
            .text_color(foreground)
            .font_family(font_family)
            .child(title_bar)
            .children(self.search_open.then(|| self.search.clone()))
            .child(div().flex_1().min_h_0().child(body))
            .child(status_bar)
    }
}
