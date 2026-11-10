use gpui_kit::*;

use super::MainWindow;
use crate::line_edit::{self, Direction, LineEdit};
use crate::ui::keymap::{DuplicateLineDown, DuplicateLineUp, MoveLineDown, MoveLineUp};

impl MainWindow {
    pub(super) fn move_line_up(
        &mut self,
        _: &MoveLineUp,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit_lines(window, cx, |text, selection| {
            line_edit::move_lines(text, selection, Direction::Up)
        });
    }

    pub(super) fn move_line_down(
        &mut self,
        _: &MoveLineDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit_lines(window, cx, |text, selection| {
            line_edit::move_lines(text, selection, Direction::Down)
        });
    }

    pub(super) fn duplicate_line_up(
        &mut self,
        _: &DuplicateLineUp,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit_lines(window, cx, |text, selection| {
            Some(line_edit::duplicate_lines(text, selection, Direction::Up))
        });
    }

    pub(super) fn duplicate_line_down(
        &mut self,
        _: &DuplicateLineDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit_lines(window, cx, |text, selection| {
            Some(line_edit::duplicate_lines(text, selection, Direction::Down))
        });
    }

    fn edit_lines(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        edit: impl FnOnce(&str, std::ops::Range<usize>) -> Option<LineEdit>,
    ) {
        if self.current.is_none() {
            return;
        }
        self.editor.update(cx, |editor, cx| {
            let text = editor.text().to_string();
            let Some(edit) = edit(&text, editor.selected_range()) else {
                return;
            };
            editor.set_selected_range(edit.range, cx);
            editor.replace(edit.text, window, cx);
            editor.set_selected_range(edit.selection, cx);
        });
    }
}
