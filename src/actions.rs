use gpui_kit::*;

gpui_kit::actions!(
    noty,
    [SaveNote, ReloadNote, ZoomIn, ZoomOut, ZoomReset, Quit]
);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("secondary-s", SaveNote, None),
        KeyBinding::new("secondary-shift-r", ReloadNote, None),
        KeyBinding::new("secondary-=", ZoomIn, None),
        KeyBinding::new("secondary-+", ZoomIn, None),
        KeyBinding::new("secondary--", ZoomOut, None),
        KeyBinding::new("secondary-0", ZoomReset, None),
        KeyBinding::new("secondary-q", Quit, None),
    ]);
}
