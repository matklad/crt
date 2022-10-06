use core::ops;

#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Color {
    pub fn new(r: f64, g: f64, b: f64) -> Color {
        assert!(r >= 0.0 && g >= 0.0 && b >= 0.0, "{r} {g} {b}");
        Color { r, g, b }
    }

    pub fn grayscale(&self) -> f64 {
        (self.r + self.g + self.b) / 3.0
    }
}

impl ops::Mul for Color {
    type Output = Color;

    fn mul(self, rhs: Color) -> Color {
        Color::new(self.r * rhs.r, self.g * rhs.g, self.b * rhs.b)
    }
}

impl ops::Mul<f64> for Color {
    type Output = Color;

    fn mul(self, c: f64) -> Color {
        Color::new(self.r * c, self.g * c, self.b * c)
    }
}

impl ops::Div<f64> for Color {
    type Output = Color;

    fn div(self, c: f64) -> Color {
        Color::new(self.r / c, self.g / c, self.b / c)
    }
}

impl ops::Add for Color {
    type Output = Color;

    fn add(self, rhs: Color) -> Color {
        Color::new(self.r + rhs.r, self.g + rhs.g, self.b + rhs.b)
    }
}
