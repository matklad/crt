use core::{fmt, num::ParseIntError, str::FromStr};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// {0}
#[derive(Debug, displaydoc::Display)]
pub struct ParseColorError(ParseColorErrorRepr);

#[derive(Debug, displaydoc::Display)]
enum ParseColorErrorRepr {
    /// Invalid color format, expected `#00aa9f`
    InvalidFormat,
    /// Invalid hex value, {0}
    InvalidColorValue(ParseIntError),
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b }
    }
    pub fn new_red(value: u8) -> Color {
        Color::new(value, 0, 0)
    }
    pub fn new_green(value: u8) -> Color {
        Color::new(0, value, 0)
    }
    pub fn new_blue(value: u8) -> Color {
        Color::new(0, 0, value)
    }
    pub fn new_black() -> Color {
        Color::new(0, 0, 0)
    }
    pub fn new_white() -> Color {
        Color::new(!0, !0, !0)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Color { r, g, b } = self;
        write!(f, "#{r:2x}{g:2x}{b:2x}")
    }
}

impl FromStr for Color {
    type Err = ParseColorError;

    fn from_str(s: &str) -> Result<Color, ParseColorError> {
        if !(s.starts_with('#') && s.len() == 7 && s.is_ascii()) {
            Err(ParseColorErrorRepr::InvalidFormat)?
        }
        let [r, g, b] = [&s[1..3], &s[3..5], &s[5..7]]
            .map(|it| u8::from_str_radix(it, 16).map_err(ParseColorErrorRepr::InvalidColorValue));
        Ok(Color { r: r?, g: g?, b: b? })
    }
}

impl From<ParseColorErrorRepr> for ParseColorError {
    fn from(repr: ParseColorErrorRepr) -> ParseColorError {
        ParseColorError(repr)
    }
}
