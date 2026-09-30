use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_kit::component::input::{
    Editor, EditorState, Input, InputEvent, InputState, Rope, TabSize,
};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, TitleBar, WindowExt as _,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::actions::{
    Confirm, ConfirmCreate, DeleteNote, Dismiss, FocusSearch, FollowSystemAppearance, Quit,
    ReloadNote, RenameNote, SaveNote, SelectNext, SelectPrev, SelectTheme, ShowAbout,
    ShowShortcuts, ToggleCloseButton, ToggleLineNumbers, UseDarkAppearance, UseLightAppearance,
    ZoomIn, ZoomOut, ZoomReset,
};
use crate::appearance;
use crate::config::{Appearance, Config, MAX_FONT_SIZE, MIN_FONT_SIZE};
use crate::notes::{self, NoteStore, SaveOutcome};
use crate::saver::{FinishedSave, SaveJob, Saver};
use crate::search::{Search, SearchEvent};
use crate::theme::{ThemeLibrary, Variant};
use crate::ui::{self, AppIcon, PRIMARY_KEY};

const AUTOSAVE_DELAY: Duration = Duration::from_millis(800);
const WORD_COUNT_DELAY: Duration = Duration::from_millis(250);
const TEXT_COLUMN: f32 = 760.;
const HOMEPAGE: &str = "https://github.com/dagimg-dot/noty";

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

struct ThemeChoice {
    id: SharedString,
    name: SharedString,
    variant: Variant,
}

struct Renaming {
    path: PathBuf,
    input: Entity<InputState>,
    error: Option<SharedString>,
}

pub struct Workspace {
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

impl Workspace {
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
                appearance::apply(&this.themes, &this.config, Some(window), cx);
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
                Notification::error(format!(
                    "Couldn't read {}: {error}",
                    ui::display_path(self.store.dir())
                )),
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
                        Notification::success(format!("Created “{}”", name.trim())),
                        cx,
                    );
                }
                Err(error) => window.push_notification(Notification::error(error.to_string()), cx),
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

    fn on_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(current) = &self.current else {
            return;
        };
        self.recount = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(WORD_COUNT_DELAY).await;
            this.update(cx, |this, cx| {
                this.word_count = ui::word_count(this.editor.read(cx).text().chunks());
                cx.notify();
            })
            .ok();
        }));
        if self.save_state != SaveState::Conflict {
            let dirty = self.editor.read(cx).text() != &current.saved;
            self.save_state = if dirty {
                SaveState::Unsaved
            } else {
                SaveState::Saved
            };
            if dirty {
                self.schedule_autosave(window, cx);
            }
        }
        cx.notify();
    }

    fn schedule_autosave(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.autosave = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(AUTOSAVE_DELAY).await;
            this.update_in(cx, |this, window, cx| this.save_in_background(window, cx))
                .ok();
        }));
    }

    /// Writes the open note right away, once any save running in the
    /// background has finished. With `force`, a note that changed on disk is
    /// overwritten, which is how the user keeps their version of a conflict.
    fn save(&mut self, force: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.autosave = None;
        self.finish_saving(window, cx);
        if let Some(job) = self.save_job(force, cx) {
            self.apply_save(job.run(), window, cx);
        }
    }

    fn save_in_background(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.autosave = None;
        if let Some(finished) = self.saver.take() {
            self.apply_save(finished, window, cx);
        }
        let Some(job) = self.save_job(false, cx) else {
            return;
        };
        if !self.saver.try_begin() {
            return;
        }
        let saver = self.saver.clone();
        let io = cx.background_spawn(async move { saver.run(job) });
        cx.spawn_in(window, async move |this, cx| {
            io.await;
            this.update_in(cx, |this, window, cx| this.collect_save(window, cx))
                .ok();
        })
        .detach();
    }

    fn save_job(&self, force: bool, cx: &App) -> Option<SaveJob> {
        let current = self.current.as_ref()?;
        let content = self.editor.read(cx).text().clone();
        if !force && (content == current.saved || self.save_state == SaveState::Conflict) {
            return None;
        }
        Some(SaveJob {
            path: current.path.clone(),
            content,
            expected_modified: (!force).then_some(current.disk_modified),
        })
    }

    fn finish_saving(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(finished) = self.saver.wait() {
            self.apply_save(finished, window, cx);
        }
    }

    fn collect_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(finished) = self.saver.take() {
            self.apply_save(finished, window, cx);
        }
        if self.save_state == SaveState::Unsaved && self.autosave.is_none() {
            self.schedule_autosave(window, cx);
        }
    }

    fn apply_save(&mut self, finished: FinishedSave, window: &mut Window, cx: &mut Context<Self>) {
        let Some(current) = self
            .current
            .as_mut()
            .filter(|note| note.path == finished.path)
        else {
            return;
        };
        match finished.outcome {
            Ok(SaveOutcome::Saved(modified)) => {
                current.disk_modified = modified;
                current.saved = finished.content;
                self.save_state = if self.editor.read(cx).text() == &current.saved {
                    SaveState::Saved
                } else {
                    SaveState::Unsaved
                };
            }
            Ok(SaveOutcome::Conflict) => {
                self.save_state = SaveState::Conflict;
                window.push_notification(
                    Notification::warning(format!(
                        "“{}” changed outside Noty. Save keeps your version, \
                         Reload loads theirs.",
                        current.name
                    ))
                    .autohide(false),
                    cx,
                );
            }
            Err(error) => {
                self.save_state = SaveState::Failed;
                window.push_notification(
                    Notification::error(format!("Couldn't save “{}”: {error}", current.name)),
                    cx,
                );
            }
        }
        cx.notify();
    }

    fn check_disk(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_saving(window, cx);
        let Some(current) = &self.current else {
            return;
        };
        let on_disk = notes::modified_time(&current.path).ok();
        if on_disk == Some(current.disk_modified) {
            return;
        }
        let dirty = self.editor.read(cx).text() != &current.saved;
        if on_disk.is_some() && !dirty {
            let (name, path) = (current.name.clone(), current.path.clone());
            self.load(&path, false, window, cx);
            window.push_notification(
                Notification::info(format!("Reloaded “{name}”, it changed outside Noty.")),
                cx,
            );
        } else if self.save_state != SaveState::Conflict {
            self.save(false, window, cx);
        }
    }

    fn save_action(&mut self, _: &SaveNote, window: &mut Window, cx: &mut Context<Self>) {
        let force = matches!(self.save_state, SaveState::Conflict | SaveState::Failed);
        self.save(force, window, cx);
        if force && self.save_state == SaveState::Saved {
            window.push_notification(Notification::success("Saved your version."), cx);
        }
    }

    fn reload_action(&mut self, _: &ReloadNote, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_saving(window, cx);
        if let Some(path) = self.current.as_ref().map(|note| note.path.clone()) {
            self.load(&path, false, window, cx);
        }
    }

    /// Opens `path`, saving the current note first. Edits that could not
    /// be saved are never dropped: the switch is refused until they are
    /// saved or reloaded.
    fn open(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        if self.current.as_ref().is_some_and(|note| note.path == path) {
            self.focus_editor(window, cx);
            return;
        }
        self.save(false, window, cx);
        if self.current.is_some()
            && matches!(self.save_state, SaveState::Conflict | SaveState::Failed)
        {
            window.push_notification(
                Notification::warning(
                    "This note has edits that couldn't be saved. Save or reload it first.",
                ),
                cx,
            );
            return;
        }
        self.load(path, true, window, cx);
    }

    fn load(&mut self, path: &Path, focus: bool, window: &mut Window, cx: &mut Context<Self>) {
        let (text, modified) = match notes::read_note(path) {
            Ok(loaded) => loaded,
            Err(error) => {
                window.push_notification(
                    Notification::error(format!(
                        "Couldn't open {}: {error}",
                        ui::display_path(path)
                    )),
                    cx,
                );
                return;
            }
        };
        let name = SharedString::from(notes::note_name(path));
        self.autosave = None;
        self.recount = None;
        self.word_count = ui::word_count([text.as_str()]);
        self.save_state = SaveState::Saved;
        self.editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        self.current = Some(OpenNote {
            path: path.to_path_buf(),
            name: name.clone(),
            disk_modified: modified,
            saved: self.editor.read(cx).text().clone(),
        });
        if !self.search_open {
            let shown = Some(name.clone());
            self.search
                .update(cx, |search, cx| search.show_current(shown, window, cx));
        }
        if focus {
            self.focus_editor(window, cx);
        }
        window.set_window_title(&format!("{name} — Noty"));
        cx.notify();
    }

    fn focus_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.current.is_some() {
            self.editor
                .update(cx, |editor, cx| editor.focus(window, cx));
            self.end_search(window, cx);
        }
    }

    fn target_note(&self, cx: &App) -> Option<(PathBuf, SharedString)> {
        if self.search_open {
            self.search.read(cx).selected_note()
        } else {
            self.current
                .as_ref()
                .map(|note| (note.path.clone(), note.name.clone()))
        }
    }

    fn refresh_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.rescan(window, cx);
        if self.search_open {
            let notes = self.store.notes().to_vec();
            self.search
                .update(cx, |search, cx| search.set_notes(notes, cx));
        }
    }

    fn rename_action(&mut self, _: &RenameNote, window: &mut Window, cx: &mut Context<Self>) {
        let Some((path, name)) = self.target_note(cx) else {
            return;
        };
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        self.renaming = Some(Renaming {
            path,
            input: input.clone(),
            error: None,
        });

        let workspace = cx.entity().downgrade();
        let field = input.clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            let error = workspace
                .upgrade()
                .and_then(|workspace| workspace.read(cx).renaming.as_ref()?.error.clone());
            let (submit, confirm) = (workspace.clone(), workspace.clone());
            dialog
                .title("Rename note")
                .w(px(360.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(Input::new(&field))
                        .when_some(error, |form, error| {
                            form.child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(cx.theme().danger)
                                    .child(error),
                            )
                        }),
                )
                .footer(
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            Button::new("cancel-rename")
                                .label("Cancel")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("confirm-rename")
                                .primary()
                                .label("Rename")
                                .on_click(move |_, window, cx| {
                                    let renamed = submit
                                        .update(cx, |this, cx| this.submit_rename(window, cx))
                                        .unwrap_or(false);
                                    if renamed {
                                        window.close_dialog(cx);
                                    }
                                }),
                        ),
                )
                .on_ok(move |_, window, cx| {
                    confirm
                        .update(cx, |this, cx| this.submit_rename(window, cx))
                        .unwrap_or(false)
                })
        });
        window.defer(cx, move |window, cx| {
            input.update(cx, |input, cx| {
                input.focus(window, cx);
                input.select_all(window, cx);
            });
        });
    }

    /// Renames the note being renamed and reports whether it worked, so the
    /// dialog can stay open to show why it did not.
    fn submit_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(renaming) = &self.renaming else {
            return true;
        };
        let path = renaming.path.clone();
        let new_name = renaming.input.read(cx).value().to_string();
        if self.current.as_ref().is_some_and(|note| note.path == path) {
            self.save(false, window, cx);
        }
        match self.store.rename(&path, &new_name) {
            Ok(new_path) => {
                self.renaming = None;
                let name = SharedString::from(notes::note_name(&new_path));
                if let Some(current) = self.current.as_mut().filter(|note| note.path == path) {
                    current.path = new_path;
                    current.name = name.clone();
                    window.set_window_title(&format!("{name} — Noty"));
                    if !self.search_open {
                        let shown = Some(name.clone());
                        self.search
                            .update(cx, |search, cx| search.show_current(shown, window, cx));
                    }
                }
                self.refresh_notes(window, cx);
                window.push_notification(Notification::success(format!("Renamed to “{name}”")), cx);
                true
            }
            Err(error) => {
                if let Some(renaming) = self.renaming.as_mut() {
                    renaming.error = Some(error.to_string().into());
                }
                cx.notify();
                false
            }
        }
    }

    fn delete_action(&mut self, _: &DeleteNote, window: &mut Window, cx: &mut Context<Self>) {
        let Some((path, name)) = self.target_note(cx) else {
            return;
        };
        let workspace = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (workspace, path) = (workspace.clone(), path.clone());
            alert
                .title("Delete note?")
                .description(format!("“{name}” will be deleted permanently."))
                .ok_text("Delete")
                .ok_variant(ButtonVariant::Danger)
                .show_cancel(true)
                .on_ok(move |_, window, cx| {
                    workspace
                        .update(cx, |this, cx| this.delete_note(&path, window, cx))
                        .ok();
                    true
                })
        });
    }

    fn delete_note(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_saving(window, cx);
        let name = notes::note_name(path);
        if self.current.as_ref().is_some_and(|note| note.path == path) {
            self.autosave = None;
            self.current = None;
            self.save_state = SaveState::Saved;
            self.editor
                .update(cx, |editor, cx| editor.set_value("", window, cx));
            window.set_window_title("Noty");
            if !self.search_open {
                self.search
                    .update(cx, |search, cx| search.show_current(None, window, cx));
            }
        }
        match self.store.delete(path) {
            Ok(()) => {
                window.push_notification(Notification::success(format!("Deleted “{name}”")), cx)
            }
            Err(error) => window.push_notification(
                Notification::error(format!("Couldn't delete “{name}”: {error}")),
                cx,
            ),
        }
        self.refresh_notes(window, cx);
        if self.current.is_none() {
            self.focus_search(window, cx);
        }
    }

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
    fn select_theme(&mut self, action: &SelectTheme, window: &mut Window, cx: &mut Context<Self>) {
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

    fn follow_system_appearance(
        &mut self,
        _: &FollowSystemAppearance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_appearance(Appearance::System, window, cx);
    }

    fn use_light_appearance(
        &mut self,
        _: &UseLightAppearance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_appearance(Appearance::Light, window, cx);
    }

    fn use_dark_appearance(
        &mut self,
        _: &UseDarkAppearance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_appearance(Appearance::Dark, window, cx);
    }

    fn settings_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.persist_config();
        appearance::apply(&self.themes, &self.config, Some(window), cx);
        cx.notify();
    }

    fn toggle_line_numbers(
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

    fn toggle_close_button(
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

    fn show_shortcuts(&mut self, _: &ShowShortcuts, window: &mut Window, cx: &mut Context<Self>) {
        window.open_dialog(cx, |dialog, _, cx| {
            dialog
                .title("Keyboard Shortcuts")
                .w(px(460.))
                .child(shortcut_list(cx))
        });
    }

    fn show_about(&mut self, _: &ShowAbout, window: &mut Window, cx: &mut Context<Self>) {
        window.open_dialog(cx, |dialog, _, cx| dialog.w(px(340.)).child(about(cx)));
    }

    fn zoom(&mut self, points: f32, cx: &mut Context<Self>) {
        self.font_size = points.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        self.config.set_font_size(self.font_size);
        self.persist_config();
        cx.notify();
    }

    fn zoom_in(&mut self, _: &ZoomIn, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom(self.font_size + 1., cx);
    }

    fn zoom_out(&mut self, _: &ZoomOut, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom(self.font_size - 1., cx);
    }

    fn zoom_reset(&mut self, _: &ZoomReset, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom(Config::default_font_size(), cx);
    }

    fn persist_config(&self) {
        if let Err(error) = self.config.save() {
            eprintln!("noty: could not save the settings: {error}");
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

    fn render_title_bar(
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
            .size(px(24.))
            .rounded_full()
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
            .child(ui::icon(AppIcon::Close, px(12.), theme.muted_foreground));

        let unsaved_dot = div()
            .size(px(7.))
            .mr(px(4.))
            .rounded_full()
            .bg(theme.primary);
        let field = Input::new(&query)
            .small()
            .cleanable(true)
            .focus_bordered(false)
            .prefix(ui::icon(AppIcon::Search, px(14.), theme.muted_foreground))
            .when(dirty, |field| field.suffix(unsaved_dot));
        let search_field = div()
            .key_context(search_context)
            .flex_1()
            .min_w_0()
            .mx(px(if show_close { 14. } else { 24. }))
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
            .child(Styled::h(field, px(30.)));

        TitleBar::new().child(
            div()
                .size_full()
                .flex()
                .items_center()
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
                    .ghost()
                    .small()
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
            .child(
                div()
                    .mt(px(14.))
                    .flex()
                    .flex_col()
                    .gap(px(9.))
                    .child(ui::shortcut_hint_groups(
                        &[&[PRIMARY_KEY, "P"], &["Alt", "S"]],
                        "Search or create",
                        cx,
                    ))
                    .child(ui::shortcut_hint_groups(
                        &[&["↑", "↓"], &["Ctrl", "J", "K"]],
                        "Move through notes",
                        cx,
                    ))
                    .child(ui::shortcut_hint(&["↵"], "Open, or create if new", cx)),
            )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let (state_icon, state_label, state_color) = match self.save_state {
            SaveState::Saved => (AppIcon::Saved, "Saved", theme.muted_foreground),
            SaveState::Unsaved => (AppIcon::Unsaved, "Editing", theme.muted_foreground),
            SaveState::Conflict => (AppIcon::Warning, "Changed on disk", theme.warning),
            SaveState::Failed => (AppIcon::Warning, "Not saved", theme.danger),
        };
        let zoom = (self.font_size / Config::default_font_size() * 100.).round() as i32;
        let location = match &self.current {
            Some(note) => ui::display_path(&note.path),
            None => ui::display_path(self.store.dir()),
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
                        .child(ui::icon(state_icon, px(12.), state_color))
                        .child(state_label),
                )
            })
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
                .chain(keys.iter().map(|key| ui::kbd(*key, cx).into_any_element()))
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
        .child(ui::app_logo(px(72.)))
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

impl Render for Workspace {
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
            .id("workspace")
            .key_context("NotyWorkspace")
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
