// Package notes manages the notes directory: the notes in it, and the
// note open in the editor.
//
// A note is a plain file in the notes directory (or one of its
// subdirectories, when recursing) whose name has no extension, or the .md
// extension. Its name is the file name without the extension.
package notes

import (
	"cmp"
	"errors"
	"fmt"
	"io/fs"
	"log/slog"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"time"
)

// Errors of the Store.
var (
	// ErrExists is returned when a note of the name exists already.
	ErrExists = errors.New("a note with this name already exists")
	// ErrInvalidName is returned for names a note cannot have.
	ErrInvalidName = errors.New("invalid note name")
	// ErrNotFound is returned for a path that is not a note of the store.
	ErrNotFound = errors.New("note not found")
	// ErrChangedExternally is returned by Save when the open note's file
	// changed on disk since Noty last read or wrote it.
	ErrChangedExternally = errors.New("note changed externally")
)

// Note is a note file.
type Note struct {
	// Name is the file name without its extension.
	Name string
	// Path is the file's path.
	Path string
	// Modified is when the file last changed.
	Modified time.Time
}

// Options configure a Store.
type Options struct {
	// Dir is the notes directory.
	Dir string
	// Recurse includes notes in subdirectories, other than hidden ones.
	Recurse bool
	// UseExtension gives new and renamed notes the .md extension.
	UseExtension bool
}

// Store lists the notes of a directory and tracks the open note. It is
// not safe for concurrent use: use it from the UI thread.
type Store struct {
	opts  Options
	notes []*Note

	openPath string
	// seen is the modification time of the open note's file when Noty
	// last read or wrote it.
	seen time.Time
}

// New returns a store of the notes in opts.Dir, which it scans.
func New(opts Options) (*Store, error) {
	s := &Store{opts: opts}
	return s, s.Reload()
}

// Options returns the store's options.
func (s *Store) Options() Options { return s.opts }

// SetOptions changes the store's options and scans the notes again. When
// the directory changes the open note is closed.
func (s *Store) SetOptions(opts Options) error {
	if opts.Dir != s.opts.Dir {
		s.Close()
	}
	s.opts = opts
	return s.Reload()
}

// Notes returns the notes, in the order of the directory scan. The slice
// and the notes in it are the store's; do not change them.
func (s *Store) Notes() []*Note { return s.notes }

// Reload scans the notes directory again. It keeps the notes it found
// when some entries cannot be read, and returns an error only when the
// directory itself cannot be.
func (s *Store) Reload() error {
	s.notes = s.notes[:0]
	visited := map[string]bool{}
	err := s.scan(s.opts.Dir, visited)
	slog.Debug("scanned notes", "dir", s.opts.Dir, "count", len(s.notes))
	return err
}

func (s *Store) scan(dir string, visited map[string]bool) error {
	if real, err := filepath.EvalSymlinks(dir); err == nil {
		if visited[real] {
			return nil
		}
		visited[real] = true
	}
	entries, err := os.ReadDir(dir)
	if err != nil {
		return fmt.Errorf("read notes directory: %w", err)
	}
	for _, e := range entries {
		path := filepath.Join(dir, e.Name())
		info, err := os.Stat(path) // follows symbolic links
		if err != nil {
			slog.Debug("skipping unreadable entry", "path", path, "err", err)
			continue
		}
		if info.IsDir() {
			if s.opts.Recurse && !strings.HasPrefix(e.Name(), ".") {
				if err := s.scan(path, visited); err != nil {
					slog.Warn("skipping unreadable directory", "path", path, "err", err)
				}
			}
			continue
		}
		if info.Mode().IsRegular() && IsNoteFile(e.Name()) {
			s.notes = append(s.notes, &Note{Name: noteName(e.Name()), Path: path, Modified: info.ModTime()})
		}
	}
	return nil
}

// IsNoteFile reports whether a file of the name is a note: not hidden,
// and either without an extension or with .md, in any case.
func IsNoteFile(name string) bool {
	if strings.HasPrefix(name, ".") {
		return false
	}
	return !strings.Contains(name, ".") || strings.EqualFold(filepath.Ext(name), ".md")
}

func noteName(file string) string {
	if n := strings.TrimSuffix(file, filepath.Ext(file)); n != "" {
		return n
	}
	return file
}

// ValidateName checks a name for a new or renamed note, after trimming
// spaces around it, and returns the trimmed name.
func ValidateName(name string) (string, error) {
	name = strings.TrimSpace(name)
	switch {
	case name == "":
		return "", fmt.Errorf("%w: the name is empty", ErrInvalidName)
	case strings.ContainsAny(name, "/\x00"):
		return "", fmt.Errorf("%w: the name cannot contain a slash", ErrInvalidName)
	case strings.HasPrefix(name, "."):
		return "", fmt.Errorf("%w: the name cannot start with a dot", ErrInvalidName)
	}
	return name, nil
}

func (s *Store) fileName(name string) string {
	if s.opts.UseExtension {
		return name + ".md"
	}
	return name
}

// Find returns the note at path, or nil.
func (s *Store) Find(path string) *Note {
	for _, n := range s.notes {
		if n.Path == path {
			return n
		}
	}
	return nil
}

// FindByName returns the note of the name, ignoring case, or nil.
func (s *Store) FindByName(name string) *Note {
	for _, n := range s.notes {
		if strings.EqualFold(n.Name, name) {
			return n
		}
	}
	return nil
}

// Create makes an empty note of the name in the notes directory.
func (s *Store) Create(name string) (*Note, error) {
	name, err := ValidateName(name)
	if err != nil {
		return nil, err
	}
	path := filepath.Join(s.opts.Dir, s.fileName(name))
	f, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o644)
	if errors.Is(err, fs.ErrExist) {
		return nil, ErrExists
	}
	if err != nil {
		return nil, fmt.Errorf("create note: %w", err)
	}
	if err := f.Close(); err != nil {
		return nil, fmt.Errorf("create note: %w", err)
	}
	n := &Note{Name: noteName(filepath.Base(path)), Path: path, Modified: modTime(path)}
	s.notes = append(s.notes, n)
	return n, nil
}

// Delete removes the note at path. Deleting the open note closes it.
func (s *Store) Delete(path string) error {
	i := slices.IndexFunc(s.notes, func(n *Note) bool { return n.Path == path })
	if i < 0 {
		return ErrNotFound
	}
	if err := os.Remove(path); err != nil {
		return fmt.Errorf("delete note: %w", err)
	}
	s.notes = slices.Delete(s.notes, i, i+1)
	if s.openPath == path {
		s.Close()
	}
	return nil
}

// Rename gives the note at path a new name, keeping it in its directory.
// Renaming the open note keeps it open.
func (s *Store) Rename(path, newName string) (*Note, error) {
	n := s.Find(path)
	if n == nil {
		return nil, ErrNotFound
	}
	newName, err := ValidateName(newName)
	if err != nil {
		return nil, err
	}
	newPath := filepath.Join(filepath.Dir(path), s.fileName(newName))
	if newPath == path {
		return n, nil
	}
	// A different case of the same name is the same file on
	// case-insensitive file systems, so only refuse other files.
	if _, err := os.Lstat(newPath); err == nil && !strings.EqualFold(newPath, path) {
		return nil, ErrExists
	}
	if err := os.Rename(path, newPath); err != nil {
		return nil, fmt.Errorf("rename note: %w", err)
	}
	n.Path, n.Name, n.Modified = newPath, noteName(filepath.Base(newPath)), modTime(newPath)
	if s.openPath == path {
		s.openPath = newPath
		s.seen = n.Modified
	}
	return n, nil
}

// Sorted returns the notes whose names contain query, ignoring case and
// the spaces around it, sorted by method: SortByName ("name") from A to
// Z ignoring case, or SortByDateModified ("date_modified") newest first.
func (s *Store) Sorted(query, method string) []*Note {
	query = strings.ToLower(strings.TrimSpace(query))
	out := make([]*Note, 0, len(s.notes))
	for _, n := range s.notes {
		if query == "" || strings.Contains(strings.ToLower(n.Name), query) {
			out = append(out, n)
		}
	}
	switch method {
	case SortByDateModified:
		slices.SortStableFunc(out, func(a, b *Note) int { return b.Modified.Compare(a.Modified) })
	default:
		slices.SortStableFunc(out, func(a, b *Note) int {
			return cmp.Compare(strings.ToLower(a.Name), strings.ToLower(b.Name))
		})
	}
	return out
}

// Sorting methods of Sorted, the values of the sorting_method setting.
const (
	SortByName         = "name"
	SortByDateModified = "date_modified"
)

// OpenPath returns the path of the open note, or "".
func (s *Store) OpenPath() string { return s.openPath }

// Open reads the note at path and makes it the open note.
func (s *Store) Open(path string) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", fmt.Errorf("open note: %w", err)
	}
	s.openPath = path
	s.seen = modTime(path)
	return string(data), nil
}

// Close forgets the open note, without saving it.
func (s *Store) Close() {
	s.openPath = ""
	s.seen = time.Time{}
}

// ChangedExternally reports whether the open note's file changed on disk
// since Noty last read or wrote it.
func (s *Store) ChangedExternally() bool {
	if s.openPath == "" || s.seen.IsZero() {
		return false
	}
	info, err := os.Stat(s.openPath)
	if err != nil {
		return false
	}
	return !info.ModTime().Equal(s.seen)
}

// Save writes content to the open note. It returns ErrChangedExternally,
// and writes nothing, when the file changed on disk since Noty last read
// or wrote it, unless overwrite is set. Content equal to the file's is
// not written again.
func (s *Store) Save(content string, overwrite bool) error {
	if s.openPath == "" {
		return nil
	}
	if !overwrite && s.ChangedExternally() {
		return ErrChangedExternally
	}
	if old, err := os.ReadFile(s.openPath); err == nil && string(old) == content {
		s.seen = modTime(s.openPath)
		return nil
	}
	if err := writeFile(s.openPath, content); err != nil {
		return fmt.Errorf("save note: %w", err)
	}
	s.seen = modTime(s.openPath)
	if n := s.Find(s.openPath); n != nil {
		n.Modified = s.seen
	}
	return nil
}

// writeFile replaces the file at path with content, keeping its mode,
// through a temporary file so a crash cannot leave it half written. A
// symbolic link is written through, to the file it points at.
func writeFile(path, content string) error {
	if real, err := filepath.EvalSymlinks(path); err == nil {
		path = real
	}
	mode := fs.FileMode(0o644)
	if info, err := os.Stat(path); err == nil {
		mode = info.Mode().Perm()
	}
	tmp, err := os.CreateTemp(filepath.Dir(path), "."+filepath.Base(path)+".*.tmp")
	if err != nil {
		return err
	}
	defer os.Remove(tmp.Name())
	if _, err := tmp.WriteString(content); err != nil {
		tmp.Close()
		return err
	}
	if err := tmp.Chmod(mode); err != nil {
		tmp.Close()
		return err
	}
	if err := tmp.Close(); err != nil {
		return err
	}
	return os.Rename(tmp.Name(), path)
}

func modTime(path string) time.Time {
	info, err := os.Stat(path)
	if err != nil {
		return time.Time{}
	}
	return info.ModTime()
}
