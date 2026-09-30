use gpui_kit::*;

gpui_kit::actions!(noty, [Quit]);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("secondary-q", Quit, None)]);
}
