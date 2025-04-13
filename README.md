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

- **Minimal Interface:** Focus on your notes without distractions.
- **Keyboard Driven:** Designed primarily for mouse-free operation.
- **Flatpak:** Easy installation and updates.
- **External change detection:** Automatically reload notes when changed outside the app.

## Installation

### Flatpak (Recommended)

Flatpak bundles are available from the [GitHub Releases page](https://github.com/dagimg/noty/releases). Download the latest `.flatpak` file and install it using your graphical software center or via the command line:

```bash
flatpak install noty.flatpak
```

### Updating

```bash
flatpak install --or-update noty.flatpak
```

### Building from Source

**Dependencies:**

- Meson (`>= 1.0.0`)
- Ninja
- GTK4
- Libadwaita
- Python 3 # dependency of ./noty
- blueprint-compiler (`>= 0.16.0`)
- glib-compile-schemas
- desktop-file-utils

**(Note:** Dependencies might vary slightly based on your distribution. You'll typically need the development packages, e.g., `libadwaita-devel` or `libadwaita-dev`.)\*

**Steps:**

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/dagimg-dot/noty.git
    cd noty
    ```
2.  **Build, Install and Run:**
    The `Makefile` provides convenient targets:

    ```bash
    make run
    ```

    Or directly:

    ```bash
    make install
    ```

    ```bash
    build/bin/noty
    ```

> **Note:** Check `Makefile` for more targets.

## TODO

- [x] Rename note
- [x] Note deletion
- [ ] Full markdown support
- [ ] Custom color scheme picker
- [ ] Custom font picker

## License

`Noty` is distributed under the terms of the GPLv3 license. See the [COPYING](COPYING) file for details.
