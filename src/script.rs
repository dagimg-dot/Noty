use std::time::Duration;

use gpui_kit::*;

/// Replays the `NOTY_SCRIPT` environment variable after launch: steps are
/// `wait:MS`, `type:TEXT` (`_` types a space), `click:X,Y` or `rclick:X,Y`
/// in window coordinates, or a keystroke such as `ctrl-p`.
pub fn run_from_env(window: AnyWindowHandle, cx: &mut App) {
    let Ok(script) = std::env::var("NOTY_SCRIPT") else {
        return;
    };
    cx.spawn(async move |cx| {
        for step in script.split_whitespace() {
            if let Some(millis) = step.strip_prefix("wait:") {
                let millis = millis.parse().unwrap_or(200);
                cx.background_executor()
                    .timer(Duration::from_millis(millis))
                    .await;
                continue;
            }
            for event in events_for(step) {
                cx.update_window(window, |_, window, cx| match event {
                    Step::Key(keystroke) => {
                        window.dispatch_keystroke(keystroke, cx);
                    }
                    Step::Mouse(input) => {
                        window.dispatch_event(input, cx);
                    }
                })
                .ok();
                cx.background_executor()
                    .timer(Duration::from_millis(35))
                    .await;
            }
        }
    })
    .detach();
}

enum Step {
    Key(Keystroke),
    Mouse(PlatformInput),
}

fn events_for(step: &str) -> Vec<Step> {
    if let Some(text) = step.strip_prefix("type:") {
        return text
            .chars()
            .filter_map(|c| {
                let key = if c == '_' {
                    "space".to_owned()
                } else {
                    c.to_string()
                };
                Keystroke::parse(&key).ok().map(Step::Key)
            })
            .collect();
    }
    for (prefix, button) in [
        ("click:", MouseButton::Left),
        ("rclick:", MouseButton::Right),
    ] {
        if let Some((x, y)) = step.strip_prefix(prefix).and_then(|xy| xy.split_once(',')) {
            let (Ok(x), Ok(y)) = (x.parse::<f32>(), y.parse::<f32>()) else {
                return Vec::new();
            };
            return click(button, point(px(x), px(y)));
        }
    }
    Keystroke::parse(step).map(Step::Key).into_iter().collect()
}

fn click(button: MouseButton, position: Point<Pixels>) -> Vec<Step> {
    let modifiers = Modifiers::default();
    vec![
        Step::Mouse(PlatformInput::MouseMove(MouseMoveEvent {
            position,
            pressed_button: None,
            modifiers,
        })),
        Step::Mouse(PlatformInput::MouseDown(MouseDownEvent {
            button,
            position,
            modifiers,
            click_count: 1,
            first_mouse: false,
        })),
        Step::Mouse(PlatformInput::MouseUp(MouseUpEvent {
            button,
            position,
            modifiers,
            click_count: 1,
        })),
    ]
}
