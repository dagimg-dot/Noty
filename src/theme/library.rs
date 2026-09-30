use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::file::{ThemeFile, Variant};
use super::palette::Theme;

const BUILT_IN: &[(&str, &str)] = &[
    (
        "noty-dark",
        include_str!("../../assets/themes/noty-dark.toml"),
    ),
    (
        "noty-light",
        include_str!("../../assets/themes/noty-light.toml"),
    ),
    (
        "libadwaita-dark",
        include_str!("../../assets/themes/libadwaita-dark.toml"),
    ),
    (
        "libadwaita-light",
        include_str!("../../assets/themes/libadwaita-light.toml"),
    ),
    (
        "catppuccin-latte",
        include_str!("../../assets/themes/catppuccin-latte.toml"),
    ),
    (
        "catppuccin-frappe",
        include_str!("../../assets/themes/catppuccin-frappe.toml"),
    ),
    (
        "catppuccin-macchiato",
        include_str!("../../assets/themes/catppuccin-macchiato.toml"),
    ),
    (
        "catppuccin-mocha",
        include_str!("../../assets/themes/catppuccin-mocha.toml"),
    ),
    ("nord", include_str!("../../assets/themes/nord.toml")),
    (
        "nord-light",
        include_str!("../../assets/themes/nord-light.toml"),
    ),
    (
        "tokyo-night",
        include_str!("../../assets/themes/tokyo-night.toml"),
    ),
    (
        "tokyo-night-storm",
        include_str!("../../assets/themes/tokyo-night-storm.toml"),
    ),
    ("vesper", include_str!("../../assets/themes/vesper.toml")),
];

pub fn base_theme_id(variant: Variant) -> &'static str {
    match variant {
        Variant::Dark => "noty-dark",
        Variant::Light => "noty-light",
    }
}

fn is_base(id: &str) -> bool {
    id == base_theme_id(Variant::Dark) || id == base_theme_id(Variant::Light)
}

pub struct ThemeLibrary {
    themes: Vec<Arc<Theme>>,
}

impl ThemeLibrary {
    pub fn load() -> Self {
        let mut order = Vec::new();
        let mut files = HashMap::new();
        for (id, source) in BUILT_IN {
            let parsed = ThemeFile::parse(id, source).expect("built-in themes are valid");
            order.push(parsed.theme.id.clone());
            files.insert(parsed.theme.id.clone(), parsed.theme);
        }

        let mut linked = HashMap::new();
        let themes = order
            .iter()
            .filter_map(|id| link(id, &files, &mut linked, &mut HashSet::new()))
            .collect();
        Self { themes }
    }

    pub fn themes(&self) -> &[Arc<Theme>] {
        &self.themes
    }

    pub fn get(&self, id: &str) -> Option<&Arc<Theme>> {
        self.themes.iter().find(|theme| theme.id() == id)
    }

    /// The theme `id` if it exists and is a `variant` theme, otherwise
    /// Noty's own theme for that variant.
    pub fn pick(&self, id: &str, variant: Variant) -> Arc<Theme> {
        self.get(id)
            .filter(|theme| theme.variant() == variant)
            .or_else(|| self.get(base_theme_id(variant)))
            .cloned()
            .expect("the base themes are built in")
    }
}

fn link(
    id: &str,
    files: &HashMap<String, ThemeFile>,
    linked: &mut HashMap<String, Arc<Theme>>,
    visiting: &mut HashSet<String>,
) -> Option<Arc<Theme>> {
    if let Some(theme) = linked.get(id) {
        return Some(theme.clone());
    }
    let file = files.get(id)?;
    let parent = if is_base(id) {
        None
    } else {
        visiting.insert(id.to_owned());
        let base = base_theme_id(file.variant);
        let wanted = file.inherits.as_deref().unwrap_or(base);
        let parent = if visiting.contains(wanted) {
            None
        } else {
            link(wanted, files, linked, visiting)
        };
        visiting.remove(id);
        parent.or_else(|| link(base, files, linked, visiting))
    };
    let theme = Arc::new(Theme::new(file.clone(), parent));
    linked.insert(id.to_owned(), theme.clone());
    Some(theme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Color, Semantic};

    #[test]
    fn built_in_themes_parse_without_warnings() {
        for (id, source) in BUILT_IN {
            let parsed = ThemeFile::parse(id, source).unwrap();
            assert!(parsed.warnings.is_empty(), "{id}: {:?}", parsed.warnings);
        }
    }

    #[test]
    fn built_in_themes_resolve_their_essential_colors() {
        let library = ThemeLibrary::load();
        assert_eq!(library.themes().len(), BUILT_IN.len());
        for theme in library.themes() {
            for semantic in [
                Semantic::Background,
                Semantic::Foreground,
                Semantic::SecondaryBackground,
                Semantic::Border,
                Semantic::Accent,
                Semantic::TextMuted,
                Semantic::Green,
                Semantic::Orange,
            ] {
                assert_ne!(
                    theme.color(semantic),
                    Color::TRANSPARENT,
                    "{} {semantic:?}",
                    theme.id()
                );
            }
        }
    }

    #[test]
    fn requested_themes_are_available_in_both_variants() {
        let library = ThemeLibrary::load();
        for (id, variant) in [
            ("libadwaita-light", Variant::Light),
            ("libadwaita-dark", Variant::Dark),
            ("tokyo-night", Variant::Dark),
            ("nord", Variant::Dark),
            ("nord-light", Variant::Light),
            ("vesper", Variant::Dark),
            ("catppuccin-latte", Variant::Light),
            ("catppuccin-mocha", Variant::Dark),
        ] {
            assert_eq!(library.pick(id, variant).id(), id);
        }
    }

    #[test]
    fn pick_falls_back_to_noty_for_unknown_or_mismatched_themes() {
        let library = ThemeLibrary::load();
        assert_eq!(library.pick("missing", Variant::Dark).id(), "noty-dark");
        assert_eq!(library.pick("vesper", Variant::Light).id(), "noty-light");
    }
}
