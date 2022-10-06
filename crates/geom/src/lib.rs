#![no_std]
use core::{fmt, num::ParseFloatError, ops, str::FromStr};

#[derive(Clone, Copy, PartialEq, Debug, Default)]
#[allow(non_camel_case_types)]
pub struct v64 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// {0}
#[derive(Debug, displaydoc::Display)]
pub struct ParseVectorError(ParseVectorErrorRepr);

#[derive(Debug, displaydoc::Display)]
enum ParseVectorErrorRepr {
    /// {0}
    ParseFloatError(ParseFloatError),
    /// expected three coma-separated coordinates
    InvalidFormat,
}

#[derive(Clone, Copy)]
pub struct Ray {
    origin: v64,
    dir: v64,
}

pub const fn v64(x: f64, y: f64, z: f64) -> v64 {
    v64 { x, y, z }
}

impl v64 {
    pub const ZERO: v64 = v64(0.0, 0.0, 0.0);

    pub fn xyz(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
    pub fn to_unit(self) -> v64 {
        self / self.norm()
    }
    pub fn norm(self) -> f64 {
        sqrt(self.norm_squared())
    }
    pub fn norm_squared(self) -> f64 {
        dot(self, self)
    }
}

impl fmt::Display for v64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [x, y, z] = self.xyz();
        write!(f, "{x},{y},{z}")
    }
}

impl FromStr for v64 {
    type Err = ParseVectorError;

    fn from_str(s: &str) -> Result<v64, ParseVectorError> {
        let [x, y, z] = split_n::<3>(s, ',')
            .ok_or(ParseVectorErrorRepr::InvalidFormat)?
            .map(|it| it.parse::<f64>().map_err(ParseVectorErrorRepr::ParseFloatError));
        Ok(v64(x?, y?, z?))
    }
}

fn split_n<const N: usize>(s: &str, p: char) -> Option<[&str; N]> {
    let mut components = s.split(p);
    let mut res = [""; N];
    for i in 0..N {
        res[i] = components.next()?;
    }
    if components.next().is_some() {
        return None;
    }
    Some(res)
}

pub fn dot(lhs: v64, rhs: v64) -> f64 {
    lhs.x * rhs.x + lhs.y * rhs.y + lhs.z * rhs.z
}
pub fn cross(lhs: v64, rhs: v64) -> v64 {
    let [lx, ly, lz] = lhs.xyz();
    let [rx, ry, rz] = rhs.xyz();
    v64(ly * rz - lz * ry, -(lx * rz - lz * rx), lx * ry - ly * rx)
}

impl ops::Neg for v64 {
    type Output = v64;

    fn neg(self) -> v64 {
        v64(-self.x, -self.y, -self.z)
    }
}

impl ops::Add for v64 {
    type Output = v64;

    fn add(self, rhs: v64) -> v64 {
        v64(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl ops::Sub for v64 {
    type Output = v64;

    fn sub(self, rhs: v64) -> v64 {
        v64(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl ops::Div<f64> for v64 {
    type Output = v64;

    fn div(self, c: f64) -> v64 {
        let r = 1.0 / c;
        v64(self.x * r, self.y * r, self.z * r)
    }
}

impl ops::Mul<f64> for v64 {
    type Output = v64;

    fn mul(self, c: f64) -> v64 {
        v64(self.x * c, self.y * c, self.z * c)
    }
}

impl ops::Mul<v64> for f64 {
    type Output = v64;

    fn mul(self, v: v64) -> v64 {
        v * self
    }
}

impl From<ParseVectorErrorRepr> for ParseVectorError {
    fn from(repr: ParseVectorErrorRepr) -> ParseVectorError {
        ParseVectorError(repr)
    }
}

impl Ray {
    pub fn new(origin: v64, dir: v64) -> Ray {
        let dir = dir.to_unit();
        Ray { origin, dir }
    }

    pub fn from_to(from: v64, to: v64) -> Ray {
        Ray::new(from, to - from)
    }

    pub fn origin(&self) -> v64 {
        self.origin
    }

    pub fn dir(&self) -> v64 {
        self.dir
    }

    pub fn at(&self, dt: f64) -> v64 {
        self.origin + self.dir * dt
    }
}

fn sqrt(mut v: f64) -> f64 {
    unsafe {
        core::arch::asm!(
            "vsqrtsd {v}, {v}, {v}",
            v = inout(xmm_reg) v,
        );
    }
    v
}
