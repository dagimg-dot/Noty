use std::collections::HashSet;

use gpui_kit::component::{Theme as KitTheme, ThemeMode, ThemeRegistry};
use gpui_kit::*;
use serde_json::{Map, Value, json};

use crate::config::{Appearance, Config};
use crate::theme::{Color, Semantic, Theme, ThemeLibrary, Variant};

const UI_FONTS: &[&str] = &[
    "SF Pro Text",
    "Inter",
    "Adwaita Sans",
    "Cantarell",
    "Noto Sans",
];
const EDITOR_FONTS: &[&str] = &[
    "Monaspace Argon NF",
    "Monaspace Argon",
    "JetBrains Mono",
    "Adwaita Mono",
    "DejaVu Sans Mono",
];

pub struct Fonts {
    pub ui: Option<SharedString>,
    pub editor: SharedString,
}

impl Global for Fonts {}

impl Fonts {
    pub fn pick(config: &Config, cx: &App) -> Self {
        let installed: HashSet<String> = cx.text_system().all_font_names().into_iter().collect();
        let first_installed = |families: &[&str]| {
            families
                .iter()
                .find(|family| installed.contains(**family))
                .map(|family| SharedString::from(family.to_string()))
        };
        let custom = config
            .editor_font()
            .filter(|family| installed.contains(family))
            .map(SharedString::from);
        Self {
            ui: first_installed(UI_FONTS),
            editor: custom
                .or_else(|| first_installed(EDITOR_FONTS))
                .unwrap_or_else(|| "monospace".into()),
        }
    }
}

pub fn register(library: &ThemeLibrary, cx: &mut App) {
    let themes: Vec<Value> = library
        .themes()
        .iter()
        .map(|theme| kit_theme(theme))
        .collect();
    let set = json!({ "name": "Noty", "themes": themes });
    if let Err(error) = ThemeRegistry::global_mut(cx).load_themes_from_str(&set.to_string()) {
        eprintln!("noty: could not load themes: {error:#}");
    }
}

/// Installs the configured light and dark themes and shows the one that
/// the appearance setting, or the system, asks for.
pub fn apply(library: &ThemeLibrary, config: &Config, window: Option<&mut Window>, cx: &mut App) {
    let registered = |variant: Variant, cx: &App| {
        let theme = library.pick(&config.theme_id(variant), variant);
        ThemeRegistry::global(cx).themes().get(theme.id()).cloned()
    };
    let light = registered(Variant::Light, cx);
    let dark = registered(Variant::Dark, cx);
    KitTheme::update(cx, |theme| {
        if let Some(light) = light {
            theme.light_theme = light;
        }
        if let Some(dark) = dark {
            theme.dark_theme = dark;
        }
    });
    match config.appearance() {
        Appearance::System => KitTheme::sync_system_appearance(window, cx),
        Appearance::Light => KitTheme::change(ThemeMode::Light, window, cx),
        Appearance::Dark => KitTheme::change(ThemeMode::Dark, window, cx),
    }
    let fonts = cx.global::<Fonts>();
    let (ui, editor) = (fonts.ui.clone(), fonts.editor.clone());
    KitTheme::update(cx, |theme| {
        if let Some(ui) = ui {
            theme.font_family = ui;
        }
        theme.font_size = px(14.);
        theme.mono_font_family = editor;
        theme.notification.placement = Anchor::BottomCenter;
        theme.notification.width = px(340.);
        theme.notification.margins.bottom = px(42.);
    });
}

fn kit_theme(theme: &Theme) -> Value {
    use Semantic::*;
    let color = |semantic| theme.color(semantic);
    let hex = |semantic| color(semantic).to_hex();
    let text_on = |semantic| color(semantic).readable_text(color(Background)).to_hex();
    let style = |semantic| json!({ "color": hex(semantic) });
    let dark = theme.is_dark();
    let accent = color(Accent);
    let (accent_hover, accent_active) = if dark {
        (accent.lighter(115), accent.darker(115))
    } else {
        (accent.darker(110), accent.darker(125))
    };
    let selection = match color(TextSelectionBackground) {
        opaque if opaque.a >= 1. => opaque.with_alpha(0.3),
        translucent => translucent,
    };

    let colors: Map<String, Value> = [
        ("background", hex(Background)),
        ("foreground", hex(TextPrimary)),
        ("border", hex(Border)),
        ("window.border", hex(MainWindowBorder)),
        ("muted.background", hex(SecondaryBackground)),
        ("muted.foreground", hex(TextMuted)),
        ("popover.background", hex(SecondaryBackground)),
        ("popover.foreground", hex(TextPrimary)),
        ("accent.background", hex(ListItemHoverBackground)),
        ("accent.foreground", hex(ListItemHoverForeground)),
        ("primary.background", accent.to_hex()),
        (
            "primary.foreground",
            accent.readable_text(color(AccentForeground)).to_hex(),
        ),
        ("primary.hover.background", accent_hover.to_hex()),
        ("primary.active.background", accent_active.to_hex()),
        ("secondary.background", hex(SecondaryBackground)),
        ("secondary.foreground", hex(TextPrimary)),
        ("secondary.hover.background", hex(ListItemHoverBackground)),
        (
            "secondary.active.background",
            hex(ListItemSelectionBackground),
        ),
        ("list.background", hex(SecondaryBackground)),
        ("list.even.background", hex(SecondaryBackground)),
        ("list.hover.background", hex(ListItemHoverBackground)),
        ("list.active.background", hex(ListItemSelectionBackground)),
        ("list.active.border", accent.to_hex()),
        ("title_bar.background", hex(Background)),
        ("title_bar.border", hex(Background)),
        ("status_bar.background", hex(StatusBarBackground)),
        ("status_bar.border", hex(Border)),
        ("input.border", hex(InputBorder)),
        ("ring", hex(InputBorderFocus)),
        ("caret", accent.to_hex()),
        ("selection.background", selection.to_hex()),
        (
            "overlay",
            Color::BLACK
                .with_alpha(if dark { 0.5 } else { 0.2 })
                .to_hex(),
        ),
        ("scrollbar.background", Color::TRANSPARENT.to_hex()),
        ("scrollbar.thumb.background", hex(ScrollBarBackground)),
        ("scrollbar.thumb.hover.background", hex(TextMuted)),
        ("link", hex(LinkDefault)),
        ("link.hover", hex(LinkDefault)),
        ("link.active", hex(LinkVisited)),
        ("success.background", hex(TextSuccess)),
        ("success.foreground", text_on(TextSuccess)),
        ("warning.background", hex(Yellow)),
        ("warning.foreground", text_on(Yellow)),
        ("danger.background", hex(TextDanger)),
        ("danger.foreground", text_on(TextDanger)),
        ("info.background", accent.to_hex()),
        ("info.foreground", text_on(Accent)),
        ("base.red", hex(Red)),
        ("base.green", hex(Green)),
        ("base.blue", hex(Blue)),
        ("base.yellow", hex(Yellow)),
        ("base.magenta", hex(Magenta)),
        ("base.cyan", hex(Cyan)),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), Value::String(value)))
    .collect();

    json!({
        "name": theme.id(),
        "mode": if dark { "dark" } else { "light" },
        "radius": 8,
        "radius.lg": 14,
        "shadow": true,
        "colors": colors,
        "highlight": {
            "editor.background": hex(Background),
            "editor.foreground": hex(TextPrimary),
            "editor.active_line.background":
                color(Foreground).with_alpha(if dark { 0.04 } else { 0.05 }).to_hex(),
            "editor.line_number": hex(TextPlaceholder),
            "editor.active_line_number": hex(TextMuted),
            "editor.gutter.background": hex(Background),
            "editor.invisible": hex(TextPlaceholder),
            "syntax": {
                "title": { "color": hex(TextPrimary), "font_weight": 700 },
                "punctuation.special": style(Accent),
                "punctuation.list_marker": style(Accent),
                "punctuation.delimiter": style(TextPlaceholder),
                "emphasis": { "color": hex(TextPrimary), "font_style": "italic" },
                "emphasis.strong": { "color": hex(TextPrimary), "font_weight": 700 },
                "link_text": style(LinkDefault),
                "link_uri": style(TextMuted),
                "text.literal": style(Green),
                "text.code.span": style(Orange),
                "string.escape": style(TextMuted),
            },
        },
    })
}
