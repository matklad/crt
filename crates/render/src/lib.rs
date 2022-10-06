#![no_std]
pub mod rgb;
mod render;

use bvh::{BoundingBox, Bvh};
use geom::{cross, v64, Ray};
use mem::{Mem, Oom};
use scene::{Color, Triangle};

/// {0}
#[derive(Debug, displaydoc::Display)]
pub struct Error<'a>(ErrorRepr<'a>);

#[derive(Debug, displaydoc::Display)]
enum ErrorRepr<'a> {
    /// {0}
    ParseSceneError(scene::ParseSceneError<'a>),
    /// oom while constructing bhv
    BhvConstructionError(Oom),
}

impl<'a> From<ErrorRepr<'a>> for Error<'a> {
    fn from(repr: ErrorRepr) -> Error {
        Error(repr)
    }
}

type ThreadPool<'t> = dyn Fn(&(dyn Fn() + Sync)) + 't;

pub fn render<'a>(
    crt: &'a str,
    mem: &mut [u8],
    in_parallel: &ThreadPool<'_>,
    buf: &mut rgb::Buf<'_>,
) -> Result<(), Error<'a>> {
    let mut mem = Mem::new(mem);
    let scene = scene::Scene::parse(&mut mem, crt).map_err(ErrorRepr::ParseSceneError)?;
    let bhvs =
        mem.alloc_array_default(scene.meshes.len()).map_err(ErrorRepr::BhvConstructionError)?;
    for (i, m) in scene.meshes.iter().enumerate() {
        let mut bbs = m.iter().map(triangle_bounding_box);
        bhvs[i] = Bvh::build(&mut mem, &mut bbs).map_err(ErrorRepr::BhvConstructionError)?;
    }

    let camera = Camera::new(&scene.camera);

    let dim = buf.dim();
    let rows = buf.partition();
    in_parallel(&|| {
        while let Some(row) = rows.next_row() {
            let y = row.y;
            for x in 0..dim[0] {
                let [dx, dy] = to_scree_space(dim, [x, y]);
                let ray = camera.cast(dx, dy);
                let color = render::render(&scene, &bhvs, &ray);
                row.buf[x as usize] = to_rgb(&color);
            }
        }
    });

    Ok(())
}

fn triangle_bounding_box(t: Triangle) -> BoundingBox {
    BoundingBox::from_points(&t.v)
}

fn to_scree_space(res: [u32; 2], idx: rgb::Idx) -> [f64; 2] {
    let f = |d: u32, x: u32| {
        let d = d as f64;
        let x = x as f64;
        (2.0 * x - d) / d
    };
    [f(res[0], idx[0]), -f(res[1], idx[1])]
}

fn to_rgb(color: &Color) -> rgb::Color {
    fn f(value: f64) -> u8 {
        (value * 255.0).clamp(0.0, 255.0).round() as u8
    }
    rgb::Color { r: f(color.r), g: f(color.g), b: f(color.b) }
}

pub(crate) struct Camera {
    pos: v64,
    center: v64,
    dx: v64,
    dy: v64,
}

impl Camera {
    pub(crate) fn new(cfg: &scene::Camera) -> Camera {
        let gaze = Ray::from_to(cfg.pos, cfg.look_at);
        let center = gaze.at(cfg.focus);
        let right = cross(gaze.dir(), cfg.up).to_unit();
        let up = cross(right, gaze.dir()).to_unit();
        let dx = right * cfg.width / 2.0;
        let dy = -up * cfg.height / 2.0;
        Camera { pos: cfg.pos, center, dx, dy }
    }

    pub(crate) fn cast(&self, dx: f64, dy: f64) -> Ray {
        let to = self.center + self.dx * dx + self.dy * dy;
        Ray::from_to(self.pos, to)
    }
}
