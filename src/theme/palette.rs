use std::sync::Arc;

use super::color::Color;
use super::file::{ColorValue, Semantic, ThemeFile, Variant};

const MAX_DEPTH: usize = 64;

/// A theme file linked to the theme it inherits from. Every semantic color
/// resolves: from the file, derived from other colors, or from the parent.
#[derive(Debug, Clone)]
pub struct Theme {
    pub file: ThemeFile,
    parent: Option<Arc<Theme>>,
}

impl Theme {
    pub fn new(file: ThemeFile, parent: Option<Arc<Theme>>) -> Self {
        Self { file, parent }
    }

    pub fn id(&self) -> &str {
        &self.file.id
    }

    pub fn name(&self) -> &str {
        &self.file.name
    }

    pub fn variant(&self) -> Variant {
        self.file.variant
    }

    pub fn is_dark(&self) -> bool {
        self.file.variant == Variant::Dark
    }

    pub fn color(&self, semantic: Semantic) -> Color {
        self.resolve(semantic, 0)
    }

    fn resolve(&self, semantic: Semantic, depth: usize) -> Color {
        if depth > MAX_DEPTH {
            return Color::TRANSPARENT;
        }
        match self.file.colors.get(&semantic) {
            Some(ColorValue::Literal(color)) => *color,
            Some(&ColorValue::Reference {
                target,
                opacity,
                lighter,
                darker,
            }) => {
                let mut color = self.resolve(target, depth + 1);
                if let Some(opacity) = opacity {
                    let background = self.resolve(Semantic::Background, depth + 1);
                    color = color.over(background, opacity);
                }
                if let Some(darker) = darker {
                    color = color.darker((darker + 100).max(0));
                }
                if let Some(lighter) = lighter {
                    color = color.lighter((lighter + 100).max(0));
                }
                color
            }
            None => self.derive(semantic, depth + 1),
        }
    }

    fn inherit(&self, semantic: Semantic, depth: usize) -> Color {
        self.parent
            .as_ref()
            .map_or(Color::TRANSPARENT, |parent| parent.resolve(semantic, depth))
    }

    fn derive(&self, semantic: Semantic, depth: usize) -> Color {
        use Semantic::*;
        let get = |semantic| self.resolve(semantic, depth);
        let hover_for = |selection| {
            if self.is_dark() {
                get(selection).darker(110)
            } else {
                get(selection).mix(get(Background), 0.25)
            }
        };
        match semantic {
            Background | Foreground | SecondaryBackground | Border | Blue | Green | Magenta
            | Orange | Red | Yellow | Cyan | Purple => self.inherit(semantic, depth),
            Accent => get(Blue),
            AccentForeground => Color::WHITE,
            TextPrimary => get(Foreground),
            TextMuted => get(TextPrimary).with_alpha(0.7),
            TextDanger => get(Red),
            TextSuccess => get(Green),
            TextPlaceholder => get(TextPrimary).with_alpha(0.6),
            LinkDefault => get(Blue),
            LinkVisited => get(Purple),
            TextSelectionBackground => get(Accent),
            TextSelectionForeground => get(AccentForeground),
            MainWindowBorder if self.is_dark() => get(Border).lighter(110),
            MainWindowBorder => get(Border).darker(110),
            SettingsWindowBorder => get(MainWindowBorder),
            StatusBarBackground => get(SecondaryBackground),
            ShortcutBorder => get(Border),
            SecondaryBackgroundBorder => get(Border).lighter(150),
            InputBackground => get(GridItemBackground),
            InputBorder => get(Border),
            InputBorderFocus => get(InputBorder).mix(get(Foreground), 0.4),
            InputBorderError => get(Red),
            ButtonPrimaryBackground => get(ListItemSelectionBackground).with_alpha(0.7),
            ButtonPrimaryForeground => get(Foreground),
            ButtonPrimaryHoverBackground => get(ButtonPrimaryBackground).with_alpha(0.7),
            ButtonPrimaryHoverForeground => get(ButtonPrimaryBackground),
            ButtonPrimaryFocusOutline => get(InputBorderFocus),
            ListItemSelectionBackground => get(SecondaryBackground),
            ListItemSelectionForeground => get(Foreground),
            ListItemHoverBackground => hover_for(ListItemSelectionBackground),
            ListItemHoverForeground => get(Foreground),
            ListItemSecondarySelectionBackground => get(ListItemSelectionBackground),
            ListItemSecondarySelectionForeground => get(ListItemSelectionForeground),
            ListItemSecondaryHoverBackground => hover_for(ListItemSecondarySelectionBackground),
            ListItemSecondaryHoverForeground => get(ListItemSecondarySelectionForeground),
            GridItemBackground => get(SecondaryBackground),
            GridItemSelectionOutline => get(Foreground),
            GridItemHoverOutline => get(GridItemSelectionOutline),
            ScrollBarBackground => get(Foreground).with_alpha(0.25),
            SecondaryScrollBarBackground => get(ScrollBarBackground),
            PopoverBackground => get(Background),
            PopoverBorder => get(MainWindowBorder),
            TooltipBackground => get(PopoverBackground),
            TooltipForeground => get(Foreground),
            TooltipBorder => get(PopoverBorder),
            LoadingBar => get(TextMuted),
            LoadingSpinner => get(Foreground),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme(id: &str, source: &str, parent: Option<Arc<Theme>>) -> Arc<Theme> {
        let parsed = ThemeFile::parse(id, source).unwrap();
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        Arc::new(Theme::new(parsed.theme, parent))
    }

    fn base() -> Arc<Theme> {
        theme(
            "base",
            r##"
            [meta]
            name = "Base"
            variant = "dark"
            [colors.core]
            background = "#101010"
            foreground = "#ffffff"
            secondary_background = "#202020"
            border = "#303030"
            [colors.accents]
            blue = "#0000ff"
            green = "#00ff00"
            red = "#ff0000"
            "##,
            None,
        )
    }

    fn hex(theme: &Theme, semantic: Semantic) -> String {
        theme.color(semantic).to_hex()
    }

    #[test]
    fn missing_colors_are_derived_like_vicinae() {
        let base = base();
        assert_eq!(hex(&base, Semantic::Accent), "#0000ffff");
        assert_eq!(hex(&base, Semantic::TextMuted), "#ffffffb3");
        assert_eq!(
            hex(&base, Semantic::ListItemSelectionBackground),
            "#202020ff"
        );
        assert_eq!(hex(&base, Semantic::ListItemHoverBackground), "#1d1d1dff");
        assert_eq!(hex(&base, Semantic::StatusBarBackground), "#202020ff");
        assert_eq!(hex(&base, Semantic::TextDanger), "#ff0000ff");
        assert_eq!(hex(&base, Semantic::Magenta), "#00000000");
    }

    #[test]
    fn children_inherit_what_they_do_not_set() {
        let child = theme(
            "child",
            r##"
            [meta]
            name = "Child"
            variant = "light"
            [colors.core]
            background = "#f0f0f0"
            "##,
            Some(base()),
        );
        assert_eq!(hex(&child, Semantic::Background), "#f0f0f0ff");
        assert_eq!(hex(&child, Semantic::Foreground), "#ffffffff");
        assert_eq!(hex(&child, Semantic::ListItemHoverBackground), "#545454ff");
    }

    #[test]
    fn references_apply_opacity_and_brightness() {
        let child = theme(
            "child",
            r##"
            [meta]
            name = "Child"
            variant = "dark"
            [colors.core]
            accent = { name = "colors.accents.red", opacity = 0.5 }
            secondary_background = { name = "colors.core.border", lighter = 100 }
            border = { name = "colors.core.background", darker = 100 }
            "##,
            Some(base()),
        );
        assert_eq!(hex(&child, Semantic::Accent), "#880808ff");
        assert_eq!(hex(&child, Semantic::Border), "#080808ff");
        assert_eq!(hex(&child, Semantic::SecondaryBackground), "#101010ff");
    }

    #[test]
    fn loops_through_derived_colors_do_not_overflow() {
        let looping = theme(
            "looping",
            r#"
            [meta]
            name = "Looping"
            variant = "dark"
            [colors.text]
            default = "colors.text.muted"
            "#,
            Some(base()),
        );
        assert!(hex(&looping, Semantic::TextPrimary).starts_with("#000000"));
    }
}
