use std::time::Duration;

use gpui_kit::component::WindowExt as _;
use gpui_kit::*;

use super::{MainWindow, SaveState};
use crate::format;
use crate::notes::saver::{FinishedSave, SaveJob};
use crate::notes::{self, SaveOutcome};
use crate::ui::keymap::{ReloadNote, SaveNote};
use crate::ui::widgets::{self, Tone};

const AUTOSAVE_DELAY: Duration = Duration::from_millis(800);
const WORD_COUNT_DELAY: Duration = Duration::from_millis(250);

impl MainWindow {
    pub(super) fn on_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(current) = &self.current else {
            return;
        };
        self.recount = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(WORD_COUNT_DELAY).await;
            this.update(cx, |this, cx| {
                this.word_count = format::word_count(this.editor.read(cx).text().chunks());
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
    pub(super) fn save(&mut self, force: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.autosave = None;
        self.finish_saving(window, cx);
        if let Some(job) = self.save_job(force, cx) {
            self.apply_save(job.run(), window, cx);
        }
    }

    pub(super) fn save_in_background(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn finish_saving(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
                    widgets::notification(
                        Tone::Warning,
                        format!(
                            "“{}” changed outside Noty. Save keeps your version, \
                         Reload loads theirs.",
                            current.name
                        ),
                    )
                    .autohide(false),
                    cx,
                );
            }
            Err(error) => {
                self.save_state = SaveState::Failed;
                window.push_notification(
                    widgets::notification(
                        Tone::Error,
                        format!("Couldn't save “{}”: {error}", current.name),
                    ),
                    cx,
                );
            }
        }
        cx.notify();
    }

    pub(super) fn check_disk(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
                widgets::notification(
                    Tone::Info,
                    format!("Reloaded “{name}”, it changed outside Noty."),
                ),
                cx,
            );
        } else if self.save_state != SaveState::Conflict {
            self.save(false, window, cx);
        }
    }

    pub(super) fn save_action(
        &mut self,
        _: &SaveNote,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let force = matches!(self.save_state, SaveState::Conflict | SaveState::Failed);
        self.save(force, window, cx);
        if force && self.save_state == SaveState::Saved {
            window.push_notification(
                widgets::notification(Tone::Success, "Saved your version."),
                cx,
            );
        }
    }

    pub(super) fn reload_action(
        &mut self,
        _: &ReloadNote,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.finish_saving(window, cx);
        if let Some(path) = self.current.as_ref().map(|note| note.path.clone()) {
            self.load(&path, false, window, cx);
        }
    }
}
