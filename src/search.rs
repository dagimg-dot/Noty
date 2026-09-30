use std::ops::Range;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::menu::ContextMenuExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::actions::{
    Confirm, ConfirmCreate, DeleteNote, Dismiss, RenameNote, SelectNext, SelectPrev,
};
use crate::fuzzy::fuzzy_match;
use crate::notes::Note;
use crate::ui::{self, AppIcon};

const ROW_HEIGHT: f32 = 32.;
const VISIBLE_ROWS: usize = 6;
const LIST_PADDING: f32 = 4.;

pub enum SearchEvent {
    Open(PathBuf),
    Create(String),
    Dismissed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Note {
        name: SharedString,
        path: PathBuf,
        modified: SystemTime,
        byte_positions: Vec<usize>,
    },
    Create(SharedString),
}

/// The rows to list for `query` and the row that starts selected.
///
/// Like Noty's search box, the highlighted row is what Enter does: an
/// exact name opens that note, any other name offers to create it first,
/// and fuzzy matches follow for picking with the arrow keys.
pub fn build_rows(notes: &[Note], query: &str, current: Option<&str>) -> (Vec<Row>, usize) {
    let query = query.trim();
    let browsing = query.is_empty() || current.is_some_and(|name| name.eq_ignore_ascii_case(query));
    if browsing {
        let rows = notes
            .iter()
            .map(|note| note_row(note, Vec::new()))
            .collect();
        let selected = current
            .and_then(|name| notes.iter().position(|note| note.name == name))
            .unwrap_or(0);
        return (rows, selected);
    }

    let mut matches: Vec<(i64, usize, Vec<usize>)> = notes
        .iter()
        .enumerate()
        .filter_map(|(ix, note)| {
            fuzzy_match(query, &note.name).map(|m| (m.score, ix, m.byte_positions))
        })
        .collect();
    matches.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

    let exact = notes
        .iter()
        .any(|note| note.name.eq_ignore_ascii_case(query));
    let create = (!exact).then(|| Row::Create(query.to_owned().into()));
    let rows = create
        .into_iter()
        .chain(
            matches
                .into_iter()
                .map(|(_, ix, positions)| note_row(&notes[ix], positions)),
        )
        .collect();
    (rows, 0)
}

fn note_row(note: &Note, byte_positions: Vec<usize>) -> Row {
    Row::Note {
        name: note.name.clone().into(),
        path: note.path.clone(),
        modified: note.modified,
        byte_positions,
    }
}

pub struct Search {
    query: Entity<InputState>,
    notes: Vec<Note>,
    rows: Vec<Row>,
    selected: usize,
    navigating: bool,
    current: Option<SharedString>,
    scroll: UniformListScrollHandle,
    generation: usize,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<SearchEvent> for Search {}

impl Search {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Search or create note…"));
        let subscriptions = vec![cx.subscribe(&query, |this, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                this.refilter(cx);
            }
        })];
        Self {
            query,
            notes: Vec::new(),
            rows: Vec::new(),
            selected: 0,
            navigating: false,
            current: None,
            scroll: UniformListScrollHandle::new(),
            generation: 0,
            _subscriptions: subscriptions,
        }
    }

    pub fn query(&self) -> &Entity<InputState> {
        &self.query
    }

    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.query.update(cx, |query, cx| query.focus(window, cx));
    }

    pub fn show_current(
        &mut self,
        name: Option<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.current = name.clone();
        self.query.update(cx, |query, cx| {
            query.set_value(name.unwrap_or_default(), window, cx)
        });
    }

    pub fn begin(&mut self, notes: Vec<Note>, window: &mut Window, cx: &mut Context<Self>) {
        self.notes = notes;
        self.generation += 1;
        self.query
            .update(cx, |query, cx| query.select_all(window, cx));
        self.refilter(cx);
    }

    pub fn set_notes(&mut self, notes: Vec<Note>, cx: &mut Context<Self>) {
        self.notes = notes;
        self.refilter(cx);
    }

    /// True once the arrow keys moved through the list, so Delete acts on
    /// the highlighted note instead of on the typed text.
    pub fn is_navigating(&self) -> bool {
        self.navigating
    }

    pub fn selected_note(&self) -> Option<(PathBuf, SharedString)> {
        match self.rows.get(self.selected)? {
            Row::Note { path, name, .. } => Some((path.clone(), name.clone())),
            Row::Create(_) => None,
        }
    }

    fn refilter(&mut self, cx: &mut Context<Self>) {
        let query = self.query.read(cx).value();
        let (rows, selected) = build_rows(&self.notes, &query, self.current.as_deref());
        self.rows = rows;
        self.navigating = false;
        self.select(selected, cx);
    }

    fn select(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.selected = ix;
        self.scroll.scroll_to_item(ix, ScrollStrategy::Top);
        cx.notify();
    }

    pub fn select_next(&mut self, _: &SelectNext, _: &mut Window, cx: &mut Context<Self>) {
        if !self.rows.is_empty() {
            self.navigating = true;
            self.select((self.selected + 1) % self.rows.len(), cx);
        }
    }

    pub fn select_prev(&mut self, _: &SelectPrev, _: &mut Window, cx: &mut Context<Self>) {
        if !self.rows.is_empty() {
            self.navigating = true;
            self.select((self.selected + self.rows.len() - 1) % self.rows.len(), cx);
        }
    }

    pub fn confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        match self.rows.get(self.selected) {
            Some(Row::Note { path, .. }) => cx.emit(SearchEvent::Open(path.clone())),
            Some(Row::Create(name)) => cx.emit(SearchEvent::Create(name.to_string())),
            None => {}
        }
    }

    pub fn confirm_create(&mut self, _: &ConfirmCreate, _: &mut Window, cx: &mut Context<Self>) {
        let query = self.query.read(cx).value().trim().to_owned();
        let existing = self
            .notes
            .iter()
            .find(|note| note.name.eq_ignore_ascii_case(&query));
        if let Some(note) = existing {
            cx.emit(SearchEvent::Open(note.path.clone()));
        } else if !query.is_empty() {
            cx.emit(SearchEvent::Create(query));
        }
    }

    pub fn dismiss(&mut self, _: &Dismiss, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(SearchEvent::Dismissed);
    }

    fn render_rows(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        range.map(|ix| self.render_row(ix, cx)).collect()
    }

    fn render_row(&self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let selected = ix == self.selected;
        let accent = theme.primary;
        let muted = theme.muted_foreground;

        let row = div()
            .size_full()
            .px(px(10.))
            .flex()
            .items_center()
            .gap(px(9.))
            .rounded(px(6.))
            .when(selected, |row| row.bg(theme.list_active))
            .when(!selected, |row| {
                row.hover(|style| style.bg(theme.list_hover))
            });

        let row = match &self.rows[ix] {
            Row::Note {
                name,
                modified,
                byte_positions,
                ..
            } => {
                let is_current = self.current.as_ref() == Some(name);
                let highlight = HighlightStyle {
                    color: Some(accent),
                    font_weight: Some(FontWeight::SEMIBOLD),
                    ..Default::default()
                };
                let highlights: Vec<(Range<usize>, HighlightStyle)> = byte_positions
                    .iter()
                    .map(|&start| {
                        let len = name[start..].chars().next().map_or(1, char::len_utf8);
                        (start..start + len, highlight)
                    })
                    .collect();
                let icon_color = if selected || is_current {
                    accent
                } else {
                    muted
                };
                row.child(ui::icon(AppIcon::Note, px(14.), icon_color))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(14.))
                            .text_color(theme.foreground)
                            .child(StyledText::new(name.clone()).with_highlights(highlights)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(muted)
                            .child(ui::relative_time(*modified)),
                    )
            }
            Row::Create(name) => row
                .child(ui::icon(AppIcon::Create, px(14.), accent))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .text_size(px(14.))
                        .text_color(theme.foreground)
                        .child("Create “")
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .truncate()
                                .child(name.clone()),
                        )
                        .child("”"),
                )
                .when(selected, |row| row.child(ui::kbd("↵", cx))),
        };

        let is_note = matches!(self.rows[ix], Row::Note { .. });
        let item = div()
            .id(ix)
            .w_full()
            .h(px(ROW_HEIGHT))
            .px(px(6.))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, window, cx| {
                this.selected = ix;
                this.confirm(&Confirm, window, cx);
            }))
            .child(row);
        if !is_note {
            return item.into_any_element();
        }
        item.on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, _, _, cx| this.select(ix, cx)),
        )
        .context_menu(|menu, _, _| {
            menu.menu("Rename…", Box::new(RenameNote))
                .menu("Delete…", Box::new(DeleteNote))
        })
        .into_any_element()
    }
}

impl Render for Search {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let row_count = self.rows.len();
        let list_height = row_count.clamp(1, VISIBLE_ROWS) as f32 * ROW_HEIGHT + 2. * LIST_PADDING;

        let content = if row_count == 0 {
            div()
                .h(px(list_height))
                .px(px(18.))
                .flex()
                .items_center()
                .text_size(px(13.))
                .text_color(theme.muted_foreground)
                .child("No notes yet. Type a name and press Enter to create one.")
                .into_any_element()
        } else {
            uniform_list(
                "search-rows",
                row_count,
                cx.processor(|this, range, _, cx| this.render_rows(range, cx)),
            )
            .track_scroll(&self.scroll)
            .w_full()
            .h(px(list_height))
            .py(px(LIST_PADDING))
            .into_any_element()
        };

        div()
            .w_full()
            .flex_none()
            .overflow_hidden()
            .bg(theme.colors.list)
            .border_b_1()
            .border_color(theme.border)
            .child(content)
            .with_animation(
                ("search-reveal", self.generation),
                Animation::new(Duration::from_millis(200)).with_easing(ease_out_quint()),
                move |list, delta| list.h(px((list_height + 1.) * delta)),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::SystemTime;

    use super::{Row, build_rows};
    use crate::notes::Note;

    fn notes(names: &[&str]) -> Vec<Note> {
        names
            .iter()
            .map(|name| Note {
                name: name.to_string(),
                path: Path::new("/notes").join(name),
                modified: SystemTime::UNIX_EPOCH,
            })
            .collect()
    }

    fn labels(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::Note { name, .. } => name.to_string(),
                Row::Create(name) => format!("+{name}"),
            })
            .collect()
    }

    #[test]
    fn an_empty_query_or_the_open_note_lists_everything() {
        let all = notes(&["cool", "hello", "there"]);
        let (rows, selected) = build_rows(&all, "", None);
        assert_eq!(labels(&rows), ["cool", "hello", "there"]);
        assert_eq!(selected, 0);

        let (rows, selected) = build_rows(&all, "hello", Some("hello"));
        assert_eq!(labels(&rows), ["cool", "hello", "there"]);
        assert_eq!(selected, 1);
    }

    #[test]
    fn a_new_name_offers_to_create_it_first() {
        let all = notes(&["Groceries", "Playground"]);
        let (rows, selected) = build_rows(&all, "gro", None);
        assert_eq!(labels(&rows), ["+gro", "Groceries", "Playground"]);
        assert_eq!(selected, 0);
    }

    #[test]
    fn an_exact_name_opens_that_note() {
        let all = notes(&["Groceries", "Groceries list"]);
        let (rows, selected) = build_rows(&all, "groceries", None);
        assert_eq!(labels(&rows), ["Groceries", "Groceries list"]);
        assert_eq!(selected, 0);
    }
}
