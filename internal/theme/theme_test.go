package theme

import (
	"os"
	"path/filepath"
	"slices"
	"strings"
	"testing"

	"github.com/egoist/mygo/ui"
	"github.com/egoist/mygo/ui/editor"
)

func hex(t *testing.T, s string) Color {
	t.Helper()
	c, err := ParseColor(s)
	if err != nil {
		t.Fatal(err)
	}
	return c
}

func writeTheme(t *testing.T, dir, id, body string) string {
	t.Helper()
	p := filepath.Join(dir, id+".toml")
	if err := os.WriteFile(p, []byte(body), 0o644); err != nil {
		t.Fatal(err)
	}
	return p
}

func TestParseColor(t *testing.T) {
	for in, want := range map[string]Color{
		"#fff":        {255, 255, 255, 255},
		"#0f08":       {0, 255, 0, 136},
		"#112233":     {0x11, 0x22, 0x33, 255},
		"112233":      {0x11, 0x22, 0x33, 255},
		"#11223344":   {0x11, 0x22, 0x33, 0x44},
		"transparent": {},
	} {
		if got, err := ParseColor(in); err != nil || got != want {
			t.Errorf("ParseColor(%q) = %v, %v; want %v", in, got, err, want)
		}
	}
	for _, bad := range []string{"", "#12", "#ggg", "red", "#1234567"} {
		if _, err := ParseColor(bad); err == nil {
			t.Errorf("ParseColor(%q) accepted", bad)
		}
	}
	if got := (Color{0x11, 0x22, 0x33, 0x44}).Hex(); got != "#11223344" {
		t.Errorf("Hex %q", got)
	}
}

func TestColorOps(t *testing.T) {
	c := Color{100, 100, 100, 255}
	if got := c.Lighter(0.5); got != (Color{178, 178, 178, 255}) {
		t.Errorf("Lighter %v", got)
	}
	if got := c.Darker(0.5); got != (Color{50, 50, 50, 255}) {
		t.Errorf("Darker %v", got)
	}
	if got := c.WithAlpha(0.5); got.A != 128 {
		t.Errorf("WithAlpha %v", got)
	}
	if contrasting(Color{255, 255, 0, 255}) != (Color{0, 0, 0, 255}) || contrasting(Color{0, 0, 128, 255}) != (Color{255, 255, 255, 255}) {
		t.Error("contrasting")
	}
}

func TestKeysAreNotPrefixesOfEachOther(t *testing.T) {
	keys := Keys()
	for _, a := range keys {
		for _, b := range keys {
			if a != b && strings.HasPrefix(b, a+".") {
				t.Errorf("%s is a table of %s", a, b)
			}
		}
	}
	for key := range syntaxTokens {
		if keyIndex[key] == nil {
			t.Errorf("syntax token of unknown key %s", key)
		}
	}
	for _, k := range keys {
		if isSyntaxKey(k) {
			if _, ok := syntaxTokens[k]; !ok {
				t.Errorf("syntax key %s has no token kind", k)
			}
		}
	}
}

func TestBundledThemes(t *testing.T) {
	db := NewDB()
	for _, p := range db.Problems() {
		t.Errorf("problem: %s", p)
	}
	ids := []string{}
	for _, th := range db.Themes() {
		ids = append(ids, th.ID)
		for _, k := range Keys() {
			if th.Color(k) == (Color{}) {
				t.Errorf("%s: %s is transparent", th.ID, k)
			}
		}
	}
	want := []string{"coldark-dark", "nord", "noty-dark", "noty-light", "tokyo-night"}
	slices.Sort(ids)
	if !slices.Equal(ids, want) {
		t.Errorf("themes %q, want %q", ids, want)
	}
	if got := db.Pick("noty-classic", true); got.ID != RootLight {
		t.Errorf("legacy noty-classic picks %s", got.ID)
	}
	if got := db.Pick("missing", true); got.ID != RootDark {
		t.Errorf("missing dark picks %s", got.ID)
	}
	if got := db.Pick("missing", false); got.ID != RootLight {
		t.Errorf("missing light picks %s", got.ID)
	}
}

// A user theme setting a few colors inherits the rest, and references
// and derivations follow its colors, also through what it inherits.
func TestInheritanceAndReferences(t *testing.T) {
	dir := t.TempDir()
	writeTheme(t, dir, "mine", `
[meta]
name = "Mine"
description = "test"
variant = "dark"
inherits = "nord"

[colors.core]
background = "#000000"

[colors.accents]
cyan = "#00ffff"

[colors.syntax]
string = { name = "colors.accents.cyan", darker = 50 }
comment = { name = "colors.syntax.comment", italic = false }
heading = { name = "colors.syntax.heading", opacity = 0.5 }
`)
	db := NewDB(dir)
	for _, p := range db.Problems() {
		t.Errorf("problem: %s", p)
	}
	th := db.Get("mine")
	if th == nil || th.Parent() != db.Get("nord") {
		t.Fatalf("mine not loaded or not inheriting nord: %+v", th)
	}
	nord := db.Get("nord")

	if got := th.Color(CoreBackground); got != hex(t, "#000000") {
		t.Errorf("own background %v", got)
	}
	if got, want := th.Color(CoreForeground), nord.Color(CoreForeground); got != want {
		t.Errorf("inherited foreground %v, want %v", got, want)
	}
	// Derived in the theme shown: the editor's background follows its
	// core background.
	if got := th.Color("editor.background"); got != hex(t, "#000000") {
		t.Errorf("derived editor background %v", got)
	}
	// Nord's code = "colors.accents.cyan" resolves in the theme shown.
	if got := th.Color("syntax.code"); got != hex(t, "#00ffff") {
		t.Errorf("inherited reference %v, want the new cyan", got)
	}
	if got := th.Color("syntax.string"); got != hex(t, "#008080") {
		t.Errorf("darker reference %v", got)
	}
	// A reference to itself is the inherited color.
	if got := th.Style("syntax.comment"); got.Italic || got.Color != nord.Color("syntax.comment") {
		t.Errorf("self reference %+v", got)
	}
	nh := nord.Style("syntax.heading")
	if got := th.Style("syntax.heading"); got.Color != nh.Color.WithAlpha(0.5) || !got.Bold {
		t.Errorf("heading %+v, nord's %+v", got, nh)
	}
}

func TestParseErrorsAndWarnings(t *testing.T) {
	for name, body := range map[string]string{
		"no meta":   `[colors.core]` + "\nbackground = \"#000\"",
		"no name":   "[meta]\nvariant = \"dark\"",
		"bad var":   "[meta]\nname = \"x\"\nvariant = \"dim\"",
		"bad toml":  "[meta\nname = 1",
		"circular":  "[meta]\nname = \"x\"\nvariant = \"dark\"\n[colors.core]\naccent = \"colors.core.border\"\nborder = \"colors.core.accent\"",
		"not table": "colors = 1\n[meta]\nname = \"x\"\nvariant = \"dark\"",
	} {
		if _, _, err := Parse("x", "", []byte(body)); err == nil {
			t.Errorf("%s: no error", name)
		}
	}

	th, warnings, err := Parse("x", "", []byte(`
[meta]
name = "x"
variant = "light"
colour = "typo"

[colors.core]
background = "#nothex"
foreground = "#123456"
shadow = "#000"

[colors.editor]
cursor = { name = "#fff", bold = true }
gutter = { foreground = "colors.no.such" }

[colors.syntax]
keyword = { name = "#fff", opacity = 2 }
`))
	if err != nil {
		t.Fatal(err)
	}
	if th.Sets(CoreBackground) || !th.Sets(CoreForeground) || th.Sets("editor.cursor") || th.Sets("syntax.keyword") {
		t.Error("invalid values were kept")
	}
	joined := strings.Join(warnings, "\n")
	for _, want := range []string{"meta.colour", "colors.core.shadow", "colors.core.background", "bold is only for syntax keys", "unknown key", "opacity must be between"} {
		if !strings.Contains(joined, want) {
			t.Errorf("no warning about %q in:\n%s", want, joined)
		}
	}
}

func TestDBPrecedenceAndProblems(t *testing.T) {
	user, sys := t.TempDir(), t.TempDir()
	writeTheme(t, user, "nord", "[meta]\nname = \"My Nord\"\nvariant = \"dark\"\n[colors.core]\nbackground = \"#010101\"")
	writeTheme(t, sys, "nord", "[meta]\nname = \"System Nord\"\nvariant = \"dark\"")
	writeTheme(t, user, "noty-dark", "[meta]\nname = \"Hijack\"\nvariant = \"dark\"")
	writeTheme(t, user, "broken", "[meta\n")
	writeTheme(t, user, "orphan", "[meta]\nname = \"Orphan\"\nvariant = \"light\"\ninherits = \"gone\"")
	writeTheme(t, user, "a", "[meta]\nname = \"A\"\nvariant = \"dark\"\ninherits = \"b\"")
	writeTheme(t, user, "b", "[meta]\nname = \"B\"\nvariant = \"dark\"\ninherits = \"a\"")
	writeTheme(t, user, "lr", "[meta]\nname = \"LR\"\nvariant = \"dark\"\n[colors.core]\naccent = \"colors.core.border\"")
	writeTheme(t, sys, "lr-child", "[meta]\nname = \"LR child\"\nvariant = \"dark\"\ninherits = \"lr\"\n[colors.core]\nborder = \"colors.core.accent\"")
	os.WriteFile(filepath.Join(user, "notes.txt"), []byte("not a theme"), 0o644)

	db := NewDB(user, sys)
	if got := db.Get("nord"); got.Name != "My Nord" || got.Color(CoreBackground) != hex(t, "#010101") {
		t.Errorf("user theme does not replace bundled and system ones: %s", got.Name)
	}
	if got := db.Get(RootDark); got.Name != "Noty Dark" {
		t.Errorf("root replaced by %s", got.Name)
	}
	if db.Get("broken") != nil {
		t.Error("broken theme loaded")
	}
	if got := db.Get("orphan"); got == nil || got.Parent() != db.Get(RootLight) {
		t.Errorf("orphan should fall back to the light root: %+v", got)
	}
	if a, b := db.Get("a"), db.Get("b"); a.Parent() == nil || b.Parent() == nil ||
		(a.Parent() == b && b.Parent() == a) {
		t.Error("inheritance loop kept")
	}

	problems := []string{}
	for _, p := range db.Problems() {
		problems = append(problems, p.String())
	}
	joined := strings.Join(problems, "\n")
	for _, want := range []string{"noty-dark is reserved", "broken.toml", "does not exist", "inherits it back", "lr-child.toml: colors.core.accent: circular reference"} {
		if !strings.Contains(joined, want) {
			t.Errorf("no problem about %q in:\n%s", want, joined)
		}
	}
	// A circle across themes resolves to transparent instead of hanging.
	if got := db.Get("lr-child").Color(CoreAccent); got != (Color{}) {
		t.Errorf("circular color %v", got)
	}
}

func TestImport(t *testing.T) {
	src := t.TempDir()
	dst := filepath.Join(t.TempDir(), "themes")
	good := writeTheme(t, src, "sunset", "[meta]\nname = \"Sunset\"\nvariant = \"light\"")
	id, err := importTo(dst, good, false)
	if err != nil || id != "sunset" {
		t.Fatalf("import: %q, %v", id, err)
	}
	if _, err := importTo(dst, good, false); err == nil {
		t.Error("imported over an existing theme")
	}
	if _, err := importTo(dst, good, true); err != nil {
		t.Errorf("replace: %v", err)
	}
	if _, err := importTo(dst, writeTheme(t, src, "bad", "[meta]\nname = 1"), false); err == nil {
		t.Error("imported an invalid theme")
	}
	if _, err := importTo(dst, writeTheme(t, src, RootLight, "[meta]\nname = \"x\"\nvariant = \"light\""), false); err == nil {
		t.Error("imported a root ID")
	}
	if db := NewDB(dst); db.Get("sunset") == nil {
		t.Error("imported theme not found")
	}
}

func TestMygoThemes(t *testing.T) {
	db := NewDB()
	th := db.Get("tokyo-night")
	base := ui.Theme{FontSize: 15, Radius: 6}
	u := th.UI(base)
	if !u.Dark || u.FontSize != 15 || u.Radius != 6 {
		t.Errorf("UI theme lost the base: %+v", u)
	}
	if bg := th.Color(CoreBackground); u.Background.R != bg.R || u.Background.G != bg.G || u.Background.A != 255 {
		t.Errorf("background %v, want %v", u.Background, bg)
	}

	et := th.Editor(true)
	kw := th.Style("syntax.keyword")
	if got := et.Tokens[editor.TokenKeyword].Color; got.R != kw.Color.R || got.B != kw.Color.B {
		t.Errorf("keyword %v, want %v", got, kw.Color)
	}
	if !et.Tokens[editor.TokenHeading].Bold || !et.Tokens[editor.TokenComment].Italic {
		t.Error("default font styles lost")
	}
	if plain := th.Editor(false); plain.Tokens[editor.TokenKeyword] != (editor.TokenStyle{}) {
		t.Error("tokens styled without syntax highlighting")
	}
}

// docs/themes.md documents every key.
func TestDocsListEveryKey(t *testing.T) {
	data, err := os.ReadFile("../../docs/themes.md")
	if err != nil {
		t.Fatal(err)
	}
	for _, k := range Keys() {
		if !strings.Contains(string(data), "`"+k+"`") {
			t.Errorf("docs/themes.md does not mention %s", k)
		}
	}
}
