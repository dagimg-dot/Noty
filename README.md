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

### I. Flatpak (Recommended)

Flatpak bundles are available from the [GitHub Releases page](https://github.com/dagimg/noty/releases). Download the latest `.flatpak` file and install it using your graphical software center or via the command line:

```bash
flatpak install noty-x.x.x.flatpak
```

### II. Tar Archive

You can also install Noty from the tar archive available on the [GitHub Releases page](https://github.com/dagimg/noty/releases):

1. Download the latest `noty-x.x.x-linux-x86_64.tar.xz` file
2. Extract the archive:
   ```bash
   tar -xf noty-x.x.x-linux-x86_64.tar.xz
   cd noty-x.x.x-linux-x86_64
   ```
3. Run:
   ```bash
   ./noty
   ```

### III. Using [eget](https://github.com/zyedidia/eget)

```bash
eget dagimg-dot/noty
```

### Updating

```bash
flatpak install --or-update noty.flatpak
```

### Building from Source

**Dependencies:**

- Meson (`>= 1.0.0`)
- Ninja
- GTK4 and Libadwaita (including development and introspection packages)
  - For Ubuntu/Debian: `libgtk-4-dev gir1.2-gtk-4.0 libadwaita-1-dev gobject-introspection libgirepository1.0-dev`
  - For Fedora: `gtk4-devel libadwaita-devel gobject-introspection-devel`
  - For Arch: `gtk4 libadwaita gobject-introspection`
- Python 3
- Blueprint Compiler (`>= 0.16.0`)

**(Note:** Package names might vary slightly based on your distribution. You'll typically need both the runtime and development packages.)\*

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

- [x] Note renaming
- [x] Note deletion
- [x] Window size persistence
- [ ] Full markdown support
- [ ] Custom color scheme picker
- [ ] Custom font picker

## License

`Noty` is distributed under the terms of the GPLv3 license. See the [COPYING](COPYING) file for details.
