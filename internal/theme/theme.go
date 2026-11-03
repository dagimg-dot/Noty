// Package theme is Noty's theme system: TOML files that color the whole
// application, its controls, the editor and its syntax highlighting.
//
// A theme file looks like this:
//
//	[meta]
//	name = "Nord"
//	description = "An arctic, north-bluish palette"
//	variant = "dark"      # "dark" | "light"
//	inherits = "noty-dark" # optional; the root of its variant by default
//
//	[colors.core]
//	background = "#2e3440"
//	foreground = "#d8dee9"
//	accent = "colors.accents.cyan"          # a reference to another key
//
//	[colors.syntax]
//	comment = { name = "#616e88", italic = true }
//	keyword = { name = "colors.accents.blue", lighter = 10 }
//
// A theme's ID is its file name without the .toml extension.
//
// A color is "#rrggbb", "#rrggbbaa", or a reference "colors.<key>" to
// another key, resolved in the theme being shown, so a theme that changes
// a color changes the keys that refer to it, in the themes it inherits
// too. A reference of a key to itself is the inherited theme's color. As
// a table, a color has a name, the color or reference, and can set
// opacity (0 to 1, multiplied with its alpha), lighter or darker (a
// percentage toward white or black) and, for syntax keys, bold and italic.
//
// A key a theme leaves out is the one of the theme it inherits, else is
// derived from other keys: see keys.go. So a theme can set the core
// colors and accents alone.
package theme

import (
	"errors"
	"fmt"
	"maps"
	"path/filepath"
	"slices"
	"strings"

	"github.com/BurntSushi/toml"
)

// Variant is whether a theme is dark or light.
type Variant string

// Variants.
const (
	Dark  Variant = "dark"
	Light Variant = "light"
)

// IDs of the root themes, which every other theme inherits in the end.
const (
	RootDark  = "noty-dark"
	RootLight = "noty-light"
)

// maxDepth bounds the references followed to resolve one key, which only
// circular references reach.
const maxDepth = 64

// Theme is a parsed theme file.
type Theme struct {
	ID          string
	Name        string
	Description string
	Variant     Variant
	// Inherits is the ID of the theme this one inherits.
	Inherits string
	// Path is the file the theme was read from, "" for bundled themes.
	Path string

	values map[string]value
	// parent is the theme inherited, nil for roots.
	parent *Theme
}

// value is a color a theme sets.
type value struct {
	color   Color
	ref     string  // a key to resolve instead of color
	opacity float64 // with hasOpacity
	lighter float64
	darker  float64
	bold    *bool
	italic  *bool

	hasOpacity bool
}

func (v value) apply(c Color) Color {
	if v.lighter > 0 {
		c = c.Lighter(v.lighter / 100)
	}
	if v.darker > 0 {
		c = c.Darker(v.darker / 100)
	}
	if v.hasOpacity {
		c = c.WithAlpha(v.opacity)
	}
	return c
}

// Dark reports whether the theme is a dark one.
func (t *Theme) Dark() bool { return t.Variant == Dark }

// Parent returns the theme this one inherits, or nil.
func (t *Theme) Parent() *Theme { return t.parent }

// Sets reports whether the theme itself sets the key.
func (t *Theme) Sets(key string) bool {
	_, ok := t.values[key]
	return ok
}

// Color resolves a key: the color the theme, or the closest theme it
// inherits, sets for it, else the color derived from other keys. Keys
// that are not known, and circular references, are transparent.
func (t *Theme) Color(key string) Color {
	c, _ := t.resolve(key, 0, 0)
	return c
}

// Style is a color and font style of syntax highlighting.
type Style struct {
	Color  Color
	Bold   bool
	Italic bool
}

// Style resolves a syntax key: its color, and whether it is bold and
// italic, as the closest theme that says so sets it, else by default.
func (t *Theme) Style(key string) Style {
	s := Style{Color: t.Color(key)}
	if d := keyIndex[key]; d != nil {
		s.Bold, s.Italic = d.bold, d.italic
	}
	boldSet, italicSet := false, false
	for th := t; th != nil && !(boldSet && italicSet); th = th.parent {
		v, ok := th.values[key]
		if !ok {
			continue
		}
		if v.bold != nil && !boldSet {
			s.Bold, boldSet = *v.bold, true
		}
		if v.italic != nil && !italicSet {
			s.Italic, italicSet = *v.italic, true
		}
	}
	return s
}

// resolve resolves key, looking for the themes that set it from level
// levels up the inheritance chain, and references always from t, the
// theme shown. It reports false for a circular reference.
func (t *Theme) resolve(key string, level, depth int) (Color, bool) {
	if depth > maxDepth {
		return Color{}, false
	}
	th := t
	for i := 0; i < level && th != nil; i++ {
		th = th.parent
	}
	for ; th != nil; th, level = th.parent, level+1 {
		v, ok := th.values[key]
		if !ok {
			continue
		}
		switch v.ref {
		case "":
			return v.apply(v.color), true
		case key:
			c, ok := t.resolve(key, level+1, depth+1)
			return v.apply(c), ok
		default:
			c, ok := t.resolve(v.ref, 0, depth+1)
			return v.apply(c), ok
		}
	}
	d := keyIndex[key]
	if d == nil || d.derive == nil {
		return Color{}, true
	}
	circular := false
	r := resolver{dark: t.Dark(), get: func(k string) Color {
		c, ok := t.resolve(k, 0, depth+1)
		circular = circular || !ok
		return c
	}}
	c := d.derive(r)
	return c, !circular
}

// Check resolves every key and returns the problems: circular
// references, and keys that resolve to nothing because no theme in the
// inheritance chain sets them.
func (t *Theme) Check() []string {
	var problems []string
	for _, d := range keyDefs {
		if _, ok := t.resolve(d.key, 0, 0); !ok {
			problems = append(problems, fmt.Sprintf("colors.%s: circular reference", d.key))
		}
	}
	for _, k := range requiredKeys() {
		set := false
		for th := t; th != nil && !set; th = th.parent {
			set = th.Sets(k)
		}
		if !set {
			problems = append(problems, fmt.Sprintf("colors.%s is not set", k))
		}
	}
	return problems
}

// Parse reads a theme file. id is the theme's ID, path where it came
// from ("" for bundled themes). Problems that do not prevent using the
// theme, such as unknown keys or invalid colors, which are left out, are
// returned as warnings.
func Parse(id, path string, data []byte) (*Theme, []string, error) {
	var doc map[string]any
	if _, err := toml.Decode(string(data), &doc); err != nil {
		return nil, nil, err
	}
	t := &Theme{ID: id, Path: path, values: map[string]value{}}
	var warnings []string

	meta, ok := doc["meta"].(map[string]any)
	if !ok {
		return nil, nil, errors.New("a [meta] table is required")
	}
	if t.Name, ok = meta["name"].(string); !ok || strings.TrimSpace(t.Name) == "" {
		return nil, nil, errors.New("meta.name must be a string")
	}
	t.Description, _ = meta["description"].(string)
	variant, _ := meta["variant"].(string)
	switch Variant(variant) {
	case Dark, Light:
		t.Variant = Variant(variant)
	default:
		return nil, nil, errors.New(`meta.variant must be "dark" or "light"`)
	}
	if v, ok := meta["inherits"]; ok {
		if t.Inherits, ok = v.(string); !ok {
			return nil, nil, errors.New("meta.inherits must be a string")
		}
	}
	for k := range meta {
		switch k {
		case "name", "description", "variant", "inherits":
		default:
			warnings = append(warnings, "unknown key meta."+k)
		}
	}

	for _, k := range slices.Sorted(maps.Keys(doc)) {
		if k == "meta" {
			continue
		}
		if k != "colors" {
			warnings = append(warnings, "unknown key "+k)
			continue
		}
		colors, ok := doc[k].(map[string]any)
		if !ok {
			return nil, nil, errors.New("colors must be a table")
		}
		warnings = append(warnings, t.parseColors(colors, "")...)
	}

	if err := t.checkCycles(); err != nil {
		return nil, nil, err
	}
	return t, warnings, nil
}

// parseColors reads the keys of a table under [colors], prefix the path
// of the table.
func (t *Theme) parseColors(table map[string]any, prefix string) []string {
	var warnings []string
	for _, k := range slices.Sorted(maps.Keys(table)) {
		key := prefix + k
		if _, known := keyIndex[key]; known {
			v, err := parseValue(key, table[k])
			if err != nil {
				warnings = append(warnings, fmt.Sprintf("colors.%s: %v", key, err))
				continue
			}
			t.values[key] = v
			continue
		}
		if sub, ok := table[k].(map[string]any); ok {
			warnings = append(warnings, t.parseColors(sub, key+".")...)
			continue
		}
		warnings = append(warnings, "unknown key colors."+key)
	}
	return warnings
}

func parseValue(key string, raw any) (value, error) {
	var v value
	var name string
	switch raw := raw.(type) {
	case string:
		name = raw
	case map[string]any:
		for k := range raw {
			switch k {
			case "name", "color", "opacity", "lighter", "darker":
			case "bold", "italic":
				if !isSyntaxKey(key) {
					return v, fmt.Errorf("%s is only for syntax keys", k)
				}
			default:
				return v, fmt.Errorf("unknown field %s", k)
			}
		}
		var ok bool
		if name, ok = raw["name"].(string); !ok {
			if name, ok = raw["color"].(string); !ok {
				return v, errors.New("a color table needs a name")
			}
		}
		var err error
		if v.opacity, v.hasOpacity, err = number(raw, "opacity", 0, 1); err != nil {
			return v, err
		}
		if v.lighter, _, err = number(raw, "lighter", 0, 100); err != nil {
			return v, err
		}
		if v.darker, _, err = number(raw, "darker", 0, 100); err != nil {
			return v, err
		}
		for field, dst := range map[string]**bool{"bold": &v.bold, "italic": &v.italic} {
			if b, ok := raw[field]; ok {
				bv, ok := b.(bool)
				if !ok {
					return v, fmt.Errorf("%s must be true or false", field)
				}
				*dst = &bv
			}
		}
	default:
		return v, errors.New("must be a color string or a table")
	}

	if r, ok := strings.CutPrefix(name, "colors."); ok {
		if _, known := keyIndex[r]; !known {
			return v, fmt.Errorf("%s refers to an unknown key", name)
		}
		v.ref = r
		return v, nil
	}
	c, err := ParseColor(name)
	if err != nil {
		return v, err
	}
	v.color = c
	return v, nil
}

// number reads the number field of a table, between lo and hi, and
// reports whether the table has it.
func number(table map[string]any, field string, lo, hi float64) (float64, bool, error) {
	raw, ok := table[field]
	if !ok {
		return 0, false, nil
	}
	var f float64
	switch n := raw.(type) {
	case int64:
		f = float64(n)
	case float64:
		f = n
	default:
		return 0, false, fmt.Errorf("%s must be a number", field)
	}
	if f < lo || f > hi {
		return 0, false, fmt.Errorf("%s must be between %g and %g", field, lo, hi)
	}
	return f, true, nil
}

// checkCycles reports a circle of references among the keys the theme
// sets, which no inherited theme can break.
func (t *Theme) checkCycles() error {
	for _, start := range slices.Sorted(maps.Keys(t.values)) {
		seen := map[string]bool{start: true}
		for k := start; ; {
			v, ok := t.values[k]
			if !ok || v.ref == "" || v.ref == k {
				break
			}
			if seen[v.ref] {
				return fmt.Errorf("colors.%s: circular reference", start)
			}
			seen[v.ref] = true
			k = v.ref
		}
	}
	return nil
}

// idFromPath returns the ID of the theme file at path.
func idFromPath(path string) string {
	return strings.TrimSuffix(filepath.Base(path), filepath.Ext(path))
}
