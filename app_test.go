package main

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/dagimg-dot/noty/internal/config"
	"github.com/dagimg-dot/noty/internal/notes"
	"github.com/dagimg-dot/noty/internal/theme"
	"github.com/egoist/mygo/ui"
)

func TestViewListsNotes(t *testing.T) {
	dir := t.TempDir()
	for _, n := range []string{"todo", "ideas.md", "photo.png"} {
		if err := os.WriteFile(filepath.Join(dir, n), nil, 0o644); err != nil {
			t.Fatal(err)
		}
	}
	cfg := config.Defaults(t.TempDir())
	cfg.NotesDir = dir
	store, err := notes.New(notes.Options{Dir: dir})
	if err != nil {
		t.Fatal(err)
	}
	a := newApp(&config.Store{Path: filepath.Join(t.TempDir(), "c.json")}, cfg, store, theme.NewDB())
	tt := ui.NewTester(a.view, 500, 800)
	for _, want := range []string{"ideas", "todo"} {
		if !tt.HasText(want) {
			t.Errorf("%q not shown; texts %q", want, tt.Texts())
		}
	}
	if tt.HasText("photo") {
		t.Errorf("non-note shown; texts %q", tt.Texts())
	}
}

func TestCurrentTheme(t *testing.T) {
	cfg := config.Defaults(t.TempDir())
	store, err := notes.New(notes.Options{Dir: t.TempDir()})
	if err != nil {
		t.Fatal(err)
	}
	a := newApp(nil, cfg, store, theme.NewDB())
	for _, tc := range []struct {
		scheme string
		dark   bool
		want   string
	}{
		// The roots, and the Python version's default, follow the appearance.
		{"noty-classic", true, theme.RootDark},
		{"noty-classic", false, theme.RootLight},
		{theme.RootLight, true, theme.RootDark},
		{theme.RootDark, false, theme.RootLight},
		{"missing", true, theme.RootDark},
		// Other themes show as chosen.
		{"nord", false, "nord"},
		{"tokyo-night", true, "tokyo-night"},
	} {
		cfg.EditorColorScheme = tc.scheme
		if got := a.currentTheme(tc.dark).ID; got != tc.want {
			t.Errorf("%s, dark %v: %s, want %s", tc.scheme, tc.dark, got, tc.want)
		}
	}
	// The window takes the theme's colors.
	cfg.EditorColorScheme = "nord"
	var bg ui.Color
	ui.NewTester(func(c *ui.Context) { a.view(c); bg = c.Theme().Background }, 400, 300)
	if want := a.themes.Get("nord").Color(theme.CoreBackground); bg.R != want.R || bg.G != want.G || bg.B != want.B {
		t.Errorf("window background %v, want %v", bg, want)
	}
}
