package theme

import (
	"fmt"
	"math"
	"strconv"
	"strings"
)

// Color is a color with straight (not premultiplied) alpha.
type Color struct{ R, G, B, A uint8 }

// ParseColor reads "#rgb", "#rgba", "#rrggbb" or "#rrggbbaa", with or
// without the "#", or "transparent". The alpha comes last, as in CSS.
func ParseColor(s string) (Color, error) {
	s = strings.TrimSpace(s)
	if strings.EqualFold(s, "transparent") {
		return Color{}, nil
	}
	h := strings.TrimPrefix(s, "#")
	switch len(h) {
	case 3, 4:
		var long strings.Builder
		for _, c := range h {
			long.WriteRune(c)
			long.WriteRune(c)
		}
		h = long.String()
	case 6, 8:
	default:
		return Color{}, fmt.Errorf("%q is not a color: use #rrggbb or #rrggbbaa", s)
	}
	if len(h) == 6 {
		h += "ff"
	}
	v, err := strconv.ParseUint(h, 16, 32)
	if err != nil {
		return Color{}, fmt.Errorf("%q is not a color: use #rrggbb or #rrggbbaa", s)
	}
	return Color{R: uint8(v >> 24), G: uint8(v >> 16), B: uint8(v >> 8), A: uint8(v)}, nil
}

// Hex returns the color as "#rrggbb", or "#rrggbbaa" when it is not
// opaque.
func (c Color) Hex() string {
	if c.A == 255 {
		return fmt.Sprintf("#%02x%02x%02x", c.R, c.G, c.B)
	}
	return fmt.Sprintf("#%02x%02x%02x%02x", c.R, c.G, c.B, c.A)
}

func (c Color) String() string { return c.Hex() }

// WithAlpha multiplies the color's alpha by a, between 0 and 1.
func (c Color) WithAlpha(a float64) Color {
	c.A = clampByte(float64(c.A) * clamp01(a))
	return c
}

// Mix returns the color t of the way from c to o, t between 0 and 1.
func (c Color) Mix(o Color, t float64) Color {
	t = clamp01(t)
	mix := func(a, b uint8) uint8 { return clampByte(float64(a) + (float64(b)-float64(a))*t) }
	return Color{R: mix(c.R, o.R), G: mix(c.G, o.G), B: mix(c.B, o.B), A: mix(c.A, o.A)}
}

// Lighter mixes the color toward white by p, between 0 and 1, keeping
// its alpha.
func (c Color) Lighter(p float64) Color {
	w := Color{255, 255, 255, c.A}
	return c.Mix(w, p)
}

// Darker mixes the color toward black by p, between 0 and 1, keeping its
// alpha.
func (c Color) Darker(p float64) Color {
	b := Color{0, 0, 0, c.A}
	return c.Mix(b, p)
}

// Luminance returns the relative luminance of the color, between 0
// (black) and 1 (white), ignoring alpha.
func (c Color) Luminance() float64 {
	lin := func(v uint8) float64 {
		s := float64(v) / 255
		if s <= 0.04045 {
			return s / 12.92
		}
		return math.Pow((s+0.055)/1.055, 2.4)
	}
	return 0.2126*lin(c.R) + 0.7152*lin(c.G) + 0.0722*lin(c.B)
}

// contrasting returns black or white, whichever reads better on c.
func contrasting(c Color) Color {
	// The luminance at which black and white have equal contrast.
	if c.Luminance() > 0.179 {
		return Color{0, 0, 0, 255}
	}
	return Color{255, 255, 255, 255}
}

func clamp01(v float64) float64 { return math.Min(math.Max(v, 0), 1) }

func clampByte(v float64) uint8 { return uint8(math.Round(math.Min(math.Max(v, 0), 255))) }
