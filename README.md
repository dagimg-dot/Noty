<div align="center">
  <img src="https://raw.githubusercontent.com/dagimg-dot/noty/main/data/icons/hicolor/scalable/apps/com.dagimg.noty.svg" width="100">
  <h1 align="center">Noty</h1>
  <p align="center">A minimal keyboard driven note taking application</p>
  <p>
  <a href="https://github.com/dagimg-dot/noty/releases/latest">
    <img src="https://img.shields.io/github/v/release/dagimg-dot/noty?label=version" alt="Latest Release">
  </a>
  <a href="https://github.com/dagimg-dot/noty/releases/latest">
    <img src="https://img.shields.io/github/downloads/dagimg-dot/noty/latest/total?label=downloads" alt="Downloads">
  </a>
  </p>
</div>

<div align="center">
  <img src="https://i.ibb.co/352fBR7r/Screenshot-From-2025-04-09-21-49-52.png" width="300">
  <img src="https://i.ibb.co/gbZr6ks5/Screenshot-From-2025-04-09-21-50-26.png" width="300">
</div>

## Features

- **Search in the title bar:** Type to find a note and press Enter to open it, or to create it when nothing matches.
- **Keyboard driven:** Every action has a shortcut, so the mouse stays optional.
- **Markdown editor:** Syntax highlighting and autosave, without silently overwriting notes that changed outside the app.
- **Themes:** Adwaita, Catppuccin, Nord, Tokyo Night and Vesper are built in, and your own themes are plain TOML files.
- **Plain files:** Notes are ordinary text files in a folder you choose.
- **Cross-platform:** Built with [GPUI](https://www.gpui.rs) for Linux, macOS and Windows.

## Keyboard Shortcuts

| Keys | Action |
| --- | --- |
| <kbd>Ctrl</kbd>+<kbd>P</kbd>, <kbd>Alt</kbd>+<kbd>S</kbd> or <kbd>Esc</kbd> | Switch between search and the editor |
| <kbd>↑</kbd> <kbd>↓</kbd> or <kbd>Ctrl</kbd>+<kbd>J</kbd> <kbd>Ctrl</kbd>+<kbd>K</kbd> | Move through the note list |
| <kbd>Enter</kbd> | Open the selected note, or create a new one |
| <kbd>Shift</kbd>+<kbd>Enter</kbd> | Create a note named exactly what you typed |
| <kbd>F2</kbd>, or <kbd>Ctrl</kbd>+<kbd>R</kbd> while searching | Rename a note |
| <kbd>Delete</kbd> on a selected note, or <kbd>Shift</kbd>+<kbd>Delete</kbd> while searching | Delete a note |
| <kbd>Ctrl</kbd>+<kbd>S</kbd> | Save now, or keep your version after a conflict |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd> | Reload the note from disk |
| <kbd>Ctrl</kbd>+<kbd>=</kbd>, <kbd>Ctrl</kbd>+<kbd>-</kbd>, <kbd>Ctrl</kbd>+<kbd>0</kbd> | Bigger, smaller or default text size |
| <kbd>Alt</kbd>+<kbd>L</kbd> | Toggle line numbers |
| <kbd>Ctrl</kbd>+<kbd>?</kbd> | Show all shortcuts |
| <kbd>Ctrl</kbd>+<kbd>Q</kbd> | Quit |

On macOS, <kbd>⌘</kbd> takes the place of <kbd>Ctrl</kbd>, except in <kbd>Ctrl</kbd>+<kbd>J</kbd>, <kbd>Ctrl</kbd>+<kbd>K</kbd> and <kbd>Ctrl</kbd>+<kbd>R</kbd>.

## Themes

Pick a light and a dark theme from the <kbd>⋯</kbd> menu, and choose whether Noty follows the system appearance or always stays light or dark. The built-in themes are Noty Paper, Noty Night, Adwaita Light, Adwaita Dark, Catppuccin Latte, Frappé, Macchiato and Mocha, Nord, Nord Light, Tokyo Night, Tokyo Night Storm and Vesper.

### Custom Themes

Themes use the [Vicinae](https://github.com/vicinaehq/vicinae) TOML format, so Vicinae themes work too. Put `.toml` files in the themes folder and restart Noty:

| Platform | Folder |
| --- | --- |
| Linux | `~/.local/share/noty/themes` |
| macOS | `~/Library/Application Support/noty/themes` |
| Windows | `%APPDATA%\noty\themes` |

The file name is the theme's id, and a file with the same name as a built-in theme replaces it. A theme only needs a few colors, because the rest are derived from them or inherited from its parent:

```toml
[meta]
name = "Mint Paper"
variant = "light"
inherits = "noty-light"

[colors.core]
background = "#f6faf7"
foreground = "#1e2b24"
accent = "#2f8f63"

[colors.text]
muted = { name = "colors.core.foreground", opacity = 0.6 }
```

Colors are hex values or references to other colors, optionally with `opacity`, `lighter` or `darker`. Problems in theme files are printed to the terminal when Noty starts.

## Settings

The <kbd>⋯</kbd> menu covers themes, appearance, line numbers, the close button and text size. Everything else lives in `com.dagimg.noty.json`, the same file the GTK version of Noty used. It is in `~/.config` on Linux, `~/Library/Application Support` on macOS and `%APPDATA%` on Windows.

| Key | Default | Description |
| --- | --- | --- |
| `notes_dir` | `~/Documents/Noties` | Folder that holds your notes |
| `sorting_method` | `"name"` | Sort notes by `"name"` or `"date_modified"` |
| `recurse_subfolders` | `false` | Include notes from subfolders |
| `use_file_extension` | `false` | Create new notes with a `.md` extension |
| `use_custom_font` | `false` | Use `custom_font` in the editor |
| `custom_font` | `"Monospace 12"` | Editor font, such as `"JetBrains Mono 12"` |
| `persist_window_size` | `true` | Reopen at the last window size |

Run `noty --notes-dir DIR` to open a different folder for one session.

## Installation

### Release Tarball (Recommended)

Download the latest `noty-x.x.x-linux-<arch>.tar.gz` for your CPU (`x86_64` or `aarch64`) from the [GitHub Releases page](https://github.com/dagimg-dot/noty/releases) and extract it into `~/.local`:

```bash
tar -xzf noty-x.x.x-linux-x86_64.tar.gz -C ~/.local --strip-components=1
```

This puts `noty` in `~/.local/bin` and its desktop entry and icons in `~/.local/share`, so Noty appears in your app launcher as long as `~/.local/bin` is on your `PATH`. Extracting a newer tarball the same way updates it.

### Building from Source

**Dependencies:**

- Rust 1.92 or newer
- On Linux, a C compiler, `pkg-config`, a Vulkan driver and the development files for fontconfig, freetype, libxcb and libxkbcommon
  - For Ubuntu/Debian: `build-essential pkg-config libfontconfig-dev libfreetype-dev libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev`
  - For Fedora: `gcc pkgconf-pkg-config fontconfig-devel freetype-devel libxcb-devel libxkbcommon-devel libxkbcommon-x11-devel`
  - For Arch: `base-devel fontconfig freetype2 libxcb libxkbcommon libxkbcommon-x11`

**Steps:**

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/dagimg-dot/noty.git
    cd noty
    ```
2.  **Build and Run:**
    The `Makefile` provides convenient targets:

    ```bash
    make run
    ```

    This runs a development build with its own settings file. To install a release build into `~/.local` instead:

    ```bash
    make install
    ```

> **Note:** Check `Makefile` for more targets, such as `test`, `lint` and `dist`, which builds the release tarball.

## TODO

- [x] Note renaming
- [x] Note deletion
- [x] Window size persistence
- [x] Markdown highlighting
- [x] Custom themes
- [ ] Preferences window
- [ ] Vim mode

## Credits

The built-in themes are adapted from the [Vicinae](https://github.com/vicinaehq/vicinae) theme collection.

## License

`Noty` is distributed under the terms of the GPLv3 license. See the [COPYING](COPYING) file for details.
