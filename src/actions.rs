use gpui_kit as gpui;
use gpui_kit::*;

gpui_kit::actions!(
    noty,
    [
        FocusSearch,
        SelectNext,
        SelectPrev,
        Confirm,
        ConfirmCreate,
        Dismiss,
        SaveNote,
        ReloadNote,
        RenameNote,
        DeleteNote,
        ZoomIn,
        ZoomOut,
        ZoomReset,
        FollowSystemAppearance,
        UseLightAppearance,
        UseDarkAppearance,
        ToggleLineNumbers,
        Quit,
    ]
);

#[derive(Clone, Debug, PartialEq, gpui::Action)]
#[action(namespace = noty, no_json)]
pub struct SelectTheme {
    pub id: SharedString,
}

const SEARCH_FIELD: Option<&str> = Some("NotySearch > Input");
const EDITOR: Option<&str> = Some("NotyEditor > Input");
const NOTE_LIST: Option<&str> = Some("navigating > Input");

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("secondary-p", FocusSearch, None),
        KeyBinding::new("alt-s", FocusSearch, None),
        KeyBinding::new("escape", FocusSearch, EDITOR),
        KeyBinding::new("down", SelectNext, SEARCH_FIELD),
        KeyBinding::new("up", SelectPrev, SEARCH_FIELD),
        KeyBinding::new("ctrl-j", SelectNext, SEARCH_FIELD),
        KeyBinding::new("ctrl-k", SelectPrev, SEARCH_FIELD),
        KeyBinding::new("enter", Confirm, SEARCH_FIELD),
        KeyBinding::new("shift-enter", ConfirmCreate, SEARCH_FIELD),
        KeyBinding::new("escape", Dismiss, SEARCH_FIELD),
        KeyBinding::new("f2", RenameNote, None),
        KeyBinding::new("ctrl-r", RenameNote, SEARCH_FIELD),
        KeyBinding::new("delete", DeleteNote, NOTE_LIST),
        KeyBinding::new("shift-delete", DeleteNote, SEARCH_FIELD),
        KeyBinding::new("secondary-s", SaveNote, None),
        KeyBinding::new("secondary-shift-r", ReloadNote, None),
        KeyBinding::new("secondary-=", ZoomIn, None),
        KeyBinding::new("secondary-+", ZoomIn, None),
        KeyBinding::new("secondary--", ZoomOut, None),
        KeyBinding::new("secondary-0", ZoomReset, None),
        KeyBinding::new("alt-l", ToggleLineNumbers, None),
        KeyBinding::new("secondary-q", Quit, None),
    ]);
}
