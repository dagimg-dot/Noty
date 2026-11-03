package theme

import (
	"cmp"
	"embed"
	"errors"
	"fmt"
	"io/fs"
	"log/slog"
	"os"
	"path/filepath"
	"slices"
	"strings"
)

//go:embed themes/*.toml
var bundled embed.FS

// Problem is something wrong with a theme file.
type Problem struct {
	// Path is the file; "bundled:<name>" for bundled themes.
	Path string
	// Message says what is wrong.
	Message string
	// Skipped is set when the theme could not be used at all.
	Skipped bool
}

func (p Problem) String() string { return p.Path + ": " + p.Message }

// DB is the set of themes: the bundled ones and those of the theme
// directories. It is not safe for concurrent use.
type DB struct {
	dirs     []string
	themes   []*Theme
	byID     map[string]*Theme
	problems []Problem
}

// NewDB returns a theme database that reads themes from dirs, in order of
// precedence, after loading them.
func NewDB(dirs ...string) *DB {
	db := &DB{dirs: dirs}
	db.Load()
	return db
}

// UserDir returns the directory of the user's themes:
// $XDG_DATA_HOME/noty/themes, ~/.local/share/noty/themes without it.
func UserDir() string {
	data := os.Getenv("XDG_DATA_HOME")
	if !filepath.IsAbs(data) {
		home, _ := os.UserHomeDir()
		data = filepath.Join(home, ".local", "share")
	}
	return filepath.Join(data, "noty", "themes")
}

// DefaultDirs returns the theme directories: the user's, then
// noty/themes in each of $XDG_DATA_DIRS.
func DefaultDirs() []string {
	dirs := []string{UserDir()}
	sys := os.Getenv("XDG_DATA_DIRS")
	if sys == "" {
		sys = "/usr/local/share:/usr/share"
	}
	for _, d := range filepath.SplitList(sys) {
		if p := filepath.Join(d, "noty", "themes"); filepath.IsAbs(d) && !slices.Contains(dirs, p) {
			dirs = append(dirs, p)
		}
	}
	return dirs
}

// Dirs returns the directories the database reads themes from, to watch
// them for changes.
func (db *DB) Dirs() []string { return db.dirs }

// Load reads the themes again. The roots come first and cannot be
// replaced; then the first theme of an ID wins, of the directories in
// order and then of the bundled themes, so a user's theme can replace a
// bundled one of the same name.
func (db *DB) Load() {
	db.themes, db.byID, db.problems = nil, map[string]*Theme{}, nil

	var roots, others []*Theme
	for _, e := range mustReadDir(bundled, "themes") {
		name := e.Name()
		data, err := bundled.ReadFile("themes/" + name)
		if err != nil {
			panic(err) // embedded
		}
		t, warnings, err := Parse(idFromPath(name), "", data)
		path := "bundled:" + name
		if err != nil {
			db.problem(path, err.Error(), true)
			continue
		}
		for _, w := range warnings {
			db.problem(path, w, false)
		}
		if t.ID == RootDark || t.ID == RootLight {
			roots = append(roots, t)
		} else {
			others = append(others, t)
		}
	}
	for _, t := range roots {
		db.add(t)
	}
	for _, dir := range db.dirs {
		entries, err := os.ReadDir(dir)
		if err != nil {
			if !errors.Is(err, fs.ErrNotExist) {
				slog.Warn("reading theme directory", "dir", dir, "err", err)
			}
			continue
		}
		for _, e := range entries {
			if e.IsDir() || !strings.EqualFold(filepath.Ext(e.Name()), ".toml") {
				continue
			}
			path := filepath.Join(dir, e.Name())
			id := idFromPath(path)
			if db.byID[id] != nil {
				if id == RootDark || id == RootLight {
					db.problem(path, "the ID "+id+" is reserved", true)
				}
				continue
			}
			t, warnings, err := ReadFile(path)
			if err != nil {
				db.problem(path, err.Error(), true)
				continue
			}
			for _, w := range warnings {
				db.problem(path, w, false)
			}
			db.add(t)
		}
	}
	for _, t := range others {
		if db.byID[t.ID] == nil {
			db.add(t)
		}
	}

	for _, t := range db.themes {
		db.link(t)
	}
	for _, t := range db.themes {
		for _, p := range t.Check() {
			db.problem(t.source(), p, false)
		}
	}
	slices.SortStableFunc(db.themes, func(a, b *Theme) int {
		return cmp.Compare(strings.ToLower(a.Name), strings.ToLower(b.Name))
	})
	for _, p := range db.problems {
		if p.Skipped {
			slog.Warn("theme skipped", "path", p.Path, "problem", p.Message)
		} else {
			slog.Debug("theme problem", "path", p.Path, "problem", p.Message)
		}
	}
}

func (db *DB) add(t *Theme) {
	db.themes = append(db.themes, t)
	db.byID[t.ID] = t
}

func (db *DB) problem(path, msg string, skipped bool) {
	db.problems = append(db.problems, Problem{Path: path, Message: msg, Skipped: skipped})
}

// link sets the theme a theme inherits: the one it names, else the root
// of its variant. A chain that would come back to the theme falls back to
// the root too.
func (db *DB) link(t *Theme) {
	if t.ID == RootDark || t.ID == RootLight {
		return
	}
	root := db.byID[RootDark]
	if t.Variant == Light {
		root = db.byID[RootLight]
	}
	parentID := cmp.Or(t.Inherits, root.ID)
	parent := db.byID[parentID]
	if parent == nil {
		db.problem(t.source(), fmt.Sprintf("inherits %q, which does not exist; using %s", parentID, root.ID), false)
		parent = root
	}
	seen := map[*Theme]bool{}
	for p := parent; p != nil && !seen[p]; p = db.byID[p.Inherits] {
		if p == t {
			db.problem(t.source(), fmt.Sprintf("inherits %q, which inherits it back; using %s", parentID, root.ID), false)
			parent = root
			break
		}
		seen[p] = true
	}
	t.parent = parent
}

func (t *Theme) source() string {
	if t.Path == "" {
		return "bundled:" + t.ID + ".toml"
	}
	return t.Path
}

// Themes returns the themes, sorted by name.
func (db *DB) Themes() []*Theme { return db.themes }

// Problems returns what was wrong with the theme files at the last Load.
func (db *DB) Problems() []Problem { return db.problems }

// Get returns the theme of the ID, or nil.
func (db *DB) Get(id string) *Theme { return db.byID[id] }

// renamed maps the IDs of GtkSourceView schemes of the Python version
// that became themes of other IDs.
var renamed = map[string]string{"noty-classic": RootLight}

// Pick returns the theme of the ID, else the root of the variant asked
// for.
func (db *DB) Pick(id string, dark bool) *Theme {
	if t := db.byID[id]; t != nil {
		return t
	}
	if t := db.byID[renamed[id]]; t != nil {
		return t
	}
	if dark {
		return db.byID[RootDark]
	}
	return db.byID[RootLight]
}

// ReadFile parses the theme file at path.
func ReadFile(path string) (*Theme, []string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, nil, err
	}
	return Parse(idFromPath(path), path, data)
}

// Import validates the theme file at src and copies it into the user's
// theme directory, returning its ID. It refuses a file that cannot be
// parsed, the IDs of the roots, and, unless replace is set, an ID the
// user's directory already has.
func Import(src string, replace bool) (string, error) {
	return importTo(UserDir(), src, replace)
}

func importTo(dir, src string, replace bool) (string, error) {
	if !strings.EqualFold(filepath.Ext(src), ".toml") {
		return "", errors.New("a theme file has the .toml extension")
	}
	data, err := os.ReadFile(src)
	if err != nil {
		return "", err
	}
	id := idFromPath(src)
	if id == RootDark || id == RootLight {
		return "", fmt.Errorf("the ID %s is reserved: rename the file", id)
	}
	if _, _, err := Parse(id, src, data); err != nil {
		return "", fmt.Errorf("invalid theme: %w", err)
	}
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return "", err
	}
	dst := filepath.Join(dir, id+".toml")
	flags := os.O_WRONLY | os.O_CREATE | os.O_TRUNC
	if !replace {
		flags |= os.O_EXCL
	}
	f, err := os.OpenFile(dst, flags, 0o644)
	if errors.Is(err, fs.ErrExist) {
		return "", fmt.Errorf("a theme with the ID %s exists already", id)
	}
	if err != nil {
		return "", err
	}
	if _, err := f.Write(data); err != nil {
		f.Close()
		return "", err
	}
	return id, f.Close()
}

func mustReadDir(fsys embed.FS, dir string) []fs.DirEntry {
	entries, err := fsys.ReadDir(dir)
	if err != nil {
		panic(err)
	}
	return entries
}
