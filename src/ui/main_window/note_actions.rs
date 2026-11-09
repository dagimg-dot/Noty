use std::path::Path;
use std::path::PathBuf;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::button::ButtonVariants as _;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputState;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{MainWindow, OpenNote, Renaming, SaveState};
use crate::format;
use crate::notes;
use crate::ui::keymap::DeleteNote;
use crate::ui::keymap::RenameNote;
use crate::ui::widgets;
use crate::ui::widgets::Tone;

impl MainWindow {
    /// Opens `path`, saving the current note first. Edits that could not
    /// be saved are never dropped: the switch is refused until they are
    /// saved or reloaded.
    pub(super) fn open(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        if self.current.as_ref().is_some_and(|note| note.path == path) {
            self.focus_editor(window, cx);
            return;
        }
        self.save(false, window, cx);
        if self.current.is_some()
            && matches!(self.save_state, SaveState::Conflict | SaveState::Failed)
        {
            window.push_notification(
                widgets::notification(
                    Tone::Warning,
                    "This note has edits that couldn't be saved. Save or reload it first.",
                ),
                cx,
            );
            return;
        }
        self.load(path, true, window, cx);
    }

    pub(super) fn load(
        &mut self,
        path: &Path,
        focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (text, modified) = match notes::read_note(path) {
            Ok(loaded) => loaded,
            Err(error) => {
                window.push_notification(
                    widgets::notification(
                        Tone::Error,
                        format!("Couldn't open {}: {error}", format::display_path(path)),
                    ),
                    cx,
                );
                return;
            }
        };
        let name = SharedString::from(notes::note_name(path));
        self.autosave = None;
        self.recount = None;
        self.word_count = format::word_count([text.as_str()]);
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

    pub(super) fn focus_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn rename_action(
        &mut self,
        _: &RenameNote,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((path, name)) = self.target_note(cx) else {
            return;
        };
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        self.renaming = Some(Renaming {
            path,
            input: input.clone(),
            error: None,
        });

        let main_window = cx.entity().downgrade();
        let field = input.clone();
        window.open_dialog(cx, move |dialog, _, cx| {
            let error = main_window
                .upgrade()
                .and_then(|main_window| main_window.read(cx).renaming.as_ref()?.error.clone());
            let (submit, confirm) = (main_window.clone(), main_window.clone());
            dialog
                .title("Rename note")
                .w(px(360.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(Input::new(&field).focus_bordered(false))
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
                window.push_notification(
                    widgets::notification(Tone::Success, format!("Renamed to “{name}”")),
                    cx,
                );
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

    pub(super) fn delete_action(
        &mut self,
        _: &DeleteNote,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((path, name)) = self.target_note(cx) else {
            return;
        };
        let main_window = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (main_window, path) = (main_window.clone(), path.clone());
            alert
                .title("Delete note?")
                .description(format!("“{name}” will be deleted permanently."))
                .ok_text("Delete")
                .ok_variant(ButtonVariant::Danger)
                .show_cancel(true)
                .on_ok(move |_, window, cx| {
                    main_window
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
            Ok(()) => window.push_notification(
                widgets::notification(Tone::Success, format!("Deleted “{name}”")),
                cx,
            ),
            Err(error) => window.push_notification(
                widgets::notification(Tone::Error, format!("Couldn't delete “{name}”: {error}")),
                cx,
            ),
        }
        self.refresh_notes(window, cx);
        if self.current.is_none() {
            self.focus_search(window, cx);
        }
    }
}
