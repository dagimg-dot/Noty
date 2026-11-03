package main

import (
	"log/slog"

	"github.com/dagimg-dot/noty/internal/config"
	"github.com/dagimg-dot/noty/internal/notes"
	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"
)

// app is Noty's state. Its view builds the window from it on the UI
// thread.
//
// This is the scaffold of the port: it lists the notes. The search bar,
// editor, preferences and themes come in the next steps.
type app struct {
	cfgStore *config.Store
	cfg      *config.Config
	store    *notes.Store

	win *mygo.Window

	selected int
	list     ui.ListState
}

func newApp(cfgStore *config.Store, cfg *config.Config, store *notes.Store) *app {
	return &app{cfgStore: cfgStore, cfg: cfg, store: store}
}

func (a *app) start() {
	opts := mygo.WindowOptions{
		Title:     "Noty",
		Width:     a.cfg.WindowSize.Width,
		Height:    a.cfg.WindowSize.Height,
		MinWidth:  320,
		MinHeight: 300,
		Content:   ui.View(a.view),
	}
	a.win = mygo.NewWindow(opts)
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

func (a *app) view(c *ui.Context) {
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
