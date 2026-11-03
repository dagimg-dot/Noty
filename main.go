// Noty is a minimal, keyboard driven note taking application.
package main

import (
	"flag"
	"fmt"
	"io"
	"log"
	"log/slog"
	"os"
	"path/filepath"
	"strings"

	"github.com/dagimg-dot/noty/internal/config"
	"github.com/dagimg-dot/noty/internal/notes"
	"github.com/dagimg-dot/noty/internal/theme"
	"github.com/egoist/mygo"
)

// Application IDs: development builds keep settings apart from releases.
const (
	appIDRelease = "com.dagimg.noty"
	appIDDev     = "com.dagimg.dev.noty"
)

func appID() string {
	if mygo.IsDev() {
		return appIDDev
	}
	return appIDRelease
}

func main() {
	debug := flag.Bool("debug", false, "enable debug logging")
	logFile := flag.String("log-file", "", "write the log to this file")
	logLevel := flag.String("log-level", "WARNING", "log level: DEBUG, INFO, WARNING or ERROR")
	checkTheme := flag.String("check-theme", "", "check the theme `file` for problems and exit")
	flag.Parse()

	if *checkTheme != "" {
		os.Exit(runCheckTheme(*checkTheme))
	}

	if err := setupLogging(*debug, *logLevel, *logFile); err != nil {
		fmt.Fprintln(os.Stderr, "noty:", err)
		os.Exit(2)
	}

	mygo.App.SetName("Noty")
	if !mygo.App.RequestSingleInstanceLock() {
		return
	}

	id := appID()
	slog.Info("starting Noty", "version", mygo.App.Version(), "id", id)
	cfgStore := config.NewStore(id)
	cfg, err := cfgStore.Load()
	if err != nil {
		slog.Error("loading settings", "err", err)
	}
	store, err := notes.New(notes.Options{
		Dir:          cfg.NotesDir,
		Recurse:      cfg.RecurseSubfolders,
		UseExtension: cfg.UseFileExtension,
	})
	if err != nil {
		slog.Error("loading notes", "err", err)
	}

	if err := os.MkdirAll(theme.UserDir(), 0o755); err != nil {
		slog.Warn("creating the themes directory", "err", err)
	}
	themes := theme.NewDB(theme.DefaultDirs()...)
	setAppearance(cfg.Theme)

	a := newApp(cfgStore, cfg, store, themes)
	mygo.App.OnSecondInstance(func([]string, string) { a.present() })
	mygo.App.WhenReady(a.start)
	if err := mygo.App.Run(); err != nil {
		log.Fatal(err)
	}
}

func setupLogging(debug bool, level, file string) error {
	var lvl slog.Level
	switch strings.ToUpper(level) {
	case "DEBUG":
		lvl = slog.LevelDebug
	case "INFO":
		lvl = slog.LevelInfo
	case "WARNING", "WARN":
		lvl = slog.LevelWarn
	case "ERROR", "CRITICAL":
		lvl = slog.LevelError
	default:
		return fmt.Errorf("unknown log level %q", level)
	}
	if debug {
		lvl = slog.LevelDebug
	}
	var w io.Writer = os.Stderr
	if file != "" {
		f, err := os.OpenFile(file, os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0o644)
		if err != nil {
			return err
		}
		w = io.MultiWriter(os.Stderr, f)
	}
	slog.SetDefault(slog.New(slog.NewTextHandler(w, &slog.HandlerOptions{Level: lvl})))
	return nil
}

// setAppearance makes the app light, dark, or follow the system.
func setAppearance(appearance string) {
	switch appearance {
	case config.AppearanceLight:
		mygo.Theme.SetSource(mygo.ThemeLight)
	case config.AppearanceDark:
		mygo.Theme.SetSource(mygo.ThemeDark)
	default:
		mygo.Theme.SetSource(mygo.ThemeSystem)
	}
}

// runCheckTheme reports the problems of a theme file, resolved against
// the installed themes, and returns the exit status.
func runCheckTheme(path string) int {
	if abs, err := filepath.Abs(path); err == nil {
		path = abs
	}
	t, warnings, err := theme.ReadFile(path)
	if err != nil {
		fmt.Fprintf(os.Stderr, "%s: %v\n", path, err)
		return 1
	}
	// Resolve it among the installed themes, from its own directory first.
	dirs := append([]string{filepath.Dir(path)}, theme.DefaultDirs()...)
	if th := theme.NewDB(dirs...).Get(t.ID); th != nil && th.Path == path {
		warnings = append(warnings, th.Check()...)
		if th.Parent() != nil {
			fmt.Printf("%s (%s, %s) inherits %s\n", t.Name, t.ID, t.Variant, th.Parent().ID)
		}
	}
	for _, w := range warnings {
		fmt.Printf("%s: %s\n", path, w)
	}
	if len(warnings) > 0 {
		return 1
	}
	fmt.Printf("%s: OK\n", path)
	return 0
}
