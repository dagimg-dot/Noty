package notes

import (
	"errors"
	"os"
	"path/filepath"
	"slices"
	"testing"
	"time"
)

func write(t *testing.T, path, content string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
}

func names(ns []*Note) []string {
	out := make([]string, len(ns))
	for i, n := range ns {
		out[i] = n.Name
	}
	return out
}

func sortedNames(ns []*Note) []string {
	out := names(ns)
	slices.Sort(out)
	return out
}

// setTime sets a file's modification time, as another program writing it
// at that time would.
func setTime(t *testing.T, path string, at time.Time) {
	t.Helper()
	if err := os.Chtimes(path, at, at); err != nil {
		t.Fatal(err)
	}
}

func newDir(t *testing.T) string {
	t.Helper()
	dir := t.TempDir()
	for _, f := range []string{
		"todo", "Ideas.md", "readme.MD", "image.png", "notes.txt", ".hidden",
		"sub/nested", "sub/deeper/deep.md", ".git/config",
	} {
		write(t, filepath.Join(dir, f), "content of "+f)
	}
	return dir
}

func TestScan(t *testing.T) {
	dir := newDir(t)
	s, err := New(Options{Dir: dir})
	if err != nil {
		t.Fatal(err)
	}
	if got, want := sortedNames(s.Notes()), []string{"Ideas", "readme", "todo"}; !slices.Equal(got, want) {
		t.Errorf("flat scan %q, want %q", got, want)
	}

	if err := s.SetOptions(Options{Dir: dir, Recurse: true}); err != nil {
		t.Fatal(err)
	}
	if got, want := sortedNames(s.Notes()), []string{"Ideas", "deep", "nested", "readme", "todo"}; !slices.Equal(got, want) {
		t.Errorf("recursive scan %q, want %q", got, want)
	}
	if n := s.FindByName("NESTED"); n == nil || n.Path != filepath.Join(dir, "sub", "nested") {
		t.Errorf("FindByName: %+v", n)
	}
}

func TestScanSymlinkLoop(t *testing.T) {
	dir := newDir(t)
	if err := os.Symlink(dir, filepath.Join(dir, "sub", "loop")); err != nil {
		t.Skip("no symbolic links:", err)
	}
	s, err := New(Options{Dir: dir, Recurse: true})
	if err != nil {
		t.Fatal(err)
	}
	if got := len(s.Notes()); got != 5 {
		t.Errorf("%d notes, want 5: %q", got, names(s.Notes()))
	}
}

func TestScanMissingDir(t *testing.T) {
	if _, err := New(Options{Dir: filepath.Join(t.TempDir(), "missing")}); err == nil {
		t.Error("no error for a missing directory")
	}
}

func TestIsNoteFile(t *testing.T) {
	for name, want := range map[string]bool{
		"todo": true, "a.md": true, "a.Md": true, "a.b.md": true,
		"a.txt": false, ".md": false, ".hidden": false, "v1.2 notes": false,
	} {
		if got := IsNoteFile(name); got != want {
			t.Errorf("IsNoteFile(%q) = %v", name, got)
		}
	}
}

func TestSorted(t *testing.T) {
	dir := t.TempDir()
	base := time.Date(2025, 1, 1, 0, 0, 0, 0, time.UTC)
	for i, n := range []string{"banana", "Apple", "cherry pie", "apricot"} {
		p := filepath.Join(dir, n)
		write(t, p, "")
		setTime(t, p, base.Add(time.Duration(i)*time.Hour))
	}
	s, err := New(Options{Dir: dir})
	if err != nil {
		t.Fatal(err)
	}
	if got, want := names(s.Sorted("", SortByName)), []string{"Apple", "apricot", "banana", "cherry pie"}; !slices.Equal(got, want) {
		t.Errorf("by name %q, want %q", got, want)
	}
	if got, want := names(s.Sorted("", SortByDateModified)), []string{"apricot", "cherry pie", "Apple", "banana"}; !slices.Equal(got, want) {
		t.Errorf("by date %q, want %q", got, want)
	}
	if got, want := names(s.Sorted("  AP ", SortByName)), []string{"Apple", "apricot"}; !slices.Equal(got, want) {
		t.Errorf("filtered %q, want %q", got, want)
	}
	if got := s.Sorted("zzz", SortByName); len(got) != 0 {
		t.Errorf("no match gave %q", names(got))
	}
}

func TestCreate(t *testing.T) {
	dir := t.TempDir()
	s, err := New(Options{Dir: dir})
	if err != nil {
		t.Fatal(err)
	}
	n, err := s.Create("  shopping list ")
	if err != nil {
		t.Fatal(err)
	}
	if n.Name != "shopping list" || n.Path != filepath.Join(dir, "shopping list") {
		t.Errorf("created %+v", n)
	}
	if _, err := s.Create("shopping list"); !errors.Is(err, ErrExists) {
		t.Errorf("duplicate: %v", err)
	}
	for _, bad := range []string{"", "  ", "a/b", ".hidden"} {
		if _, err := s.Create(bad); !errors.Is(err, ErrInvalidName) {
			t.Errorf("Create(%q): %v", bad, err)
		}
	}

	s.SetOptions(Options{Dir: dir, UseExtension: true})
	n, err = s.Create("journal")
	if err != nil {
		t.Fatal(err)
	}
	if n.Name != "journal" || filepath.Base(n.Path) != "journal.md" {
		t.Errorf("created with extension %+v", n)
	}
	if got := sortedNames(s.Notes()); !slices.Equal(got, []string{"journal", "shopping list"}) {
		t.Errorf("notes %q", got)
	}
}

func TestRename(t *testing.T) {
	dir := t.TempDir()
	write(t, filepath.Join(dir, "old"), "text")
	write(t, filepath.Join(dir, "taken"), "")
	s, _ := New(Options{Dir: dir})
	old := filepath.Join(dir, "old")
	if _, err := s.Open(old); err != nil {
		t.Fatal(err)
	}

	if _, err := s.Rename(old, "taken"); !errors.Is(err, ErrExists) {
		t.Errorf("onto an existing note: %v", err)
	}
	if _, err := s.Rename(filepath.Join(dir, "nope"), "x"); !errors.Is(err, ErrNotFound) {
		t.Errorf("missing note: %v", err)
	}
	n, err := s.Rename(old, "new")
	if err != nil {
		t.Fatal(err)
	}
	if n.Name != "new" || n.Path != filepath.Join(dir, "new") || s.OpenPath() != n.Path {
		t.Errorf("renamed %+v, open %q", n, s.OpenPath())
	}
	if _, err := os.Stat(old); !os.IsNotExist(err) {
		t.Errorf("old file still there: %v", err)
	}
	if s.ChangedExternally() {
		t.Error("rename looks like an external change")
	}
	// Changing only the case is allowed.
	if n, err := s.Rename(n.Path, "New"); err != nil || n.Name != "New" {
		t.Errorf("case change: %+v, %v", n, err)
	}
}

func TestDelete(t *testing.T) {
	dir := t.TempDir()
	write(t, filepath.Join(dir, "a"), "")
	write(t, filepath.Join(dir, "b"), "")
	s, _ := New(Options{Dir: dir})
	a := filepath.Join(dir, "a")
	s.Open(a)
	if err := s.Delete(a); err != nil {
		t.Fatal(err)
	}
	if s.OpenPath() != "" {
		t.Error("deleted note still open")
	}
	if got := names(s.Notes()); !slices.Equal(got, []string{"b"}) {
		t.Errorf("notes %q", got)
	}
	if err := s.Delete(a); !errors.Is(err, ErrNotFound) {
		t.Errorf("deleting twice: %v", err)
	}
}

func TestOpenSave(t *testing.T) {
	dir := t.TempDir()
	p := filepath.Join(dir, "note")
	write(t, p, "first")
	setTime(t, p, time.Now().Add(-time.Hour))
	s, _ := New(Options{Dir: dir})

	if err := s.Save("ignored", false); err != nil {
		t.Errorf("saving with no open note: %v", err)
	}
	text, err := s.Open(p)
	if err != nil || text != "first" {
		t.Fatalf("Open: %q, %v", text, err)
	}
	if err := s.Save("second", false); err != nil {
		t.Fatal(err)
	}
	if data, _ := os.ReadFile(p); string(data) != "second" {
		t.Errorf("file has %q", data)
	}
	if n := s.Find(p); !n.Modified.Equal(modTime(p)) {
		t.Errorf("note's time %v, file's %v", n.Modified, modTime(p))
	}
	if entries, _ := os.ReadDir(dir); len(entries) != 1 {
		t.Errorf("temporary files left: %v", entries)
	}

	// Saving the same content does not touch the file.
	before := modTime(p)
	if err := s.Save("second", false); err != nil || !modTime(p).Equal(before) {
		t.Errorf("unchanged save: %v, time %v → %v", err, before, modTime(p))
	}
}

func TestExternalChange(t *testing.T) {
	dir := t.TempDir()
	p := filepath.Join(dir, "note")
	write(t, p, "mine")
	setTime(t, p, time.Now().Add(-time.Hour))
	s, _ := New(Options{Dir: dir})
	s.Open(p)
	if s.ChangedExternally() {
		t.Fatal("changed right after opening")
	}

	write(t, p, "theirs")
	setTime(t, p, time.Now())
	if !s.ChangedExternally() {
		t.Fatal("external write not noticed")
	}
	if err := s.Save("mine, edited", false); !errors.Is(err, ErrChangedExternally) {
		t.Errorf("Save over an external change: %v", err)
	}
	if data, _ := os.ReadFile(p); string(data) != "theirs" {
		t.Errorf("external change overwritten: %q", data)
	}

	// Reloading accepts the change.
	if text, _ := s.Open(p); text != "theirs" || s.ChangedExternally() {
		t.Errorf("reload: %q, changed %v", text, s.ChangedExternally())
	}

	// Overwriting keeps Noty's version.
	write(t, p, "theirs again")
	setTime(t, p, time.Now().Add(time.Minute))
	if err := s.Save("mine wins", true); err != nil {
		t.Fatal(err)
	}
	if data, _ := os.ReadFile(p); string(data) != "mine wins" || s.ChangedExternally() {
		t.Errorf("overwrite: %q, changed %v", data, s.ChangedExternally())
	}
}

func TestSaveKeepsModeAndSymlink(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(t.TempDir(), "real")
	write(t, target, "x")
	if err := os.Chmod(target, 0o600); err != nil {
		t.Fatal(err)
	}
	link := filepath.Join(dir, "linked")
	if err := os.Symlink(target, link); err != nil {
		t.Skip("no symbolic links:", err)
	}
	s, _ := New(Options{Dir: dir})
	s.Open(link)
	if err := s.Save("y", false); err != nil {
		t.Fatal(err)
	}
	if fi, err := os.Lstat(link); err != nil || fi.Mode()&os.ModeSymlink == 0 {
		t.Errorf("link replaced: %v", err)
	}
	info, _ := os.Stat(target)
	if data, _ := os.ReadFile(target); string(data) != "y" || info.Mode().Perm() != 0o600 {
		t.Errorf("target %q, mode %v", data, info.Mode().Perm())
	}
}
