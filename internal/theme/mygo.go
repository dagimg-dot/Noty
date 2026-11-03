package theme

import (
	"github.com/egoist/mygo/ui"
	"github.com/egoist/mygo/ui/editor"
)

func uiColor(c Color) ui.Color { return ui.RGBA(c.R, c.G, c.B, float32(c.A)/255) }

// UI returns base, the window's theme, with the theme's colors; the
// sizes and fonts of base stay.
func (t *Theme) UI(base ui.Theme) *ui.Theme {
	c := func(key string) ui.Color { return uiColor(t.Color(key)) }
	base.Dark = t.Dark()
	base.Background = c(CoreBackground)
	base.Surface = c("button.background")
	base.SurfaceHover = c("button.hover.background")
	base.SurfacePressed = c("button.pressed.background")
	base.Border = c(CoreBorder)
	base.Text = c("text.default")
	base.TextMuted = c("text.muted")
	base.Accent = c(CoreAccent)
	base.AccentHover = c("button.primary.hover.background")
	base.AccentPressed = c("button.primary.pressed.background")
	base.AccentText = c(CoreAccentForeground)
	base.Danger = c("text.danger")
	base.Warning = c("text.warning")
	base.Success = c("text.success")
	base.Selection = c("text.selection.background")
	base.Focus = c("input.border_focus")
	base.Inverse = c("tooltip.background")
	base.InverseText = c("tooltip.foreground")
	base.Scrollbar = c("scrollbars.background")
	return &base
}

// syntaxTokens maps the syntax keys to the editor's kinds of tokens.
var syntaxTokens = map[string]editor.TokenKind{
	"syntax.keyword":     editor.TokenKeyword,
	"syntax.type":        editor.TokenType,
	"syntax.function":    editor.TokenFunction,
	"syntax.variable":    editor.TokenVariable,
	"syntax.property":    editor.TokenProperty,
	"syntax.constant":    editor.TokenConstant,
	"syntax.number":      editor.TokenNumber,
	"syntax.string":      editor.TokenString,
	"syntax.escape":      editor.TokenEscape,
	"syntax.regexp":      editor.TokenRegexp,
	"syntax.comment":     editor.TokenComment,
	"syntax.operator":    editor.TokenOperator,
	"syntax.punctuation": editor.TokenPunctuation,
	"syntax.tag":         editor.TokenTag,
	"syntax.attribute":   editor.TokenAttribute,
	"syntax.meta":        editor.TokenMeta,
	"syntax.heading":     editor.TokenHeading,
	"syntax.emphasis":    editor.TokenEmphasis,
	"syntax.strong":      editor.TokenStrong,
	"syntax.link":        editor.TokenLink,
	"syntax.code":        editor.TokenCode,
	"syntax.inserted":    editor.TokenInserted,
	"syntax.deleted":     editor.TokenDeleted,
	"syntax.invalid":     editor.TokenInvalid,
}

// Editor returns the code editor's theme. Without syntax highlighting
// every token has the editor's text color.
func (t *Theme) Editor(syntax bool) *editor.Theme {
	c := func(key string) ui.Color { return uiColor(t.Color(key)) }
	et := &editor.Theme{
		Background:        c("editor.background"),
		Text:              c("editor.foreground"),
		LineNumber:        c("editor.gutter.foreground"),
		ActiveLineNumber:  c("editor.gutter.active_foreground"),
		ActiveLine:        c("editor.active_line"),
		Selection:         c("editor.selection.background"),
		InactiveSelection: c("editor.selection.inactive_background"),
		Cursor:            c("editor.cursor"),
		MatchingBracket:   c("editor.bracket_match"),
		Match:             c("editor.find.match"),
		CurrentMatch:      c("editor.find.current_match"),
		Occurrence:        c("editor.occurrence"),
		IndentGuide:       c("editor.indent_guide"),
		ActiveIndentGuide: c("editor.active_indent_guide"),
		WrapGuide:         c("editor.wrap_guide"),
		Whitespace:        c("editor.whitespace"),
		Error:             c("editor.error"),
		Warning:           c("editor.warning"),
		Info:              c("editor.info"),
		Hint:              c("editor.hint"),
		Added:             c("editor.added"),
		Deleted:           c("editor.deleted"),
		Modified:          c("editor.modified"),
	}
	if !syntax {
		return et
	}
	for key, kind := range syntaxTokens {
		s := t.Style(key)
		et.Tokens[kind] = editor.TokenStyle{Color: uiColor(s.Color), Bold: s.Bold, Italic: s.Italic}
	}
	return et
}
