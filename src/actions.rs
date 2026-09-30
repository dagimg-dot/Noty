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
        ZoomIn,
        ZoomOut,
        ZoomReset,
        Quit,
    ]
);

const SEARCH_FIELD: Option<&str> = Some("NotySearch > Input");
const EDITOR: Option<&str> = Some("NotyEditor > Input");

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
        KeyBinding::new("secondary-s", SaveNote, None),
        KeyBinding::new("secondary-shift-r", ReloadNote, None),
        KeyBinding::new("secondary-=", ZoomIn, None),
        KeyBinding::new("secondary-+", ZoomIn, None),
        KeyBinding::new("secondary--", ZoomOut, None),
        KeyBinding::new("secondary-0", ZoomReset, None),
        KeyBinding::new("secondary-q", Quit, None),
    ]);
}
