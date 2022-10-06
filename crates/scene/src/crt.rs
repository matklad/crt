use core::{
    fmt,
    iter::Peekable,
    num::{ParseFloatError, ParseIntError},
    str::SplitAsciiWhitespace,
};

use geom::{v64, ParseVectorError, Ray};
use mem::{Mem, Oom};

use crate::{color::Color, Camera, Light, Material, Mesh, MeshFace, Plane, Scene, Sphere};

#[derive(Debug, displaydoc::Display)]
pub struct ParseSceneError<'a> {
    kind: ErrorKind,
    context: [&'a str; 4],
}

#[derive(Debug, displaydoc::Display)]
enum ErrorKind {
    /// unexpected end of file
    UnexpectedEof,
    /// invalid color: {0}
    ParseColorError(ParseColorError),
    /// expected `{0}`
    Expected(&'static str),
    /// invalid vector: {0}
    ParseVectorError(ParseVectorError),
    /// invalid scalar: {0}
    ParseFloatError(ParseFloatError),
    /// invalid dimensions
    InvalidDim,
    /// invalid key
    InvalidKey,
    /// out of memory
    Oom(Oom),
    /// invalid mesh face
    InvalidFace,
    /// invalid mesh face index: {0}
    InvalidFaceIndex(ParseIntError),
    /// mesh face index out of bounds
    FaceIndexOutOfBounds,
}

#[derive(Debug, displaydoc::Display)]
enum ParseColorError {
    /// Invalid color format, expected `#00aa9f`
    InvalidFormat,
    /// Invalid hex value, {0}
    InvalidColorValue(ParseIntError),
}

pub(crate) fn parse<'m, 'i>(
    mem: &mut Mem<'m>,
    input: &'i str,
) -> Result<Scene<'m>, ParseSceneError<'i>> {
    let mut n_spheres = 0;
    let mut n_planes = 0;
    let mut n_meshes = 0;
    for word in input.split_ascii_whitespace() {
        match word {
            "sphere" => n_spheres += 1,
            "plane" => n_planes += 1,
            "mesh" => n_meshes += 1,
            _ => (),
        }
    }

    let spheres = mem
        .alloc_array_default(n_spheres)
        .map_err(|oom| ParseSceneError { kind: ErrorKind::Oom(oom), context: [""; 4] })?;
    let planes = mem
        .alloc_array_default(n_planes)
        .map_err(|oom| ParseSceneError { kind: ErrorKind::Oom(oom), context: [""; 4] })?;
    let meshes = mem
        .alloc_array_default(n_meshes)
        .map_err(|oom| ParseSceneError { kind: ErrorKind::Oom(oom), context: [""; 4] })?;
    let mut res = Scene {
        background: Default::default(),
        foreground: Default::default(),
        camera: Default::default(),
        light: Default::default(),
        spheres,
        planes,
        meshes,
    };
    let mut p = Parser::new(mem, input);
    match scene(&mut p, &mut res) {
        Ok(()) => Ok(res),
        Err(kind) => Err(ParseSceneError { kind, context: p.context }),
    }
}

impl<'a> fmt::Display for ParseSceneError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "in {}", self.context[0])?;
        for &ctx in &self.context[1..] {
            if ctx.is_empty() {
                break;
            }
            write!(f, ".{}", ctx)?
        }
        write!(f, ": {}", self.kind)
    }
}

struct Parser<'m, 'i, 'a> {
    mem: &'a mut Mem<'m>,
    context: [&'i str; 4],
    depth: usize,
    words: Peekable<SplitAsciiWhitespace<'i>>,
}

impl<'m, 'i, 'a> Parser<'m, 'i, 'a> {
    fn new(mem: &'a mut Mem<'m>, input: &'i str) -> Parser<'m, 'i, 'a> {
        Parser { mem, context: [""; 4], depth: 0, words: input.split_ascii_whitespace().peekable() }
    }
    fn next(&mut self) -> Result<&'i str, ErrorKind> {
        let res = self.words.next().ok_or(ErrorKind::UnexpectedEof)?;
        Ok(res)
    }
    fn at(&mut self, token: &'static str) -> bool {
        self.words.peek() == Some(&token)
    }
    fn expect(&mut self, token: &'static str) -> Result<(), ErrorKind> {
        let next = self.next()?;
        if next != token {
            Err(ErrorKind::Expected(token))?
        }
        Ok(())
    }

    fn push_next(&mut self) -> Result<&'i str, ErrorKind> {
        let res = self.next()?;
        self.push(res);
        Ok(res)
    }
    fn push(&mut self, ctx: &'i str) {
        if self.depth < self.context.len() {
            self.context[self.depth] = ctx;
            self.depth += 1;
        }
    }
    fn pop(&mut self) {
        self.depth -= 1;
        self.context[self.depth] = "";
    }
}

fn scene<'m, 'i>(p: &mut Parser<'m, 'i, '_>, res: &mut Scene<'m>) -> Result<(), ErrorKind> {
    p.push("scene");
    let mut spheres = res.spheres.iter_mut();
    let mut planes = res.planes.iter_mut();
    let mut meshes = res.meshes.iter_mut();
    while let Ok(w) = p.push_next() {
        match w {
            "background" => res.background = color(p)?,
            "foreground" => res.foreground = color(p)?,
            "camera" => camera(p, &mut res.camera)?,
            "sphere" => sphere(p, spheres.next().unwrap())?,
            "plane" => plane(p, planes.next().unwrap())?,
            "mesh" => mesh(p, meshes.next().unwrap())?,
            "light" => light(p, &mut res.light)?,
            _ => Err(ErrorKind::InvalidKey)?,
        }
        p.pop();
    }
    p.pop();
    Ok(())
}

fn color<'m, 'i>(p: &mut Parser<'m, 'i, '_>) -> Result<Color, ErrorKind> {
    let s = p.next()?;
    if !(s.starts_with('#') && s.len() == 7 && s.is_ascii()) {
        Err(ErrorKind::ParseColorError(ParseColorError::InvalidFormat))?
    }
    let [r, g, b] = [&s[1..3], &s[3..5], &s[5..7]].map(|it| {
        u8::from_str_radix(it, 16)
            .map_err(|err| ErrorKind::ParseColorError(ParseColorError::InvalidColorValue(err)))
    });
    let [r, g, b] = [r?, g?, b?].map(|it| (it as f64) / 255.0);
    let color = Color::new(r, g, b);
    Ok(color)
}

fn camera<'m, 'i>(p: &mut Parser<'m, 'i, '_>, res: &mut Camera) -> Result<(), ErrorKind> {
    p.expect("{")?;
    while !p.at("}") {
        match p.push_next()? {
            "pos" => res.pos = vector(p)?,
            "look_at" => res.look_at = vector(p)?,
            "up" => res.up = vector(p)?,
            "focus" => res.focus = scalar(p)?,
            "dim" => (res.width, res.height) = dim(p)?,
            _ => Err(ErrorKind::InvalidKey)?,
        }
        p.pop();
    }
    p.expect("}")?;
    Ok(())
}

fn sphere<'m, 'i>(p: &mut Parser<'m, 'i, '_>, res: &mut Sphere) -> Result<(), ErrorKind> {
    p.expect("{")?;
    while !p.at("}") {
        match p.push_next()? {
            "pos" => res.center = vector(p)?,
            "radius" => res.radius = scalar(p)?,
            "material" => material(p, &mut res.material)?,
            _ => Err(ErrorKind::InvalidKey)?,
        }
        p.pop()
    }
    p.expect("}")
}

fn plane<'m, 'i>(p: &mut Parser<'m, 'i, '_>, res: &mut Plane) -> Result<(), ErrorKind> {
    p.expect("{")?;
    let mut pos = v64::ZERO;
    let mut dir = v64(0.0, 0.0, 1.0);
    while !p.at("}") {
        match p.push_next()? {
            "pos" => pos = vector(p)?,
            "normal" => dir = vector(p)?,
            "material" => material(p, &mut res.material)?,
            _ => Err(ErrorKind::InvalidKey)?,
        }
        p.pop()
    }
    res.normal = Ray::new(pos, dir);
    p.expect("}")
}

fn mesh<'m, 'i>(p: &mut Parser<'m, 'i, '_>, res: &mut Mesh<'m>) -> Result<(), ErrorKind> {
    p.expect("{")?;
    while !p.at("}") {
        match p.push_next()? {
            "material" => material(p, &mut res.material)?,
            "data" => {
                p.expect("{")?;

                let mut words = p.words.clone();
                let mut n_v = 0;
                let mut n_n = 0;
                let mut n_f = 0;
                loop {
                    match words.next() {
                        Some("}") | None => break,
                        Some("v") => n_v += 1,
                        Some("vn") => n_n += 1,
                        Some("f") => n_f += 1,
                        _ => (),
                    }
                }
                res.v = p.mem.alloc_array_default(n_v).map_err(ErrorKind::Oom)?;
                res.n = p.mem.alloc_array_default(n_n).map_err(ErrorKind::Oom)?;
                res.f = p.mem.alloc_array_default(n_f).map_err(ErrorKind::Oom)?;
                let mut v = res.v.iter_mut();
                let mut n = res.n.iter_mut();
                let mut f = res.f.iter_mut();

                while !p.at("}") {
                    match p.push_next()? {
                        "v" => *v.next().unwrap() = vector(p)?,
                        "vn" => *n.next().unwrap() = vector(p)?,
                        "f" => face(p, n_v as u32, n_n as u32, f.next().unwrap())?,
                        _ => Err(ErrorKind::InvalidKey)?,
                    }
                    p.pop()
                }

                p.expect("}")?;
            }
            _ => Err(ErrorKind::InvalidKey)?,
        }
        p.pop()
    }
    p.expect("}")
}

fn face<'m, 'i>(
    p: &mut Parser<'m, 'i, '_>,
    n_v: u32,
    n_n: u32,
    res: &mut MeshFace,
) -> Result<(), ErrorKind> {
    for i in 0..3 {
        let f = p.next()?;
        let [vi, ni] = split_n(f, '/')
            .ok_or(ErrorKind::InvalidFace)?
            .map(|i| i.parse::<u32>().map_err(ErrorKind::InvalidFaceIndex));
        let vi = vi?.wrapping_sub(1);
        let ni = ni?.wrapping_sub(1);
        if !(vi < n_v && ni < n_n) {
            Err(ErrorKind::FaceIndexOutOfBounds)?
        }
        res.v[i] = vi;
        res.n[i] = ni;
    }
    Ok(())
}

fn material<'m, 'i>(p: &mut Parser<'m, 'i, '_>, res: &mut Material) -> Result<(), ErrorKind> {
    p.expect("{")?;
    while !p.at("}") {
        match p.push_next()? {
            "color" => res.color = color(p)?,
            "diffuse" => res.diffuse = scalar(p)?,
            _ => Err(ErrorKind::InvalidKey)?,
        }
        p.pop()
    }
    p.expect("}")
}

fn light<'m, 'i>(p: &mut Parser<'m, 'i, '_>, res: &mut Light) -> Result<(), ErrorKind> {
    p.expect("{")?;
    while !p.at("}") {
        match p.push_next()? {
            "color" => res.color = color(p)?,
            "pos" => res.pos = vector(p)?,
            _ => Err(ErrorKind::InvalidKey)?,
        }
        p.pop()
    }
    p.expect("}")
}

fn scalar<'m, 'i>(p: &mut Parser<'m, 'i, '_>) -> Result<f64, ErrorKind> {
    let t = p.next()?;
    let f = t.parse::<f64>().map_err(ErrorKind::ParseFloatError)?;
    Ok(f)
}

fn vector<'m, 'i>(p: &mut Parser<'m, 'i, '_>) -> Result<v64, ErrorKind> {
    let t = p.next()?;
    let v = t.parse::<v64>().map_err(ErrorKind::ParseVectorError)?;
    Ok(v)
}

fn dim<'m, 'i>(p: &mut Parser<'m, 'i, '_>) -> Result<(f64, f64), ErrorKind> {
    let t = p.next()?;

    let [w, h] = split_n(t, 'x')
        .ok_or(ErrorKind::InvalidDim)?
        .map(|it| it.parse().map_err(ErrorKind::ParseFloatError));
    Ok((w?, h?))
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
