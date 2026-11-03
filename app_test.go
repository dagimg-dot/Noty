package main

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/dagimg-dot/noty/internal/config"
	"github.com/dagimg-dot/noty/internal/notes"
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
	a := newApp(&config.Store{Path: filepath.Join(t.TempDir(), "c.json")}, cfg, store)
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
