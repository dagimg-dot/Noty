use std::io;
use std::mem;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::SystemTime;

use gpui_kit::component::input::Rope;

use crate::notes::{self, SaveOutcome};

pub struct SaveJob {
    pub path: PathBuf,
    pub content: Rope,
    pub expected_modified: Option<SystemTime>,
}

pub struct FinishedSave {
    pub path: PathBuf,
    pub content: Rope,
    pub outcome: io::Result<SaveOutcome>,
}

impl SaveJob {
    pub fn run(self) -> FinishedSave {
        let outcome = notes::save_note(
            &self.path,
            &String::from(&self.content),
            self.expected_modified,
        );
        FinishedSave {
            path: self.path,
            content: self.content,
            outcome,
        }
    }
}

#[derive(Default)]
enum Slot {
    #[default]
    Idle,
    Running,
    Finished(FinishedSave),
}

#[derive(Default)]
struct Shared {
    slot: Mutex<Slot>,
    finished: Condvar,
}

#[derive(Clone, Default)]
pub struct Saver(Arc<Shared>);

impl Saver {
    /// Reserves the saver for a job that the caller then passes to
    /// [`Saver::run`], usually on a background thread. Fails while an earlier
    /// save is running or its outcome has not been taken, so saves never
    /// overlap and [`Saver::wait`] can settle the file for the UI thread.
    pub fn try_begin(&self) -> bool {
        let mut slot = self.slot();
        let idle = matches!(*slot, Slot::Idle);
        if idle {
            *slot = Slot::Running;
        }
        idle
    }

    pub fn run(&self, job: SaveJob) {
        let finished = job.run();
        *self.slot() = Slot::Finished(finished);
        self.0.finished.notify_all();
    }

    pub fn take(&self) -> Option<FinishedSave> {
        take_finished(&mut self.slot())
    }

    pub fn wait(&self) -> Option<FinishedSave> {
        let mut slot = self
            .0
            .finished
            .wait_while(self.slot(), |slot| matches!(slot, Slot::Running))
            .unwrap_or_else(PoisonError::into_inner);
        take_finished(&mut slot)
    }

    fn slot(&self) -> MutexGuard<'_, Slot> {
        self.0.slot.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn take_finished(slot: &mut Slot) -> Option<FinishedSave> {
    match mem::take(slot) {
        Slot::Finished(finished) => Some(finished),
        other => {
            *slot = other;
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::thread;
    use std::time::Duration;

    use gpui_kit::component::input::Rope;

    use super::{SaveJob, Saver};
    use crate::notes::SaveOutcome;

    fn job(dir: &tempfile::TempDir, text: &str) -> SaveJob {
        SaveJob {
            path: dir.path().join("note"),
            content: Rope::from_str(text),
            expected_modified: None,
        }
    }

    #[test]
    fn a_finished_save_is_taken_once() {
        let dir = tempfile::tempdir().unwrap();
        let saver = Saver::default();
        assert!(saver.try_begin());
        saver.run(job(&dir, "hello"));
        let finished = saver.take().unwrap();
        assert!(matches!(finished.outcome, Ok(SaveOutcome::Saved(_))));
        assert_eq!(
            fs::read_to_string(dir.path().join("note")).unwrap(),
            "hello"
        );
        assert!(saver.take().is_none());
    }

    #[test]
    fn only_one_save_runs_until_its_outcome_is_taken() {
        let dir = tempfile::tempdir().unwrap();
        let saver = Saver::default();
        assert!(saver.try_begin());
        assert!(!saver.try_begin());
        saver.run(job(&dir, "first"));
        assert!(!saver.try_begin());
        saver.take();
        assert!(saver.try_begin());
    }

    #[test]
    fn waiting_returns_a_save_finishing_on_another_thread() {
        let dir = tempfile::tempdir().unwrap();
        let saver = Saver::default();
        assert!(saver.try_begin());
        let background = saver.clone();
        let job = job(&dir, "from the background");
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            background.run(job);
        });
        let finished = saver.wait().unwrap();
        handle.join().unwrap();
        assert!(matches!(finished.outcome, Ok(SaveOutcome::Saved(_))));
        assert_eq!(
            fs::read_to_string(dir.path().join("note")).unwrap(),
            "from the background"
        );
    }

    #[test]
    fn waiting_without_a_save_returns_at_once() {
        assert!(Saver::default().wait().is_none());
    }
}
