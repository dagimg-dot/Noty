// Package config loads and saves Noty's settings.
//
// The settings live in a JSON file named after the application ID,
// <config dir>/<app id>.json, with the same keys the Python version of
// Noty wrote, so existing users keep their settings. Keys this version
// does not know are kept as they are when the file is saved again.
package config

import (
	"encoding/json"
	"errors"
	"fmt"
	"io/fs"
	"log/slog"
	"os"
	"path/filepath"
)

// Appearance modes.
const (
	AppearanceSystem = "system"
	AppearanceLight  = "light"
	AppearanceDark   = "dark"
)

// Font size limits of the editor.
const (
	MinFontSize     = 8
	MaxFontSize     = 32
	DefaultFontSize = 12
)

// WindowSize is the size of the main window.
type WindowSize struct {
	Width  int `json:"width"`
	Height int `json:"height"`
}

// Config holds Noty's settings. The zero value is not useful: start from
// Defaults or Load.
type Config struct {
	WindowSize        WindowSize `json:"windowsize"`
	PersistWindowSize bool       `json:"persist_window_size"`
	NotesDir          string     `json:"notes_dir"`
	// MarkdownSyntax highlights Markdown in the editor.
	MarkdownSyntax bool `json:"show_markdown_syntax_highlighting"`
	// Theme is the appearance: AppearanceSystem, AppearanceLight or
	// AppearanceDark.
	Theme string `json:"theme"`
	// SortingMethod sorts the notes list: "name" or "date_modified", the
	// methods of notes.Store.Sorted.
	SortingMethod       string `json:"sorting_method"`
	ActivateRowOnSelect bool   `json:"activate_row_on_select"`
	// UseFileExtension gives new notes the .md extension.
	UseFileExtension bool `json:"use_file_extension"`
	// EditorColorScheme is the ID of the color theme.
	EditorColorScheme string `json:"editor_color_scheme"`
	RecurseSubfolders bool   `json:"recurse_subfolders"`
	FontSize          int    `json:"font_size"`
	UseCustomFont     bool   `json:"use_custom_font"`
	// CustomFont is a font description such as "Monospace 12"; only its
	// family is used, the size comes from FontSize.
	CustomFont                 string  `json:"custom_font"`
	LastOpenedFile             *string `json:"last_opened_file"`
	EditorShowLineNumbers      bool    `json:"editor_show_line_numbers"`
	EditorHighlightCurrentLine bool    `json:"editor_highlight_current_line"`
	// EditorVimMode is kept so the setting survives, though this version
	// has no Vim mode yet.
	EditorVimMode bool `json:"editor_vim_mode"`

	// extra holds the keys of the file this version does not know.
	extra map[string]json.RawMessage
}

// Defaults returns the settings of a new user, with notes in
// documentsDir/Noties.
func Defaults(documentsDir string) *Config {
	return &Config{
		WindowSize:                 WindowSize{Width: 500, Height: 800},
		PersistWindowSize:          true,
		NotesDir:                   filepath.Join(documentsDir, "Noties"),
		MarkdownSyntax:             false,
		Theme:                      AppearanceSystem,
		SortingMethod:              "name",
		ActivateRowOnSelect:        false,
		UseFileExtension:           false,
		EditorColorScheme:          "noty-classic",
		RecurseSubfolders:          false,
		FontSize:                   DefaultFontSize,
		UseCustomFont:              false,
		CustomFont:                 "Monospace 12",
		LastOpenedFile:             nil,
		EditorShowLineNumbers:      true,
		EditorHighlightCurrentLine: true,
		EditorVimMode:              false,
	}
}

// Store loads and saves the settings file.
type Store struct {
	// Path is where the settings are saved.
	Path string
	// Legacy are older locations read when Path does not exist yet, the
	// first that exists winning.
	Legacy []string
	// DocumentsDir is the user's documents directory, under which the
	// default notes directory is.
	DocumentsDir string
}

// NewStore returns the store of the application appID in the user's
// directories.
func NewStore(appID string) *Store {
	return &Store{
		Path:         Path(appID),
		Legacy:       LegacyPaths(appID),
		DocumentsDir: DocumentsDir(),
	}
}

// Load reads the settings. A missing file gives the defaults, which are
// saved; so does a file that cannot be parsed, which is first kept beside
// it with a .bak suffix. Keys missing from the file take their defaults.
// Load then makes sure the notes directory exists, falling back to the
// default one when it cannot be created.
func (s *Store) Load() (*Config, error) {
	defaults := Defaults(s.DocumentsDir)
	cfg, from, err := s.read(defaults)
	switch {
	case errors.Is(err, fs.ErrNotExist):
		slog.Info("config file not found, using defaults", "path", s.Path)
		cfg = defaults
		if err := s.Save(cfg); err != nil {
			return cfg, err
		}
	case err != nil:
		slog.Warn("config file unreadable, using defaults", "path", from, "err", err)
		if from == s.Path {
			if err := os.Rename(s.Path, s.Path+".bak"); err != nil {
				slog.Warn("could not back up config file", "err", err)
			}
		}
		cfg = defaults
		if err := s.Save(cfg); err != nil {
			return cfg, err
		}
	case from != s.Path:
		slog.Info("migrating config file", "from", from, "to", s.Path)
		if err := s.Save(cfg); err != nil {
			return cfg, err
		}
	}

	if err := os.MkdirAll(cfg.NotesDir, 0o755); err != nil {
		slog.Warn("could not create notes directory, using the default", "dir", cfg.NotesDir, "err", err)
		cfg.NotesDir = defaults.NotesDir
		if err := s.Save(cfg); err != nil {
			return cfg, err
		}
		if err := os.MkdirAll(cfg.NotesDir, 0o755); err != nil {
			return cfg, fmt.Errorf("create notes directory: %w", err)
		}
	}
	return cfg, nil
}

// read parses the first settings file that exists, over the defaults, and
// returns the path it came from.
func (s *Store) read(defaults *Config) (*Config, string, error) {
	for _, path := range append([]string{s.Path}, s.Legacy...) {
		data, err := os.ReadFile(path)
		if errors.Is(err, fs.ErrNotExist) {
			continue
		}
		if err != nil {
			return nil, path, err
		}
		cfg, err := parse(data, defaults)
		return cfg, path, err
	}
	return nil, "", fs.ErrNotExist
}

// parse reads settings from data over a copy of defaults.
func parse(data []byte, defaults *Config) (*Config, error) {
	cfg := *defaults
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, err
	}
	var raw map[string]json.RawMessage
	if err := json.Unmarshal(data, &raw); err != nil {
		return nil, err
	}
	// Older versions had a dark_mode switch instead of the theme.
	if dm, ok := raw["dark_mode"]; ok {
		if _, hasTheme := raw["theme"]; !hasTheme {
			var dark bool
			if json.Unmarshal(dm, &dark) == nil {
				cfg.Theme = AppearanceLight
				if dark {
					cfg.Theme = AppearanceDark
				}
			}
		}
		delete(raw, "dark_mode")
	}
	for _, k := range knownKeys {
		delete(raw, k)
	}
	if len(raw) > 0 {
		cfg.extra = raw
	}
	return &cfg, nil
}

// Save writes the settings, replacing the file atomically.
func (s *Store) Save(cfg *Config) error {
	data, err := cfg.MarshalJSON()
	if err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(s.Path), 0o755); err != nil {
		return fmt.Errorf("save config: %w", err)
	}
	tmp, err := os.CreateTemp(filepath.Dir(s.Path), filepath.Base(s.Path)+".*.tmp")
	if err != nil {
		return fmt.Errorf("save config: %w", err)
	}
	defer os.Remove(tmp.Name())
	if _, err := tmp.Write(data); err != nil {
		tmp.Close()
		return fmt.Errorf("save config: %w", err)
	}
	if err := tmp.Close(); err != nil {
		return fmt.Errorf("save config: %w", err)
	}
	if err := os.Rename(tmp.Name(), s.Path); err != nil {
		return fmt.Errorf("save config: %w", err)
	}
	return nil
}

// knownKeys are the JSON keys of Config.
var knownKeys = []string{
	"windowsize", "persist_window_size", "notes_dir",
	"show_markdown_syntax_highlighting", "theme", "sorting_method",
	"activate_row_on_select", "use_file_extension", "editor_color_scheme",
	"recurse_subfolders", "font_size", "use_custom_font", "custom_font",
	"last_opened_file", "editor_show_line_numbers",
	"editor_highlight_current_line", "editor_vim_mode",
}

// MarshalJSON encodes the settings together with the keys this version
// does not know.
func (c *Config) MarshalJSON() ([]byte, error) {
	type plain Config
	data, err := json.Marshal((*plain)(c))
	if err != nil || len(c.extra) == 0 {
		return data, err
	}
	merged := make(map[string]json.RawMessage, len(c.extra)+len(knownKeys))
	for k, v := range c.extra {
		merged[k] = v
	}
	var own map[string]json.RawMessage
	if err := json.Unmarshal(data, &own); err != nil {
		return nil, err
	}
	for k, v := range own {
		merged[k] = v
	}
	return json.Marshal(merged)
}

// SetFontSize sets the editor's font size, clamped to the allowed range.
func (c *Config) SetFontSize(size int) {
	c.FontSize = min(max(size, MinFontSize), MaxFontSize)
}

// SetLastOpenedFile records the note open when Noty quit; "" clears it.
func (c *Config) SetLastOpenedFile(path string) {
	if path == "" {
		c.LastOpenedFile = nil
		return
	}
	c.LastOpenedFile = &path
}
