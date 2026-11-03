package main

import (
	"context"
	"errors"
	"io"
	"log/slog"

	"github.com/dagimg-dot/noty/internal/config"
	"github.com/dagimg-dot/noty/internal/notes"
	"github.com/dagimg-dot/noty/internal/theme"
	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"
)

// app is Noty's state. Its view builds the window from it on the UI
// thread.
//
// This is the scaffold of the port: it lists the notes, in the chosen
// theme. The search bar, editor and preferences come in the next steps.
type app struct {
	cfgStore *config.Store
	cfg      *config.Config
	store    *notes.Store
	themes   *theme.DB

	win *mygo.Window

	selected int
	list     ui.ListState

	// uiTheme caches the window's theme for themeFor.
	uiTheme struct {
		theme *theme.Theme
		base  ui.Theme
		out   *ui.Theme
	}
}

func newApp(cfgStore *config.Store, cfg *config.Config, store *notes.Store, themes *theme.DB) *app {
	return &app{cfgStore: cfgStore, cfg: cfg, store: store, themes: themes}
}

func (a *app) start() {
	a.win = mygo.NewWindow(mygo.WindowOptions{
		Title:     "Noty",
		Width:     a.cfg.WindowSize.Width,
		Height:    a.cfg.WindowSize.Height,
		MinWidth:  320,
		MinHeight: 300,
		Content:   ui.View(a.view),
	})
	go a.watchThemes(theme.UserDir())
}

// watchThemes reloads the themes when the files of dir change, so a
// theme being written shows its changes right away.
func (a *app) watchThemes(dir string) {
	w, err := mygo.WatchFiles(context.Background(), dir, mygo.FileWatchOptions{})
	if err != nil {
		slog.Warn("watching the themes directory", "dir", dir, "err", err)
		return
	}
	for {
		if _, err := w.Next(context.Background()); err != nil {
			if !errors.Is(err, io.EOF) { // EOF: the app is quitting
				slog.Warn("watching the themes directory", "dir", dir, "err", err)
			}
			return
		}
		a.win.Update(func() {
			slog.Debug("reloading themes", "dir", dir)
			a.themes.Load()
		})
	}
}

// present brings the window to the front, as a second launch of Noty
// does.
func (a *app) present() {
	if a.win != nil {
		a.win.Focus()
	}
}

func (a *app) save() {
	if err := a.cfgStore.Save(a.cfg); err != nil {
		slog.Error("saving settings", "err", err)
	}
}

// currentTheme returns the theme to show when the app is dark or light.
// The roots, Noty Dark and Noty Light, are Noty's default look and follow
// the appearance; any other theme is shown as chosen.
func (a *app) currentTheme(dark bool) *theme.Theme {
	t := a.themes.Pick(a.cfg.EditorColorScheme, dark)
	if t.Parent() == nil && t.Dark() != dark {
		return a.themes.Pick("", dark)
	}
	return t
}

// themeFor returns the window's theme for base, the theme MyGo gives the
// frame, which follows the appearance.
func (a *app) themeFor(base ui.Theme) *ui.Theme {
	t := a.currentTheme(base.Dark)
	c := &a.uiTheme
	if c.out == nil || c.theme != t || c.base != base {
		c.theme, c.base, c.out = t, base, t.UI(base)
	}
	return c.out
}

func (a *app) view(c *ui.Context) {
	// Each frame starts from MyGo's theme, which follows the appearance.
	c.SetTheme(a.themeFor(*c.Theme()))
	if c.Shortcut(ui.Cmd, ui.KeyQ) {
		a.save()
		mygo.App.Quit()
	}
	shown := a.store.Sorted("", a.cfg.SortingMethod)
	a.selected = min(a.selected, max(len(shown)-1, 0))
	a.list.Selected = &a.selected
	a.list.Label = func(i int) string { return shown[i].Name }

	ui.Column(c).Fill().Padding(12).Gap(8).Children(func() {
		ui.Textf(c, "%d notes in %s", len(shown), a.store.Options().Dir).TextColor(c.Theme().TextMuted)
		ui.List(c.Key("notes"), &a.list, len(shown), func(i int) {
			ui.Text(c, shown[i].Name).Padding(6, 12)
		}).Grow(1)
	})
}
