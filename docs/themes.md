# Themes

Noty's themes color the whole app: the window, its controls, the editor and
its syntax highlighting. A theme is a TOML file, in the spirit of
[Vicinae](https://github.com/vicinaehq/vicinae)'s themes: you set the colors
you care about, and the others are inherited from another theme or derived
from the colors you set.

## Where themes live

| Directory | |
|---|---|
| `~/.local/share/noty/themes` (`$XDG_DATA_HOME/noty/themes`) | your themes; Noty reloads them as soon as a file changes |
| `noty/themes` in each of `$XDG_DATA_DIRS` | themes installed for every user |
| bundled | Noty Dark, Noty Light, Coldark Dark, Nord, Tokyo Night |

A theme's ID is its file name without `.toml`. When two directories have a
theme of the same ID, the first one in the table above wins, so a file named
`nord.toml` in your directory replaces the bundled Nord. The IDs
`noty-dark` and `noty-light` belong to the bundled roots and cannot be
replaced.

Check a theme while you write it:

```bash
noty --check-theme ~/.local/share/noty/themes/my-theme.toml
```

It reports unknown keys, invalid colors, circular references and the theme
it inherits.

## The file

```toml
[meta]
name = "Sunset"                    # required
description = "Warm evening colors"
variant = "dark"                   # required: "dark" | "light"
inherits = "noty-dark"             # optional: the root of its variant by default

[colors.core]
background = "#1e1a24"
foreground = "#efe6dc"
accent = "colors.accents.orange"   # a reference to another key

[colors.accents]
orange = "#f2994a"

[colors.syntax]
comment = { name = "colors.text.muted", italic = false }
heading = { name = "colors.core.accent", lighter = 15 }
```

### Colors

A color is one of:

- `"#rrggbb"`, `"#rrggbbaa"`, `"#rgb"` or `"#rgba"`. The alpha comes last, as
  in CSS (Vicinae puts it first).
- `"colors.<key>"`, a reference to another key, such as
  `"colors.accents.blue"`. References resolve in the theme being shown, so
  when a theme changes `accents.blue`, every key that refers to it follows,
  in the themes it inherits too. A key that refers to itself gets the color
  of the theme it inherits: `keyword = { name = "colors.syntax.keyword", darker = 10 }`.
- a table with a `name` (a color or reference, as above), and any of:
  - `opacity`, from 0 to 1, multiplied with the color's alpha
  - `lighter`, from 0 to 100, the percentage to move toward white
  - `darker`, from 0 to 100, the percentage to move toward black
  - `bold` and `italic`, true or false, for `syntax` keys only

### How a key gets its color

1. The theme sets it.
2. Else the closest theme it inherits sets it.
3. Else it is derived from other keys, as the tables below say.

The roots, Noty Dark and Noty Light, set the core colors and accents, so a
theme can set as little as a background and a foreground.

When the app's theme is a root, Noty follows the appearance (Preferences ›
Theme, or the system's light or dark style) and shows the root of that
variant. Any other theme shows as chosen.

## Keys

Keys are written as their path under `[colors]`: `core.accent` is `accent`
in `[colors.core]`, and `editor.gutter.foreground` can be written
`gutter = { foreground = "#888" }` in `[colors.editor]`.

### Core

| Key | Derived from |
|---|---|
| `core.background` | set by the roots |
| `core.foreground` | set by the roots |
| `core.accent` | `accents.blue` |
| `core.accent_foreground` | black or white, whichever reads better on the accent |
| `core.secondary_background` | the background, 4% toward the foreground's side |
| `core.border` | the background mixed 15% with the foreground |

### Accents

`accents.blue`, `accents.green`, `accents.magenta`, `accents.orange`,
`accents.red`, `accents.yellow`, `accents.cyan`, `accents.purple`: set by
the roots, and the source of most derived colors.

### Text and controls

| Key | Used for | Derived from |
|---|---|---|
| `text.default` | text | `core.foreground` |
| `text.muted` | secondary text, placeholders | `text.default` at 65% opacity |
| `text.danger` | destructive actions, errors | `accents.red` |
| `text.success` | | `accents.green` |
| `text.warning` | | `accents.yellow` |
| `text.selection.background` | selected text in fields | `core.accent` at 30% opacity |
| `text.links.default` | links | `accents.blue` |
| `input.border_focus` | the focus ring | `core.accent` |
| `button.background` | buttons, fields and other controls | `core.secondary_background` |
| `button.hover.background` | | `button.background` mixed 6% with the foreground |
| `button.pressed.background` | | `button.background` mixed 12% with the foreground |
| `button.primary.hover.background` | primary buttons and checked controls, which use the accent | the accent, 8% away from the background |
| `button.primary.pressed.background` | | the accent, 16% away from the background |
| `list.item.selection.background` | the selected note | `core.accent` at 22% opacity |
| `list.item.selection.foreground` | | `text.default` |
| `list.item.hover.background` | | `core.foreground` at 6% opacity |
| `tooltip.background` | tooltips and toasts | `text.default` |
| `tooltip.foreground` | | `core.background` |
| `scrollbars.background` | scroll bar thumbs | `core.foreground` at 25% opacity |

### Editor

| Key | Derived from |
|---|---|
| `editor.background` | `core.background` |
| `editor.foreground` | `text.default` |
| `editor.gutter.foreground` | line numbers: `editor.foreground` at 45% opacity |
| `editor.gutter.active_foreground` | the current line's number: `editor.foreground` |
| `editor.active_line` | `core.foreground` at 5% opacity |
| `editor.cursor` | `core.accent` |
| `editor.selection.background` | `text.selection.background` |
| `editor.selection.inactive_background` | the selection at 55% opacity, while the editor is not focused |
| `editor.bracket_match` | `text.muted` |
| `editor.find.match` | `accents.yellow` at 30% opacity |
| `editor.find.current_match` | `accents.orange` at 50% opacity |
| `editor.occurrence` | other occurrences of the selection: `core.foreground` at 10% opacity |
| `editor.whitespace` | `editor.foreground` at 30% opacity |
| `editor.indent_guide` | `core.border` at 60% opacity |
| `editor.active_indent_guide` | `editor.foreground` at 35% opacity |
| `editor.wrap_guide` | `core.border` at 60% opacity |
| `editor.error`, `editor.warning`, `editor.info`, `editor.hint` | red, yellow, blue, `text.muted` |
| `editor.added`, `editor.deleted`, `editor.modified` | green, red, yellow |

### Syntax

Syntax keys take `bold` and `italic` too. In Markdown notes, `heading`,
`emphasis`, `strong`, `deleted` (strikethrough), `link` (the link's text),
`string` (its URL), `code`, `punctuation` (list and quote markers, fences),
`type` (a fence's language), `keyword` (task list check boxes), `escape`,
and `comment` and `tag` (HTML) color the note; the code of fenced blocks
uses the others too.

| Key | Derived from | Style |
|---|---|---|
| `syntax.keyword` | `accents.red` | |
| `syntax.type` | `accents.orange` | |
| `syntax.function` | `accents.purple` | |
| `syntax.variable` | `editor.foreground` | |
| `syntax.property` | `accents.blue` | |
| `syntax.constant` | `accents.blue` | |
| `syntax.number` | `syntax.constant` | |
| `syntax.string` | `accents.cyan` | |
| `syntax.escape` | `accents.magenta` | |
| `syntax.regexp` | `syntax.string` | |
| `syntax.comment` | `text.muted` | italic |
| `syntax.operator` | `editor.foreground` | |
| `syntax.punctuation` | `text.muted` | |
| `syntax.tag` | `accents.green` | |
| `syntax.attribute` | `accents.blue` | |
| `syntax.meta` | `accents.purple` | |
| `syntax.heading` | `core.accent` | bold |
| `syntax.emphasis` | `editor.foreground` | italic |
| `syntax.strong` | `editor.foreground` | bold |
| `syntax.link` | `text.links.default` | |
| `syntax.code` | `accents.orange` | |
| `syntax.inserted` | `accents.green` | |
| `syntax.deleted` | `accents.red` | |
| `syntax.invalid` | `accents.red` | |
