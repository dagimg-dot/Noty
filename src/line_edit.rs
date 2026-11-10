use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LineEdit {
    pub range: Range<usize>,
    pub text: String,
    pub selection: Range<usize>,
}

pub fn move_lines(text: &str, selection: Range<usize>, direction: Direction) -> Option<LineEdit> {
    let block = block_range(text, &selection);
    let lines = &text[block.clone()];
    match direction {
        Direction::Up => {
            let above = line_start(text, block.start.checked_sub(1)?);
            let neighbour = &text[above..block.start - 1];
            Some(LineEdit {
                range: above..block.end,
                text: format!("{lines}\n{neighbour}"),
                selection: shift_back(&selection, block.start - above),
            })
        }
        Direction::Down => {
            let below = block.end + 1;
            if below > text.len() {
                return None;
            }
            let end = line_end(text, below);
            let neighbour = &text[below..end];
            Some(LineEdit {
                range: block.start..end,
                text: format!("{neighbour}\n{lines}"),
                selection: shift_forward(&selection, end - block.end),
            })
        }
    }
}

pub fn duplicate_lines(text: &str, selection: Range<usize>, direction: Direction) -> LineEdit {
    let block = block_range(text, &selection);
    let lines = &text[block.clone()];
    match direction {
        Direction::Up => LineEdit {
            range: block.start..block.start,
            text: format!("{lines}\n"),
            selection,
        },
        Direction::Down => LineEdit {
            range: block.end..block.end,
            text: format!("\n{lines}"),
            selection: shift_forward(&selection, lines.len() + 1),
        },
    }
}

fn block_range(text: &str, selection: &Range<usize>) -> Range<usize> {
    let start = line_start(text, selection.start);
    let last = if selection.end > selection.start && text[..selection.end].ends_with('\n') {
        selection.end - 1
    } else {
        selection.end
    };
    start..line_end(text, last.max(start))
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map_or(0, |ix| ix + 1)
}

fn line_end(text: &str, offset: usize) -> usize {
    text[offset..]
        .find('\n')
        .map_or(text.len(), |ix| offset + ix)
}

fn shift_back(range: &Range<usize>, by: usize) -> Range<usize> {
    range.start - by..range.end - by
}

fn shift_forward(range: &Range<usize>, by: usize) -> Range<usize> {
    range.start + by..range.end + by
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(text: &str, edit: &LineEdit) -> String {
        let mut out = text.to_owned();
        out.replace_range(edit.range.clone(), &edit.text);
        out
    }

    fn moved(text: &str, selection: Range<usize>, direction: Direction) -> (String, Range<usize>) {
        let edit = move_lines(text, selection, direction).expect("line should move");
        (apply(text, &edit), edit.selection)
    }

    fn duplicated(
        text: &str,
        selection: Range<usize>,
        direction: Direction,
    ) -> (String, Range<usize>) {
        let edit = duplicate_lines(text, selection, direction);
        (apply(text, &edit), edit.selection)
    }

    #[test]
    fn moves_the_cursor_line_up_and_keeps_the_column() {
        assert_eq!(
            moved("one\ntwo\nthree", 5..5, Direction::Up),
            ("two\none\nthree".into(), 1..1)
        );
    }

    #[test]
    fn moves_the_cursor_line_down_and_keeps_the_column() {
        assert_eq!(
            moved("one\ntwo\nthree", 5..5, Direction::Down),
            ("one\nthree\ntwo".into(), 11..11)
        );
    }

    #[test]
    fn leaves_the_first_line_when_moving_up() {
        assert_eq!(move_lines("one\ntwo", 2..2, Direction::Up), None);
    }

    #[test]
    fn leaves_the_last_line_when_moving_down() {
        assert_eq!(move_lines("one\ntwo", 5..5, Direction::Down), None);
    }

    #[test]
    fn moves_the_last_line_without_a_trailing_newline_up() {
        assert_eq!(
            moved("one\ntwo", 7..7, Direction::Up),
            ("two\none".into(), 3..3)
        );
    }

    #[test]
    fn moves_a_line_below_the_empty_line_after_a_trailing_newline() {
        assert_eq!(
            moved("one\ntwo\n", 4..4, Direction::Down),
            ("one\n\ntwo".into(), 5..5)
        );
    }

    #[test]
    fn moves_every_line_the_selection_touches() {
        let text = "a\nbb\ncc\nd";
        assert_eq!(
            moved(text, 3..6, Direction::Up),
            ("bb\ncc\na\nd".into(), 1..4)
        );
        assert_eq!(
            moved(text, 3..6, Direction::Down),
            ("a\nd\nbb\ncc".into(), 5..8)
        );
    }

    #[test]
    fn ignores_the_line_a_selection_ends_at_the_start_of() {
        assert_eq!(
            moved("a\nb\nc", 0..2, Direction::Down),
            ("b\na\nc".into(), 2..4)
        );
    }

    #[test]
    fn duplicates_the_cursor_line_down_onto_the_copy() {
        assert_eq!(
            duplicated("one\ntwo", 1..1, Direction::Down),
            ("one\none\ntwo".into(), 5..5)
        );
    }

    #[test]
    fn duplicates_the_cursor_line_up_and_stays_on_the_upper_copy() {
        assert_eq!(
            duplicated("one\ntwo", 5..5, Direction::Up),
            ("one\ntwo\ntwo".into(), 5..5)
        );
    }

    #[test]
    fn duplicates_the_last_line_without_a_trailing_newline() {
        assert_eq!(
            duplicated("one\ntwo", 6..6, Direction::Down),
            ("one\ntwo\ntwo".into(), 10..10)
        );
    }

    #[test]
    fn duplicates_every_line_the_selection_touches() {
        assert_eq!(
            duplicated("a\nbb\ncc", 3..6, Direction::Down),
            ("a\nbb\ncc\nbb\ncc".into(), 9..12)
        );
    }

    #[test]
    fn duplicates_an_empty_note() {
        assert_eq!(duplicated("", 0..0, Direction::Down), ("\n".into(), 1..1));
    }
}
