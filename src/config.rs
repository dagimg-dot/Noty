use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

pub const MIN_FONT_SIZE: f32 = 8.;
pub const MAX_FONT_SIZE: f32 = 32.;
const DEFAULT_FONT_SIZE: f32 = 12.;
const DEFAULT_WINDOW_SIZE: (f32, f32) = (500., 800.);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMethod {
    Name,
    Modified,
}

/// Noty's settings file, shared with the Python version of the app.
///
/// Keys this version doesn't understand are kept as they are, so switching
/// between versions never loses a setting.
#[derive(Debug, Default)]
pub struct Config {
    path: Option<PathBuf>,
    values: Map<String, Value>,
}

impl Config {
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| home_dir().join(".config"))
            .join(format!("{}.json", crate::APP_ID))
    }

    pub fn load(path: PathBuf) -> Self {
        let values = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self {
            path: Some(path),
            values,
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, serde_json::to_string(&self.values)?)
    }

    pub fn notes_dir(&self) -> PathBuf {
        self.get::<String>("notes_dir")
            .map(|dir| expand_home(&dir))
            .unwrap_or_else(default_notes_dir)
    }

    pub fn recurse_subfolders(&self) -> bool {
        self.get("recurse_subfolders").unwrap_or(false)
    }

    pub fn use_file_extension(&self) -> bool {
        self.get("use_file_extension").unwrap_or(false)
    }

    pub fn sort_method(&self) -> SortMethod {
        match self.get::<String>("sorting_method").as_deref() {
            Some("date_modified") => SortMethod::Modified,
            _ => SortMethod::Name,
        }
    }

    pub fn appearance(&self) -> Appearance {
        match self.get::<String>("theme").as_deref() {
            Some("light") => Appearance::Light,
            Some("dark") => Appearance::Dark,
            _ => Appearance::System,
        }
    }

    pub fn set_appearance(&mut self, appearance: Appearance) {
        let name = match appearance {
            Appearance::System => "system",
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        };
        self.set("theme", name);
    }

    /// The editor font size in points.
    pub fn font_size(&self) -> f32 {
        self.get("font_size")
            .unwrap_or(DEFAULT_FONT_SIZE)
            .clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
    }

    pub fn set_font_size(&mut self, points: f32) {
        self.set(
            "font_size",
            points.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE).round() as i64,
        );
    }

    pub fn default_font_size() -> f32 {
        DEFAULT_FONT_SIZE
    }

    /// The custom editor font family, when the user picked one.
    pub fn editor_font(&self) -> Option<String> {
        if !self.get::<bool>("use_custom_font").unwrap_or(false) {
            return None;
        }
        let family = pango_family(&self.get::<String>("custom_font")?);
        (!family.is_empty() && family != "Monospace").then_some(family)
    }

    pub fn show_line_numbers(&self) -> bool {
        self.get("editor_show_line_numbers").unwrap_or(true)
    }

    pub fn set_show_line_numbers(&mut self, show: bool) {
        self.set("editor_show_line_numbers", show);
    }

    pub fn window_size(&self) -> (f32, f32) {
        self.persist_window_size()
            .then(|| self.get::<Map<String, Value>>("windowsize"))
            .flatten()
            .and_then(|size| Some((size.get("width")?.as_f64()?, size.get("height")?.as_f64()?)))
            .map(|(width, height)| (width as f32, height as f32))
            .unwrap_or(DEFAULT_WINDOW_SIZE)
    }

    pub fn set_window_size(&mut self, width: f32, height: f32) {
        if self.persist_window_size() {
            self.set(
                "windowsize",
                json!({ "width": width.round() as i64, "height": height.round() as i64 }),
            );
        }
    }

    pub fn last_opened_file(&self) -> Option<PathBuf> {
        self.get::<String>("last_opened_file")
            .map(|path| expand_home(&path))
    }

    pub fn set_last_opened_file(&mut self, path: Option<&Path>) {
        self.set("last_opened_file", path.map(|path| path.to_string_lossy()));
    }

    fn persist_window_size(&self) -> bool {
        self.get("persist_window_size").unwrap_or(true)
    }

    fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        serde_json::from_value(self.values.get(key)?.clone()).ok()
    }

    fn set(&mut self, key: &str, value: impl Serialize) {
        if let Ok(value) = serde_json::to_value(value) {
            self.values.insert(key.to_owned(), value);
        }
    }
}

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
}

fn expand_home(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => home_dir().join(rest),
        None => PathBuf::from(path),
    }
}

fn default_notes_dir() -> PathBuf {
    dirs::document_dir()
        .unwrap_or_else(|| home_dir().join("Documents"))
        .join("Noties")
}

/// Takes the family out of a Pango font description like "Iosevka Bold 12".
fn pango_family(description: &str) -> String {
    const STYLE_WORDS: &[&str] = &[
        "Bold",
        "Italic",
        "Oblique",
        "Light",
        "Medium",
        "Semi-Bold",
        "SemiBold",
        "Heavy",
        "Regular",
        "Thin",
        "Book",
        "Condensed",
    ];
    let mut words: Vec<&str> = description.split_whitespace().collect();
    while let Some(last) = words.last() {
        let is_size = last.trim_end_matches("px").parse::<f32>().is_ok();
        if is_size || STYLE_WORDS.contains(last) {
            words.pop();
        } else {
            break;
        }
    }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(json: &str) -> (tempfile::TempDir, Config) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("noty.json");
        fs::write(&path, json).unwrap();
        let config = Config::load(path);
        (dir, config)
    }

    #[test]
    fn missing_file_gives_python_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(dir.path().join("absent.json"));
        assert_eq!(config.appearance(), Appearance::System);
        assert_eq!(config.sort_method(), SortMethod::Name);
        assert_eq!(config.font_size(), 12.);
        assert_eq!(config.window_size(), (500., 800.));
        assert!(config.show_line_numbers());
        assert!(!config.use_file_extension());
        assert!(config.editor_font().is_none());
        assert!(config.last_opened_file().is_none());
    }

    #[test]
    fn reads_values_written_by_the_python_app() {
        let (_dir, config) = config_with(
            r#"{"windowsize": {"width": 640, "height": 900}, "notes_dir": "/notes",
                "sorting_method": "date_modified", "theme": "dark", "font_size": 14,
                "use_custom_font": true, "custom_font": "Iosevka Bold 13",
                "last_opened_file": null, "editor_vim_mode": false}"#,
        );
        assert_eq!(config.notes_dir(), PathBuf::from("/notes"));
        assert_eq!(config.sort_method(), SortMethod::Modified);
        assert_eq!(config.appearance(), Appearance::Dark);
        assert_eq!(config.font_size(), 14.);
        assert_eq!(config.editor_font().as_deref(), Some("Iosevka"));
        assert_eq!(config.window_size(), (640., 900.));
        assert!(config.last_opened_file().is_none());
    }

    #[test]
    fn ignores_values_of_the_wrong_type() {
        let (_dir, config) = config_with(r#"{"font_size": "huge", "theme": 3, "windowsize": []}"#);
        assert_eq!(config.font_size(), 12.);
        assert_eq!(config.appearance(), Appearance::System);
        assert_eq!(config.window_size(), (500., 800.));
    }

    #[test]
    fn saving_keeps_settings_this_version_does_not_use() {
        let (dir, mut config) = config_with(r#"{"editor_vim_mode": true, "font_size": 12}"#);
        config.set_font_size(15.4);
        config.set_appearance(Appearance::Light);
        config.set_last_opened_file(Some(Path::new("/notes/todo")));
        config.save().unwrap();

        let saved: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("noty.json")).unwrap())
                .unwrap();
        assert_eq!(saved["editor_vim_mode"], true);
        assert_eq!(saved["font_size"], 15);
        assert_eq!(saved["theme"], "light");
        assert_eq!(saved["last_opened_file"], "/notes/todo");
    }

    #[test]
    fn window_size_is_not_stored_when_persistence_is_off() {
        let (_dir, mut config) = config_with(r#"{"persist_window_size": false}"#);
        config.set_window_size(700., 700.);
        assert_eq!(config.window_size(), (500., 800.));
    }

    #[test]
    fn pango_descriptions_reduce_to_the_family() {
        assert_eq!(pango_family("JetBrains Mono Bold 12"), "JetBrains Mono");
        assert_eq!(pango_family("Monaspace Argon NF"), "Monaspace Argon NF");
        assert_eq!(pango_family("Monospace 12"), "Monospace");
    }
}
