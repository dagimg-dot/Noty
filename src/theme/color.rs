#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Color = Color::rgba(0., 0., 0., 0.);
    pub const BLACK: Color = Color::rgba(0., 0., 0., 1.);
    pub const WHITE: Color = Color::rgba(1., 1., 1., 1.);

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Parses a color the way Qt does, which is what theme files are written
    /// against: `#RGB`, `#RRGGBB`, `#AARRGGBB` (alpha first), the same
    /// without `#`, and a few names.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        match text.to_ascii_lowercase().as_str() {
            "transparent" => return Some(Self::TRANSPARENT),
            "black" => return Some(Self::BLACK),
            "white" => return Some(Self::WHITE),
            _ => {}
        }
        let hex = text.strip_prefix('#').unwrap_or(text);
        if hex.is_empty() || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let channel = |index: usize, digits: usize| {
            let value = u32::from_str_radix(&hex[index * digits..(index + 1) * digits], 16).ok()?;
            Some(value as f32 / ((1u32 << (4 * digits)) - 1) as f32)
        };
        match hex.len() {
            3 => Some(Self::rgba(
                channel(0, 1)?,
                channel(1, 1)?,
                channel(2, 1)?,
                1.,
            )),
            6 => Some(Self::rgba(
                channel(0, 2)?,
                channel(1, 2)?,
                channel(2, 2)?,
                1.,
            )),
            8 => Some(Self::rgba(
                channel(1, 2)?,
                channel(2, 2)?,
                channel(3, 2)?,
                channel(0, 2)?,
            )),
            9 => Some(Self::rgba(
                channel(0, 3)?,
                channel(1, 3)?,
                channel(2, 3)?,
                1.,
            )),
            12 => Some(Self::rgba(
                channel(0, 4)?,
                channel(1, 4)?,
                channel(2, 4)?,
                1.,
            )),
            _ => None,
        }
    }

    /// `#rrggbbaa`, the CSS order gpui-kit themes use.
    pub fn to_hex(self) -> String {
        let byte = |channel: f32| (channel.clamp(0., 1.) * 255.).round() as u8;
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            byte(self.r),
            byte(self.g),
            byte(self.b),
            byte(self.a)
        )
    }

    pub fn with_alpha(self, alpha: f32) -> Self {
        Self {
            a: alpha.clamp(0., 1.),
            ..self
        }
    }

    pub fn mix(self, other: Self, amount: f32) -> Self {
        let amount = amount.clamp(0., 1.);
        let lerp = |from: f32, to: f32| from + (to - from) * amount;
        Self::rgba(
            lerp(self.r, other.r),
            lerp(self.g, other.g),
            lerp(self.b, other.b),
            lerp(self.a, other.a),
        )
    }

    /// Paints this color at `opacity` over `background`, giving an opaque color.
    pub fn over(self, background: Self, opacity: f32) -> Self {
        let opacity = opacity.clamp(0., 1.);
        let blend = |top: f32, bottom: f32| top * opacity + bottom * (1. - opacity);
        Self::rgba(
            blend(self.r, background.r),
            blend(self.g, background.g),
            blend(self.b, background.b),
            1.,
        )
    }

    /// Qt's `QColor::lighter`: 150 is 50% brighter, values under 100 darken.
    pub fn lighter(self, factor: i32) -> Self {
        if factor <= 0 {
            return self;
        }
        if factor < 100 {
            return self.darker(10_000 / factor);
        }
        let (hue, mut saturation, mut value) = self.to_hsv();
        value *= factor as f32 / 100.;
        if value > 1. {
            saturation = (saturation - (value - 1.)).max(0.);
            value = 1.;
        }
        Self::from_hsv(hue, saturation, value, self.a)
    }

    /// Qt's `QColor::darker`: 200 halves the brightness, values under 100 lighten.
    pub fn darker(self, factor: i32) -> Self {
        if factor <= 0 {
            return self;
        }
        if factor < 100 {
            return self.lighter(10_000 / factor);
        }
        let (hue, saturation, value) = self.to_hsv();
        Self::from_hsv(hue, saturation, value * 100. / factor as f32, self.a)
    }

    fn to_hsv(self) -> (f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let delta = max - min;
        let saturation = if max > 0. { delta / max } else { 0. };
        let hue = if delta == 0. {
            0.
        } else if max == self.r {
            60. * ((self.g - self.b) / delta)
        } else if max == self.g {
            60. * ((self.b - self.r) / delta + 2.)
        } else {
            60. * ((self.r - self.g) / delta + 4.)
        };
        (hue.rem_euclid(360.), saturation, max)
    }

    fn from_hsv(hue: f32, saturation: f32, value: f32, alpha: f32) -> Self {
        let chroma = value * saturation;
        let sector = hue / 60.;
        let x = chroma * (1. - (sector.rem_euclid(2.) - 1.).abs());
        let (r, g, b) = match sector as u32 {
            0 => (chroma, x, 0.),
            1 => (x, chroma, 0.),
            2 => (0., chroma, x),
            3 => (0., x, chroma),
            4 => (x, 0., chroma),
            _ => (chroma, 0., x),
        };
        let offset = value - chroma;
        Self::rgba(r + offset, g + offset, b + offset, alpha)
    }

    /// `preferred` when it reads well on this color, otherwise black or
    /// white, whichever contrasts more.
    pub fn readable_text(self, preferred: Self) -> Self {
        if self.contrast(preferred) >= 3. {
            preferred
        } else if self.contrast(Self::WHITE) >= self.contrast(Self::BLACK) {
            Self::WHITE
        } else {
            Self::BLACK
        }
    }

    fn contrast(self, other: Self) -> f32 {
        let (lighter, darker) = {
            let (a, b) = (self.luminance(), other.luminance());
            (a.max(b), a.min(b))
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn luminance(self) -> f32 {
        let linear = |channel: f32| {
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(self.r) + 0.7152 * linear(self.g) + 0.0722 * linear(self.b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readable_text_keeps_good_choices_and_fixes_bad_ones() {
        let blue = Color::parse("#3584e4").unwrap();
        assert_eq!(blue.readable_text(Color::WHITE), Color::WHITE);
        assert_eq!(blue.readable_text(blue), Color::BLACK);
        let peach = Color::parse("#ffc799").unwrap();
        assert_eq!(
            peach
                .readable_text(Color::parse("#101010").unwrap())
                .to_hex(),
            "#101010ff"
        );
    }

    fn hex(text: &str) -> String {
        Color::parse(text).unwrap().to_hex()
    }

    #[test]
    fn parses_the_formats_qt_accepts() {
        assert_eq!(hex("#1a1b26"), "#1a1b26ff");
        assert_eq!(hex("1A1B26"), "#1a1b26ff");
        assert_eq!(hex("#fa0"), "#ffaa00ff");
        assert_eq!(hex("#bbffffff"), "#ffffffbb");
        assert_eq!(hex("#fff000000"), "#ff0000ff");
        assert_eq!(hex("transparent"), "#00000000");
        assert_eq!(hex("White"), "#ffffffff");
    }

    #[test]
    fn rejects_everything_else() {
        for text in [
            "",
            "#",
            "#12",
            "#12345",
            "#gggggg",
            "rebeccapurple",
            "colors.core.accent",
        ] {
            assert!(Color::parse(text).is_none(), "{text:?}");
        }
    }

    #[test]
    fn lighter_and_darker_follow_qt() {
        assert_eq!(
            Color::parse("#808080").unwrap().darker(200).to_hex(),
            "#404040ff"
        );
        assert_eq!(
            Color::parse("#808080").unwrap().lighter(150).to_hex(),
            "#c0c0c0ff"
        );
        assert_eq!(
            Color::parse("#ff0000").unwrap().lighter(200).to_hex(),
            "#ffffffff"
        );
        assert_eq!(
            Color::parse("#808080").unwrap().lighter(50).to_hex(),
            "#404040ff"
        );
        assert_eq!(
            Color::parse("#336699").unwrap().darker(100).to_hex(),
            "#336699ff"
        );
    }

    #[test]
    fn mixing_and_layering() {
        let black = Color::BLACK;
        let white = Color::WHITE;
        assert_eq!(black.mix(white, 0.25).to_hex(), "#404040ff");
        assert_eq!(white.with_alpha(0.1).over(black, 0.5).to_hex(), "#808080ff");
        assert_eq!(white.with_alpha(0.5).to_hex(), "#ffffff80");
    }
}
