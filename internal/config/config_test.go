package config

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func newStore(t *testing.T) *Store {
	t.Helper()
	dir := t.TempDir()
	return &Store{
		Path:         filepath.Join(dir, "config", "com.dagimg.noty.json"),
		DocumentsDir: filepath.Join(dir, "Documents"),
	}
}

func writeJSON(t *testing.T, path, data string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(data), 0o644); err != nil {
		t.Fatal(err)
	}
}

func readRaw(t *testing.T, path string) map[string]any {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var m map[string]any
	if err := json.Unmarshal(data, &m); err != nil {
		t.Fatalf("saved config is not JSON: %v\n%s", err, data)
	}
	return m
}

func TestLoadMissingWritesDefaults(t *testing.T) {
	s := newStore(t)
	cfg, err := s.Load()
	if err != nil {
		t.Fatal(err)
	}
	want := Defaults(s.DocumentsDir)
	if cfg.NotesDir != want.NotesDir || cfg.Theme != "system" || cfg.FontSize != 12 || cfg.EditorColorScheme != "noty-classic" {
		t.Errorf("got %+v, want defaults", cfg)
	}
	if info, err := os.Stat(cfg.NotesDir); err != nil || !info.IsDir() {
		t.Errorf("notes directory not created: %v", err)
	}
	raw := readRaw(t, s.Path)
	if raw["notes_dir"] != want.NotesDir || raw["last_opened_file"] != nil {
		t.Errorf("saved %v", raw)
	}
	if _, ok := raw["last_opened_file"]; !ok {
		t.Error("last_opened_file not saved as null")
	}
}

// A file the Python version wrote, with json.dumps, some keys missing and
// one this version does not know.
func TestLoadPythonFile(t *testing.T) {
	s := newStore(t)
	notes := filepath.Join(t.TempDir(), "my notes")
	writeJSON(t, s.Path, `{"windowsize": {"width": 640, "height": 900}, "persist_window_size": false, `+
		`"notes_dir": "`+notes+`", "theme": "dark", "sorting_method": "date_modified", `+
		`"font_size": 15, "last_opened_file": "`+notes+`/todo", "future_key": [1, 2]}`)
	cfg, err := s.Load()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.WindowSize != (WindowSize{640, 900}) || cfg.PersistWindowSize || cfg.NotesDir != notes ||
		cfg.Theme != "dark" || cfg.SortingMethod != "date_modified" || cfg.FontSize != 15 ||
		cfg.LastOpenedFile == nil || *cfg.LastOpenedFile != notes+"/todo" {
		t.Errorf("got %+v", cfg)
	}
	// Missing keys take their defaults.
	if !cfg.EditorShowLineNumbers || cfg.CustomFont != "Monospace 12" || cfg.EditorColorScheme != "noty-classic" {
		t.Errorf("missing keys did not take defaults: %+v", cfg)
	}

	cfg.SetFontSize(99)
	if err := s.Save(cfg); err != nil {
		t.Fatal(err)
	}
	raw := readRaw(t, s.Path)
	if raw["font_size"] != float64(MaxFontSize) {
		t.Errorf("font_size %v, want %d", raw["font_size"], MaxFontSize)
	}
	if fk, ok := raw["future_key"].([]any); !ok || len(fk) != 2 {
		t.Errorf("unknown key not kept: %v", raw["future_key"])
	}
	if raw["editor_show_line_numbers"] != true {
		t.Errorf("defaults not written: %v", raw)
	}
}

func TestLoadDarkModeMigration(t *testing.T) {
	for _, tc := range []struct {
		json, want string
	}{
		{`{"dark_mode": true}`, "dark"},
		{`{"dark_mode": false}`, "light"},
		{`{"dark_mode": true, "theme": "system"}`, "system"},
	} {
		s := newStore(t)
		writeJSON(t, s.Path, tc.json)
		cfg, err := s.Load()
		if err != nil {
			t.Fatal(err)
		}
		if cfg.Theme != tc.want {
			t.Errorf("%s: theme %q, want %q", tc.json, cfg.Theme, tc.want)
		}
		if err := s.Save(cfg); err != nil {
			t.Fatal(err)
		}
		if _, ok := readRaw(t, s.Path)["dark_mode"]; ok {
			t.Errorf("%s: dark_mode still saved", tc.json)
		}
	}
}

func TestLoadCorruptFileIsBackedUp(t *testing.T) {
	s := newStore(t)
	writeJSON(t, s.Path, `{"theme": "dark",`)
	cfg, err := s.Load()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Theme != "system" {
		t.Errorf("theme %q, want the default", cfg.Theme)
	}
	if data, err := os.ReadFile(s.Path + ".bak"); err != nil || string(data) != `{"theme": "dark",` {
		t.Errorf("backup %q, %v", data, err)
	}
	readRaw(t, s.Path)
}

func TestLoadLegacyPath(t *testing.T) {
	s := newStore(t)
	legacy := filepath.Join(t.TempDir(), "flatpak", "com.dagimg.noty.json")
	s.Legacy = []string{filepath.Join(t.TempDir(), "missing.json"), legacy}
	writeJSON(t, legacy, `{"theme": "light", "font_size": 20}`)
	cfg, err := s.Load()
	if err != nil {
		t.Fatal(err)
	}
	if cfg.Theme != "light" || cfg.FontSize != 20 {
		t.Errorf("legacy settings not read: %+v", cfg)
	}
	if raw := readRaw(t, s.Path); raw["theme"] != "light" {
		t.Errorf("not migrated to the new path: %v", raw)
	}
	if _, err := os.Stat(legacy); err != nil {
		t.Errorf("legacy file removed: %v", err)
	}
}

func TestLoadUncreatableNotesDirFallsBack(t *testing.T) {
	s := newStore(t)
	blocker := filepath.Join(t.TempDir(), "file")
	writeJSON(t, blocker, "")
	writeJSON(t, s.Path, `{"notes_dir": "`+filepath.Join(blocker, "notes")+`"}`)
	cfg, err := s.Load()
	if err != nil {
		t.Fatal(err)
	}
	if want := Defaults(s.DocumentsDir).NotesDir; cfg.NotesDir != want {
		t.Errorf("notes dir %q, want %q", cfg.NotesDir, want)
	}
	if readRaw(t, s.Path)["notes_dir"] != cfg.NotesDir {
		t.Error("fallback notes dir not saved")
	}
}

func TestSetLastOpenedFile(t *testing.T) {
	cfg := Defaults("/docs")
	cfg.SetLastOpenedFile("/docs/Noties/a")
	if cfg.LastOpenedFile == nil || *cfg.LastOpenedFile != "/docs/Noties/a" {
		t.Errorf("got %v", cfg.LastOpenedFile)
	}
	cfg.SetLastOpenedFile("")
	if cfg.LastOpenedFile != nil {
		t.Errorf("not cleared: %v", *cfg.LastOpenedFile)
	}
}

func TestPaths(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	t.Setenv("XDG_CONFIG_HOME", "")
	if got, want := Path("com.dagimg.noty"), filepath.Join(home, ".config", "com.dagimg.noty.json"); got != want {
		t.Errorf("Path %q, want %q", got, want)
	}
	if got := LegacyPaths("com.dagimg.noty"); len(got) != 1 ||
		got[0] != filepath.Join(home, ".var", "app", "com.dagimg.noty", "config", "com.dagimg.noty.json") {
		t.Errorf("LegacyPaths %q", got)
	}

	xdg := filepath.Join(home, "xdg")
	t.Setenv("XDG_CONFIG_HOME", xdg)
	if got := Path("id"); got != filepath.Join(xdg, "id.json") {
		t.Errorf("Path with XDG_CONFIG_HOME %q", got)
	}
	if got := LegacyPaths("id"); len(got) != 2 || got[0] != filepath.Join(home, ".config", "id.json") {
		t.Errorf("LegacyPaths with XDG_CONFIG_HOME %q", got)
	}
}

func TestDocumentsDir(t *testing.T) {
	home := t.TempDir()
	t.Setenv("HOME", home)
	t.Setenv("XDG_CONFIG_HOME", "")
	if got := DocumentsDir(); got != filepath.Join(home, "Documents") {
		t.Errorf("without user-dirs.dirs: %q", got)
	}
	for _, tc := range []struct{ line, want string }{
		{`XDG_DOCUMENTS_DIR="$HOME/Dokumente"`, filepath.Join(home, "Dokumente")},
		{`XDG_DOCUMENTS_DIR="/srv/docs"`, "/srv/docs"},
		{`XDG_DOCUMENTS_DIR="$HOME/"`, filepath.Join(home, "Documents")},
		{`# XDG_DOCUMENTS_DIR="/nope"`, filepath.Join(home, "Documents")},
	} {
		writeJSON(t, filepath.Join(home, ".config", "user-dirs.dirs"),
			"XDG_DESKTOP_DIR=\"$HOME/Desktop\"\n"+tc.line+"\n")
		if got := DocumentsDir(); got != tc.want {
			t.Errorf("%s: %q, want %q", tc.line, got, tc.want)
		}
	}
}
