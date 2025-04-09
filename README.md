<div align="center">
  <img src="https://raw.githubusercontent.com/dagimg-dot/noty/main/data/icons/hicolor/scalable/apps/com.dagimg.noty.svg" width="100">
  <h1 align="center">Noty</h1>
  <p align="center">A minimal keyboard driven note appcation</p>
</div>

<div align="center">
  <img src="
  https://i.ibb.co/352fBR7r/Screenshot-From-2025-04-09-21-49-52.png
https://i.ibb.co/gbZr6ks5/Screenshot-From-2025-04-09-21-50-26.png
  " width="300">
</div>

## Features

- **Minimal Interface:** Focus on your notes without distractions.
- **Keyboard Driven:** Designed primarily for mouse-free operation.
- **Markdown Support:** Write notes using standard Markdown syntax.
- **Plain Text:** Simple text format is also supported.

## Installation

### Flatpak (Recommended)

Flatpak bundles are available from the [GitHub Releases page](https://github.com/dagimg/noty/releases). Download the latest `.flatpak` file and install it using your graphical software center or via the command line:

```bash
flatpak install noty.flatpak
```

### Building from Source

**Dependencies:**

- Meson (`>= 1.0.0`)
- Ninja
- GTK4 (`>= 4.10`)
- Libadwaita (`>= 1.4`)
- Python 3 (`>= 3.6`) # dependency of ./noty
- blueprint-compiler (`>= 0.8.1`)
- glib-compile-schemas
- desktop-file-utils

**(Note:** Dependencies might vary slightly based on your distribution. You'll typically need the development packages, e.g., `libadwaita-devel` or `libadwaita-dev`.)\*

**Steps:**

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/dagimg/noty.git
    cd noty
    ```
2.  **Build and Install:**
    The `Makefile` provides convenient targets:

    ```bash
    # Build the project
    make build

    # Install the built artifacts (needed for running)
    make install
    ```

3.  **Run:**
    ```bash
    # Run the development version
    make run
    ```
    Or directly:
    ```bash
    build/bin/noty
    ```

## TODO

- [ ] Note name update
- [ ] Note deletion
- [ ] Custom color scheme picker

## License

`Noty` is distributed under the terms of the GPLv3 license. See the [COPYING](COPYING) file for details.
