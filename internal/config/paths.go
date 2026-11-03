package config

import (
	"bufio"
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

// Path returns where the settings of appID are saved:
// $XDG_CONFIG_HOME/<appID>.json, ~/.config/<appID>.json without it.
func Path(appID string) string {
	return filepath.Join(configHome(), appID+".json")
}

// LegacyPaths returns where earlier versions of Noty saved the settings
// of appID and Path does not already cover: ~/.config when
// $XDG_CONFIG_HOME points elsewhere (the Python version ignored it), and
// the Flatpak's own config directory.
func LegacyPaths(appID string) []string {
	home, err := os.UserHomeDir()
	if err != nil {
		return nil
	}
	var paths []string
	if p := filepath.Join(home, ".config", appID+".json"); p != Path(appID) {
		paths = append(paths, p)
	}
	paths = append(paths, filepath.Join(home, ".var", "app", appID, "config", appID+".json"))
	return paths
}

func configHome() string {
	if dir := os.Getenv("XDG_CONFIG_HOME"); filepath.IsAbs(dir) {
		return dir
	}
	home, _ := os.UserHomeDir()
	return filepath.Join(home, ".config")
}

// DocumentsDir returns the user's documents directory: XDG_DOCUMENTS_DIR
// of user-dirs.dirs, ~/Documents without it.
func DocumentsDir() string {
	home, _ := os.UserHomeDir()
	if dir := userDir(filepath.Join(configHome(), "user-dirs.dirs"), "XDG_DOCUMENTS_DIR", home); dir != "" {
		return dir
	}
	return filepath.Join(home, "Documents")
}

// userDir reads the directory named key from a user-dirs.dirs file, as
// xdg-user-dirs writes it: KEY="$HOME/Documents". It returns "" when the
// file or key is missing, or names the home directory itself, which is
// what xdg-user-dirs sets for a disabled directory.
func userDir(file, key, home string) string {
	f, err := os.Open(file)
	if err != nil {
		return ""
	}
	defer f.Close()
	sc := bufio.NewScanner(f)
	for sc.Scan() {
		line := strings.TrimSpace(sc.Text())
		name, value, ok := strings.Cut(line, "=")
		if !ok || strings.HasPrefix(line, "#") || strings.TrimSpace(name) != key {
			continue
		}
		value = strings.TrimSpace(value)
		if unq, err := strconv.Unquote(value); err == nil {
			value = unq
		}
		switch {
		case value == "$HOME" || value == "$HOME/":
			return ""
		case strings.HasPrefix(value, "$HOME/"):
			value = filepath.Join(home, value[len("$HOME/"):])
		case !filepath.IsAbs(value):
			return ""
		}
		if filepath.Clean(value) == filepath.Clean(home) {
			return ""
		}
		return filepath.Clean(value)
	}
	return ""
}
