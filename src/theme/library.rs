use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
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

fn without_vicinae_alias(id: &str) -> &str {
    match id {
        "vicinae-dark" => base_theme_id(Variant::Dark),
        "vicinae-light" => base_theme_id(Variant::Light),
        other => other,
    }
}

pub struct ThemeLibrary {
    themes: Vec<Arc<Theme>>,
    problems: Vec<String>,
}

impl ThemeLibrary {
    /// The built-in themes plus the `.toml` files in `dirs`. Earlier
    /// directories win, and a user theme replaces a built-in one with the
    /// same file name.
    pub fn load(dirs: &[PathBuf]) -> Self {
        let mut order = Vec::new();
        let mut files = HashMap::new();
        for (id, source) in BUILT_IN {
            let parsed = ThemeFile::parse(id, source).expect("built-in themes are valid");
            order.push(parsed.theme.id.clone());
            files.insert(parsed.theme.id.clone(), parsed.theme);
        }

        let (mut user_themes, problems) = read_theme_dirs(dirs);
        user_themes.sort_by_key(|theme| theme.name.to_lowercase());
        for theme in user_themes {
            if !files.contains_key(&theme.id) {
                order.push(theme.id.clone());
            }
            files.insert(theme.id.clone(), theme);
        }

        let mut linked = HashMap::new();
        let themes = order
            .iter()
            .filter_map(|id| link(id, &files, &mut linked, &mut HashSet::new()))
            .collect();
        Self { themes, problems }
    }

    pub fn user_dir() -> Option<PathBuf> {
        dirs::data_dir().map(|dir| dir.join("noty").join("themes"))
    }

    /// The user's theme folder first, then the system data folders on Linux.
    pub fn search_dirs() -> Vec<PathBuf> {
        let mut search_dirs: Vec<PathBuf> = Self::user_dir().into_iter().collect();
        if cfg!(target_os = "linux")
            && let Ok(data_dirs) = std::env::var("XDG_DATA_DIRS")
        {
            search_dirs.extend(
                data_dirs
                    .split(':')
                    .filter(|dir| !dir.is_empty())
                    .map(|dir| Path::new(dir).join("noty").join("themes")),
            );
        }
        search_dirs
    }

    pub fn themes(&self) -> &[Arc<Theme>] {
        &self.themes
    }

    pub fn get(&self, id: &str) -> Option<&Arc<Theme>> {
        self.themes.iter().find(|theme| theme.id() == id)
    }

    pub fn problems(&self) -> &[String] {
        &self.problems
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

fn read_theme_dirs(dirs: &[PathBuf]) -> (Vec<ThemeFile>, Vec<String>) {
    let mut themes: Vec<ThemeFile> = Vec::new();
    let mut problems = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "toml")
            })
            .collect();
        paths.sort();
        for path in paths {
            match ThemeFile::load(&path) {
                Ok(parsed) if !themes.iter().any(|theme| theme.id == parsed.theme.id) => {
                    problems.extend(
                        parsed
                            .warnings
                            .iter()
                            .map(|warning| format!("{}: {warning}", path.display())),
                    );
                    themes.push(parsed.theme);
                }
                Ok(_) => {}
                Err(error) => problems.push(format!("{}: {error:#}", path.display())),
            }
        }
    }
    (themes, problems)
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
        let wanted = file.inherits.as_deref().map_or(base, without_vicinae_alias);
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

    fn write(dir: &Path, file_name: &str, source: &str) {
        fs::write(dir.join(file_name), source).unwrap();
    }

    #[test]
    fn built_in_themes_parse_without_warnings() {
        for (id, source) in BUILT_IN {
            let parsed = ThemeFile::parse(id, source).unwrap();
            assert!(parsed.warnings.is_empty(), "{id}: {:?}", parsed.warnings);
        }
    }

    #[test]
    fn built_in_themes_resolve_their_essential_colors() {
        let library = ThemeLibrary::load(&[]);
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
        let library = ThemeLibrary::load(&[]);
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
        let library = ThemeLibrary::load(&[]);
        assert_eq!(library.pick("missing", Variant::Dark).id(), "noty-dark");
        assert_eq!(library.pick("vesper", Variant::Light).id(), "noty-light");
    }

    #[test]
    fn user_themes_extend_and_replace_built_in_ones() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "ink.toml",
            "[meta]\nname = \"Ink\"\nvariant = \"dark\"\ninherits = \"vicinae-dark\"\n\
             [colors.core]\nbackground = \"#000000\"\n",
        );
        write(
            dir.path(),
            "nord.toml",
            "[meta]\nname = \"My Nord\"\nvariant = \"dark\"\n",
        );
        write(dir.path(), "broken.toml", "[meta]\nname = 3\n");
        write(dir.path(), "notes.txt", "not a theme");

        let library = ThemeLibrary::load(&[dir.path().to_path_buf()]);
        let ink = library.get("ink").unwrap();
        assert_eq!(ink.color(Semantic::Background).to_hex(), "#000000ff");
        assert_eq!(ink.color(Semantic::Foreground).to_hex(), "#d9dce3ff");
        assert_eq!(library.get("nord").unwrap().name(), "My Nord");
        assert_eq!(library.themes().len(), BUILT_IN.len() + 1);
        assert_eq!(library.problems().len(), 1, "{:?}", library.problems());
    }

    #[test]
    fn earlier_directories_win() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        write(
            first.path(),
            "a.toml",
            "[meta]\nname = \"First\"\nvariant = \"dark\"\n",
        );
        write(
            second.path(),
            "a.toml",
            "[meta]\nname = \"Second\"\nvariant = \"dark\"\n",
        );
        let library =
            ThemeLibrary::load(&[first.path().to_path_buf(), second.path().to_path_buf()]);
        assert_eq!(library.get("a").unwrap().name(), "First");
    }

    #[test]
    fn inheritance_cycles_fall_back_to_the_base_theme() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "a.toml",
            "[meta]\nname = \"A\"\nvariant = \"light\"\ninherits = \"b\"\n",
        );
        write(
            dir.path(),
            "b.toml",
            "[meta]\nname = \"B\"\nvariant = \"light\"\ninherits = \"a\"\n",
        );
        let library = ThemeLibrary::load(&[dir.path().to_path_buf()]);
        let a = library.get("a").unwrap();
        assert_eq!(a.color(Semantic::Background).to_hex(), "#fbfaf7ff");
    }
}
