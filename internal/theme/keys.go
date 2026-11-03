package theme

// A key names a color of a theme, as its path under [colors] in a theme
// file: "core.accent" is accent in [colors.core].
//
// Keys a theme leaves out come from the theme it inherits, else are
// derived from other keys by the rules below, so a theme can set the core
// colors and accents alone.

// Keys of the core palette, which roots must set.
const (
	CoreAccent              = "core.accent"
	CoreAccentForeground    = "core.accent_foreground"
	CoreBackground          = "core.background"
	CoreForeground          = "core.foreground"
	CoreSecondaryBackground = "core.secondary_background"
	CoreBorder              = "core.border"
)

// derivation computes a key a theme does not set from other keys.
type derivation func(r resolver) Color

// resolver resolves keys in the theme being resolved.
type resolver struct {
	get  func(key string) Color
	dark bool
}

func ref(key string) derivation { return func(r resolver) Color { return r.get(key) } }

func alpha(key string, a float64) derivation {
	return func(r resolver) Color { return r.get(key).WithAlpha(a) }
}

func mixed(a, b string, t float64) derivation {
	return func(r resolver) Color { return r.get(a).Mix(r.get(b), t) }
}

// emphasized moves a color away from the background: lighter in a dark
// theme, darker in a light one.
func emphasized(key string, p float64) derivation {
	return func(r resolver) Color {
		if r.dark {
			return r.get(key).Lighter(p)
		}
		return r.get(key).Darker(p)
	}
}

// keyDef describes a key: how it is derived when no theme sets it, and,
// for syntax keys, the font style it has by default.
type keyDef struct {
	key    string
	derive derivation // nil: only set by themes
	bold   bool
	italic bool
}

// keyDefs are the keys of a theme, in the order of the documentation.
var keyDefs = []keyDef{
	// The core palette.
	{key: CoreAccent, derive: ref("accents.blue")},
	{key: CoreAccentForeground, derive: func(r resolver) Color { return contrasting(r.get(CoreAccent)) }},
	{key: CoreBackground},
	{key: CoreForeground},
	{key: CoreSecondaryBackground, derive: emphasized(CoreBackground, 0.04)},
	{key: CoreBorder, derive: mixed(CoreBackground, CoreForeground, 0.15)},

	// Accent colors, which other keys are derived from.
	{key: "accents.blue"},
	{key: "accents.green"},
	{key: "accents.magenta"},
	{key: "accents.orange"},
	{key: "accents.red"},
	{key: "accents.yellow"},
	{key: "accents.cyan"},
	{key: "accents.purple"},

	// Text.
	{key: "text.default", derive: ref(CoreForeground)},
	{key: "text.muted", derive: alpha("text.default", 0.65)},
	{key: "text.danger", derive: ref("accents.red")},
	{key: "text.success", derive: ref("accents.green")},
	{key: "text.warning", derive: ref("accents.yellow")},
	{key: "text.selection.background", derive: alpha(CoreAccent, 0.3)},
	{key: "text.links.default", derive: ref("accents.blue")},

	// Controls.
	{key: "input.border_focus", derive: ref(CoreAccent)},
	{key: "button.background", derive: ref(CoreSecondaryBackground)},
	{key: "button.hover.background", derive: mixed("button.background", CoreForeground, 0.06)},
	{key: "button.pressed.background", derive: mixed("button.background", CoreForeground, 0.12)},
	{key: "button.primary.hover.background", derive: emphasized(CoreAccent, 0.08)},
	{key: "button.primary.pressed.background", derive: emphasized(CoreAccent, 0.16)},
	{key: "list.item.selection.background", derive: alpha(CoreAccent, 0.22)},
	{key: "list.item.selection.foreground", derive: ref("text.default")},
	{key: "list.item.hover.background", derive: alpha(CoreForeground, 0.06)},
	{key: "tooltip.background", derive: ref("text.default")},
	{key: "tooltip.foreground", derive: ref(CoreBackground)},
	{key: "scrollbars.background", derive: alpha(CoreForeground, 0.25)},

	// The editor.
	{key: "editor.background", derive: ref(CoreBackground)},
	{key: "editor.foreground", derive: ref("text.default")},
	{key: "editor.gutter.foreground", derive: alpha("editor.foreground", 0.45)},
	{key: "editor.gutter.active_foreground", derive: ref("editor.foreground")},
	{key: "editor.active_line", derive: alpha(CoreForeground, 0.05)},
	{key: "editor.cursor", derive: ref(CoreAccent)},
	{key: "editor.selection.background", derive: ref("text.selection.background")},
	{key: "editor.selection.inactive_background", derive: alpha("editor.selection.background", 0.55)},
	{key: "editor.bracket_match", derive: ref("text.muted")},
	{key: "editor.find.match", derive: alpha("accents.yellow", 0.3)},
	{key: "editor.find.current_match", derive: alpha("accents.orange", 0.5)},
	{key: "editor.occurrence", derive: alpha(CoreForeground, 0.1)},
	{key: "editor.whitespace", derive: alpha("editor.foreground", 0.3)},
	{key: "editor.indent_guide", derive: alpha(CoreBorder, 0.6)},
	{key: "editor.active_indent_guide", derive: alpha("editor.foreground", 0.35)},
	{key: "editor.wrap_guide", derive: alpha(CoreBorder, 0.6)},
	{key: "editor.error", derive: ref("accents.red")},
	{key: "editor.warning", derive: ref("accents.yellow")},
	{key: "editor.info", derive: ref("accents.blue")},
	{key: "editor.hint", derive: ref("text.muted")},
	{key: "editor.added", derive: ref("accents.green")},
	{key: "editor.deleted", derive: ref("accents.red")},
	{key: "editor.modified", derive: ref("accents.yellow")},

	// Syntax highlighting, which can also set bold and italic.
	{key: "syntax.keyword", derive: ref("accents.red")},
	{key: "syntax.type", derive: ref("accents.orange")},
	{key: "syntax.function", derive: ref("accents.purple")},
	{key: "syntax.variable", derive: ref("editor.foreground")},
	{key: "syntax.property", derive: ref("accents.blue")},
	{key: "syntax.constant", derive: ref("accents.blue")},
	{key: "syntax.number", derive: ref("syntax.constant")},
	{key: "syntax.string", derive: ref("accents.cyan")},
	{key: "syntax.escape", derive: ref("accents.magenta")},
	{key: "syntax.regexp", derive: ref("syntax.string")},
	{key: "syntax.comment", derive: ref("text.muted"), italic: true},
	{key: "syntax.operator", derive: ref("editor.foreground")},
	{key: "syntax.punctuation", derive: ref("text.muted")},
	{key: "syntax.tag", derive: ref("accents.green")},
	{key: "syntax.attribute", derive: ref("accents.blue")},
	{key: "syntax.meta", derive: ref("accents.purple")},
	{key: "syntax.heading", derive: ref(CoreAccent), bold: true},
	{key: "syntax.emphasis", derive: ref("editor.foreground"), italic: true},
	{key: "syntax.strong", derive: ref("editor.foreground"), bold: true},
	{key: "syntax.link", derive: ref("text.links.default")},
	{key: "syntax.code", derive: ref("accents.orange")},
	{key: "syntax.inserted", derive: ref("accents.green")},
	{key: "syntax.deleted", derive: ref("accents.red")},
	{key: "syntax.invalid", derive: ref("accents.red")},
}

var keyIndex = func() map[string]*keyDef {
	m := make(map[string]*keyDef, len(keyDefs))
	for i := range keyDefs {
		m[keyDefs[i].key] = &keyDefs[i]
	}
	return m
}()

// Keys returns the keys a theme can set, in documentation order.
func Keys() []string {
	out := make([]string, len(keyDefs))
	for i, d := range keyDefs {
		out[i] = d.key
	}
	return out
}

// requiredKeys are the keys with no derivation, which root themes set.
func requiredKeys() []string {
	var out []string
	for _, d := range keyDefs {
		if d.derive == nil {
			out = append(out, d.key)
		}
	}
	return out
}

func isSyntaxKey(key string) bool { return len(key) > 7 && key[:7] == "syntax." }
