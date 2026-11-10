# Contributing to Noty

## Building

You need Rust 1.92 or newer. On Linux you also need a C compiler, `pkg-config`, a Vulkan driver and the development files for fontconfig, freetype, libxcb and libxkbcommon:

- Ubuntu/Debian: `build-essential pkg-config libfontconfig-dev libfreetype-dev libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev`
- Fedora: `gcc pkgconf-pkg-config fontconfig-devel freetype-devel libxcb-devel libxkbcommon-devel libxkbcommon-x11-devel`
- Arch: `base-devel fontconfig freetype2 libxcb libxkbcommon libxkbcommon-x11`

`make run` starts a development build with its own settings file. The `Makefile` also has `test`, `lint`, `install` and `dist`, which builds the release tarball.

## Project Layout

Start at `src/app.rs`, which loads the settings and opens the window.

| Path                  | What lives there                                                              |
| --------------------- | ----------------------------------------------------------------------------- |
| `src/app.rs`          | Startup: command line, settings, opening the window                           |
| `src/config.rs`       | The settings file                                                             |
| `src/notes/`          | Notes on disk: the folder, safe single-file saves, the background save worker |
| `src/theme/`          | Theme files, inheritance and colors                                           |
| `src/ui/main_window/` | The app window, split into saving, note actions, settings, bars and dialogs   |
| `src/ui/search/`      | The search list and fuzzy matching                                            |
| `src/ui/`             | Key bindings, shared widgets, and handing themes to the UI toolkit            |
| `src/platform/`       | Focusing the running Noty when it is launched again                           |
| `src/format.rs`       | Word counts, relative times and shortened paths                               |
| `src/line_edit.rs`    | Moving and duplicating lines in the editor                                    |
| `src/debug_replay.rs` | Keystroke replay for UI testing, in debug builds only                         |

`notes`, `theme` and `platform` never depend on `ui`, so storage and theming can be read and tested without the interface.

## Themes

Theme colors are hex values or references to other colors, optionally with `opacity`, `lighter` or `darker`, such as `muted = { name = "colors.core.foreground", opacity = 0.6 }`. A file with the same name as a built-in theme replaces it, and problems in theme files are printed to the terminal when Noty starts.
