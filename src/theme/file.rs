use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context as _, Result, bail};

use super::color::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Variant {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Semantic {
    Accent,
    AccentForeground,
    Background,
    Foreground,
    SecondaryBackground,
    Border,
    ShortcutBorder,
    MainWindowBorder,
    StatusBarBackground,
    SettingsWindowBorder,
    Blue,
    Green,
    Magenta,
    Orange,
    Red,
    Yellow,
    Cyan,
    Purple,
    TextPrimary,
    TextMuted,
    TextDanger,
    TextSuccess,
    TextPlaceholder,
    LinkDefault,
    LinkVisited,
    TextSelectionBackground,
    TextSelectionForeground,
    InputBackground,
    InputBorder,
    InputBorderFocus,
    InputBorderError,
    ButtonPrimaryBackground,
    ButtonPrimaryForeground,
    ButtonPrimaryHoverBackground,
    ButtonPrimaryHoverForeground,
    ButtonPrimaryFocusOutline,
    ListItemHoverBackground,
    ListItemHoverForeground,
    ListItemSecondaryHoverForeground,
    ListItemSecondaryHoverBackground,
    ListItemSelectionBackground,
    ListItemSelectionForeground,
    ListItemSecondarySelectionBackground,
    ListItemSecondarySelectionForeground,
    GridItemSelectionOutline,
    GridItemHoverOutline,
    GridItemBackground,
    ScrollBarBackground,
    SecondaryScrollBarBackground,
    PopoverBackground,
    PopoverBorder,
    SecondaryBackgroundBorder,
    TooltipBackground,
    TooltipForeground,
    TooltipBorder,
    LoadingBar,
    LoadingSpinner,
}

const KEYS: &[(&str, Semantic)] = &[
    ("core.accent", Semantic::Accent),
    ("core.accent_foreground", Semantic::AccentForeground),
    ("core.background", Semantic::Background),
    ("core.foreground", Semantic::Foreground),
    ("core.secondary_background", Semantic::SecondaryBackground),
    ("core.border", Semantic::Border),
    ("shortcut.border", Semantic::ShortcutBorder),
    ("main_window.border", Semantic::MainWindowBorder),
    (
        "main_window.footer.background",
        Semantic::StatusBarBackground,
    ),
    ("settings_window.border", Semantic::SettingsWindowBorder),
    ("accents.blue", Semantic::Blue),
    ("accents.green", Semantic::Green),
    ("accents.magenta", Semantic::Magenta),
    ("accents.orange", Semantic::Orange),
    ("accents.red", Semantic::Red),
    ("accents.yellow", Semantic::Yellow),
    ("accents.cyan", Semantic::Cyan),
    ("accents.purple", Semantic::Purple),
    ("text.default", Semantic::TextPrimary),
    ("text.muted", Semantic::TextMuted),
    ("text.danger", Semantic::TextDanger),
    ("text.success", Semantic::TextSuccess),
    ("text.placeholder", Semantic::TextPlaceholder),
    ("text.links.default", Semantic::LinkDefault),
    ("text.links.visited", Semantic::LinkVisited),
    (
        "text.selection.background",
        Semantic::TextSelectionBackground,
    ),
    (
        "text.selection.foreground",
        Semantic::TextSelectionForeground,
    ),
    ("input.background", Semantic::InputBackground),
    ("input.border", Semantic::InputBorder),
    ("input.border_focus", Semantic::InputBorderFocus),
    ("input.border_error", Semantic::InputBorderError),
    (
        "button.primary.background",
        Semantic::ButtonPrimaryBackground,
    ),
    (
        "button.primary.foreground",
        Semantic::ButtonPrimaryForeground,
    ),
    (
        "button.primary.hover.background",
        Semantic::ButtonPrimaryHoverBackground,
    ),
    (
        "button.primary.hover.foreground",
        Semantic::ButtonPrimaryHoverForeground,
    ),
    (
        "button.primary.focus.outline",
        Semantic::ButtonPrimaryFocusOutline,
    ),
    (
        "list.item.hover.background",
        Semantic::ListItemHoverBackground,
    ),
    (
        "list.item.hover.foreground",
        Semantic::ListItemHoverForeground,
    ),
    (
        "list.item.hover.secondary_foreground",
        Semantic::ListItemSecondaryHoverForeground,
    ),
    (
        "list.item.selection.background",
        Semantic::ListItemSelectionBackground,
    ),
    (
        "list.item.selection.foreground",
        Semantic::ListItemSelectionForeground,
    ),
    (
        "list.item.selection.secondary_background",
        Semantic::ListItemSecondarySelectionBackground,
    ),
    (
        "list.item.selection.secondary_foreground",
        Semantic::ListItemSecondarySelectionForeground,
    ),
    (
        "grid.item.selection.outline",
        Semantic::GridItemSelectionOutline,
    ),
    ("grid.item.hover.outline", Semantic::GridItemHoverOutline),
    ("grid.item.background", Semantic::GridItemBackground),
    ("scrollbars.background", Semantic::ScrollBarBackground),
    (
        "scrollbars.secondary_background",
        Semantic::SecondaryScrollBarBackground,
    ),
    ("tooltip.background", Semantic::TooltipBackground),
    ("tooltip.foreground", Semantic::TooltipForeground),
    ("tooltip.border", Semantic::TooltipBorder),
    ("loading.bar", Semantic::LoadingBar),
    ("loading.spinner", Semantic::LoadingSpinner),
];

impl Semantic {
    pub fn from_key(key: &str) -> Option<Self> {
        KEYS.iter()
            .find(|(name, _)| *name == key)
            .map(|&(_, semantic)| semantic)
    }

    pub fn key(self) -> Option<&'static str> {
        KEYS.iter()
            .find(|&&(_, semantic)| semantic == self)
            .map(|&(name, _)| name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorValue {
    Literal(Color),
    Reference {
        target: Semantic,
        opacity: Option<f32>,
        lighter: Option<i32>,
        darker: Option<i32>,
    },
}

/// A theme as written in a `.toml` file, in the format Vicinae uses.
#[derive(Debug, Clone)]
pub struct ThemeFile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub variant: Variant,
    pub inherits: Option<String>,
    pub colors: HashMap<Semantic, ColorValue>,
}

#[derive(Debug)]
pub struct Parsed {
    pub theme: ThemeFile,
    pub warnings: Vec<String>,
}

impl ThemeFile {
    pub fn load(path: &Path) -> Result<Parsed> {
        let id = path
            .file_stem()
            .context("theme files need a name")?
            .to_string_lossy();
        let source = std::fs::read_to_string(path)?;
        Self::parse(&id, &source)
    }

    pub fn parse(id: &str, source: &str) -> Result<Parsed> {
        let table: toml::Table = source.parse()?;
        let meta = table
            .get("meta")
            .and_then(toml::Value::as_table)
            .context("a [meta] table is required")?;
        let text = |key: &str| meta.get(key).and_then(toml::Value::as_str);
        let name = text("name").context("meta.name must be a string")?;
        let variant = match text("variant") {
            Some("light") => Variant::Light,
            Some(_) => Variant::Dark,
            None => bail!(r#"meta.variant must be "light" or "dark""#),
        };

        let mut colors = HashMap::new();
        let mut warnings = Vec::new();
        collect_colors(&table, "", &mut colors, &mut warnings);
        if let Some(semantic) = first_circular_reference(&colors) {
            bail!(
                "colors.{} refers back to itself",
                semantic.key().unwrap_or_default()
            );
        }

        Ok(Parsed {
            theme: ThemeFile {
                id: id.to_owned(),
                name: name.to_owned(),
                description: text("description").unwrap_or_default().to_owned(),
                variant,
                inherits: text("inherits").map(str::to_owned),
                colors,
            },
            warnings,
        })
    }
}

fn collect_colors(
    table: &toml::Table,
    prefix: &str,
    colors: &mut HashMap<Semantic, ColorValue>,
    warnings: &mut Vec<String>,
) {
    for (key, value) in table {
        if prefix.is_empty() && key == "meta" {
            continue;
        }
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        if let Some(semantic) = path.strip_prefix("colors.").and_then(Semantic::from_key) {
            match parse_color(value) {
                Ok(color) => {
                    colors.insert(semantic, color);
                }
                Err(error) => warnings.push(format!("{path} is not a valid color: {error}")),
            }
            continue;
        }
        match value.as_table() {
            Some(table) => collect_colors(table, &path, colors, warnings),
            None => warnings.push(format!("unused key {path}")),
        }
    }
}

fn parse_color(value: &toml::Value) -> Result<ColorValue> {
    if let Some(name) = value.as_str() {
        return parse_color_name(name);
    }
    let table = value.as_table().context("expected a string or a table")?;
    let name = table
        .get("name")
        .and_then(toml::Value::as_str)
        .context("a color table needs a name")?;
    let opacity = table
        .get("opacity")
        .and_then(|value| value.as_float().or(value.as_integer().map(|n| n as f64)))
        .map(|opacity| opacity as f32);
    let factor = |key: &str| {
        table
            .get(key)
            .and_then(toml::Value::as_integer)
            .map(|n| n as i32)
    };
    let (lighter, darker) = (factor("lighter"), factor("darker"));
    Ok(match parse_color_name(name)? {
        ColorValue::Literal(mut color) => {
            if let Some(opacity) = opacity {
                color = color.with_alpha(opacity);
            }
            if let Some(lighter) = lighter {
                color = color.lighter(lighter);
            }
            if let Some(darker) = darker {
                color = color.darker(darker);
            }
            ColorValue::Literal(color)
        }
        ColorValue::Reference { target, .. } => ColorValue::Reference {
            target,
            opacity,
            lighter,
            darker,
        },
    })
}

fn parse_color_name(name: &str) -> Result<ColorValue> {
    if let Some(target) = name.strip_prefix("colors.").and_then(Semantic::from_key) {
        return Ok(ColorValue::Reference {
            target,
            opacity: None,
            lighter: None,
            darker: None,
        });
    }
    match Color::parse(name) {
        Some(color) => Ok(ColorValue::Literal(color)),
        None => bail!("{name} is not a color or a color reference"),
    }
}

fn first_circular_reference(colors: &HashMap<Semantic, ColorValue>) -> Option<Semantic> {
    colors.keys().copied().find(|&start| {
        let mut seen = HashSet::new();
        let mut current = start;
        while let Some(ColorValue::Reference { target, .. }) = colors.get(&current) {
            if !seen.insert(*target) {
                return true;
            }
            current = *target;
        }
        false
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Parsed {
        ThemeFile::parse("test", source).unwrap()
    }

    #[test]
    fn reads_meta_colors_and_references() {
        let parsed = parse(
            r##"
            [meta]
            name = "Test"
            description = "A test theme"
            variant = "light"
            inherits = "noty-light"

            [colors.core]
            background = "#fafafb"
            accent = "colors.accents.blue"

            [colors.main_window]
            footer = { background = "colors.core.secondary_background" }

            [colors.list.item.selection]
            secondary_background = { name = "colors.core.accent", opacity = 0.45 }
            background = { name = "#808080", darker = 200 }
            "##,
        );
        let theme = parsed.theme;
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        assert_eq!(theme.name, "Test");
        assert_eq!(theme.variant, Variant::Light);
        assert_eq!(theme.inherits.as_deref(), Some("noty-light"));
        assert_eq!(
            theme.colors[&Semantic::Background],
            ColorValue::Literal(Color::parse("#fafafb").unwrap())
        );
        assert!(matches!(
            theme.colors[&Semantic::Accent],
            ColorValue::Reference {
                target: Semantic::Blue,
                opacity: None,
                ..
            }
        ));
        assert!(matches!(
            theme.colors[&Semantic::StatusBarBackground],
            ColorValue::Reference {
                target: Semantic::SecondaryBackground,
                ..
            }
        ));
        assert!(matches!(
            theme.colors[&Semantic::ListItemSecondarySelectionBackground],
            ColorValue::Reference { target: Semantic::Accent, opacity: Some(o), .. } if (o - 0.45).abs() < 1e-6
        ));
        assert_eq!(
            theme.colors[&Semantic::ListItemSelectionBackground],
            ColorValue::Literal(Color::parse("#404040").unwrap())
        );
    }

    #[test]
    fn unknown_keys_and_bad_colors_are_warnings() {
        let parsed = parse(
            r##"
            [meta]
            name = "Test"
            variant = "dark"
            version = 1

            [colors.core]
            background = "not a color"
            glow = "#ffffff"
            "##,
        );
        assert_eq!(parsed.theme.variant, Variant::Dark);
        assert!(parsed.theme.colors.is_empty());
        assert_eq!(parsed.warnings.len(), 2, "{:?}", parsed.warnings);
    }

    #[test]
    fn meta_name_and_variant_are_required() {
        assert!(ThemeFile::parse("t", "[colors.core]\nbackground = \"#000\"").is_err());
        assert!(ThemeFile::parse("t", "[meta]\nvariant = \"dark\"").is_err());
        assert!(ThemeFile::parse("t", "[meta]\nname = \"x\"").is_err());
    }

    #[test]
    fn circular_references_are_rejected() {
        let source = r#"
            [meta]
            name = "Loop"
            variant = "dark"
            [colors.core]
            background = "colors.core.foreground"
            foreground = "colors.core.background"
        "#;
        let error = ThemeFile::parse("loop", source).unwrap_err().to_string();
        assert!(error.contains("refers back to itself"), "{error}");
    }

    #[test]
    fn every_semantic_with_a_key_round_trips() {
        for (key, semantic) in KEYS {
            assert_eq!(Semantic::from_key(key), Some(*semantic));
            assert_eq!(semantic.key(), Some(*key));
        }
    }
}
