#![no_std]
mod crt;
mod color;

use geom::{v64, Ray};
use mem::Mem;

pub use crate::{color::Color, crt::ParseSceneError};

pub struct Scene<'m> {
    pub background: Color,
    pub foreground: Color,
    pub camera: Camera,
    pub light: Light,
    pub spheres: &'m mut [Sphere],
    pub planes: &'m mut [Plane],
    pub meshes: &'m mut [Mesh<'m>],
}

#[derive(Default)]
pub struct Camera {
    pub pos: v64,
    pub look_at: v64,
    pub up: v64,
    pub focus: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Default)]
pub struct Light {
    pub pos: v64,
    pub color: Color,
}

#[derive(Default, Clone)]
pub struct Sphere {
    pub center: v64,
    pub radius: f64,
    pub material: Material,
}

#[derive(Clone)]
pub struct Plane {
    pub normal: Ray,
    pub material: Material,
}

pub struct Mesh<'m> {
    pub v: &'m mut [v64],
    pub n: &'m mut [v64],
    pub f: &'m mut [MeshFace],
    pub material: Material,
}

#[derive(Default)]
pub struct MeshFace {
    v: [u32; 3],
    n: [u32; 3],
}

pub struct Triangle {
    pub v: [v64; 3],
    pub n: [v64; 3],
}

#[derive(Default, Clone)]
pub struct Material {
    pub color: Color,
    pub diffuse: f64,
    pub specular: f64,
    pub reflectance: f64,
}

impl<'m> Scene<'m> {
    pub fn parse<'a>(mem: &mut Mem<'m>, s: &'a str) -> Result<Scene<'m>, ParseSceneError<'a>> {
        crt::parse(mem, s)
    }
}

impl Default for Plane {
    fn default() -> Plane {
        Plane { normal: Ray::from_to(v64::ZERO, v64(0.0, 0.0, 1.0)), material: Default::default() }
    }
}

impl<'m> Default for Mesh<'m> {
    fn default() -> Mesh<'m> {
        Mesh { v: &mut [], n: &mut [], f: &mut [], material: Material::default() }
    }
}

impl<'m> Mesh<'m> {
    pub fn iter(&self) -> impl Iterator<Item = Triangle> + ExactSizeIterator + '_ {
        self.f.iter().map(|f| Triangle {
            v: f.v.map(|it| self.v[it as usize]),
            n: f.n.map(|it| self.n[it as usize]),
        })
    }

    pub fn triangle(&self, idx: usize) -> Triangle {
        let f = &self.f[idx];
        Triangle { v: f.v.map(|it| self.v[it as usize]), n: f.n.map(|it| self.n[it as usize]) }
    }
}
