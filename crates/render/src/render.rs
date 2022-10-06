use core::iter;

use bvh::Bvh;
use geom::{cross, dot, v64, Ray};
use scene::{Color, Material, Mesh, Plane, Scene, Sphere, Triangle};

pub(crate) fn render(scene: &Scene, bvhs: &[Bvh<'_>], ray: &Ray) -> Color {
    let mut res = scene.background;
    if let Some(i) = intersect(&scene, bvhs, &ray) {
        let ambient_color = i.material.color;
        res = ambient_color;

        let mut p = ray.at(i.t);
        p = p + i.n * 0.0001;

        let lr = Ray::from_to(p, scene.light.pos);

        let obscured = match intersect(&scene, bvhs, &lr) {
            None => false,
            Some(i) => i.t.powi(2) < (scene.light.pos - p).norm_squared(),
        };

        if !obscured {
            let k = (dot(lr.dir(), i.n)).max(0.0) * i.material.diffuse;
            let diffuse_color = i.material.color * scene.light.color * k;
            res = res + diffuse_color;
        }
    }
    res
}

struct Intersection<'a> {
    t: f64,
    n: v64,
    material: &'a Material,
}

fn intersect<'a>(scene: &'a Scene, bvhs: &'a [Bvh<'_>], ray: &Ray) -> Option<Intersection<'a>> {
    let mut res: Option<Intersection> = None;
    intersect_with(ray, &mut res, &*scene.spheres);
    intersect_with(ray, &mut res, &*scene.planes);
    intersect_with(ray, &mut res, iter::zip(&*scene.meshes, bvhs));
    if let Some(i) = &mut res {
        i.n = i.n.to_unit()
    }
    res
}

fn intersect_with<'a, T, I>(ray: &Ray, res: &mut Option<Intersection<'a>>, objects: I)
where
    T: RenderObject<'a>,
    I: IntoIterator<Item = T>,
{
    let max_t = res.as_ref().map(|it| it.t).unwrap_or(f64::INFINITY);
    for o in objects {
        if let Some((t, n)) = o.intersect(&ray, max_t) {
            *res = Some(Intersection { t, n, material: o.material() })
        }
    }
}

trait RenderObject<'a> {
    fn intersect(&self, ray: &Ray, max_t: f64) -> Option<(f64, v64)>;
    fn material(&self) -> &'a Material;
}

impl<'a> RenderObject<'a> for &'a Sphere {
    fn intersect(&self, ray: &Ray, max_t: f64) -> Option<(f64, v64)> {
        let o = ray.origin() - self.center;
        let k = dot(ray.dir(), o);
        let c = dot(o, o) - self.radius.powi(2);

        let disc = k * k - c;
        if disc < 0.0 {
            return None;
        }

        let t = {
            let t1 = -k - disc.sqrt();
            let t2 = -k + disc.sqrt();
            if t1 > 0.0 {
                t1
            } else if t2 > 0.0 {
                t2
            } else {
                return None;
            }
        };
        if t >= max_t {
            return None;
        }

        let point = ray.at(t);
        let normal = point - self.center;
        Some((t, normal))
    }
    fn material(&self) -> &'a Material {
        &self.material
    }
}

impl<'a> RenderObject<'a> for &'a Plane {
    fn intersect(&self, ray: &Ray, max_t: f64) -> Option<(f64, v64)> {
        let o = ray.origin() - self.normal.origin();
        let t = -dot(o, self.normal.dir()) / dot(ray.dir(), self.normal.dir());
        if !(0.0 < t && t < max_t) {
            return None;
        }
        Some((t, self.normal.dir()))
    }
    fn material(&self) -> &'a Material {
        &self.material
    }
}

impl<'a, 'm> RenderObject<'a> for (&'a Mesh<'m>, &'a Bvh<'m>) {
    fn intersect(&self, ray: &Ray, mut max_t: f64) -> Option<(f64, v64)> {
        let (mesh, bvh) = *self;
        let mut n = None;
        bvh.intersect(ray, &mut max_t, &mut |face, max_t| {
            if let Some((t, nn)) = intersect_triangle(&mesh.triangle(face as usize), ray, *max_t) {
                *max_t = t;
                n = Some(nn);
            }
        });
        let n = n?;
        Some((max_t, n))
    }

    fn material(&self) -> &'a Material {
        &self.0.material
    }
}

fn intersect_triangle(tr: &Triangle, ray: &Ray, max_t: f64) -> Option<(f64, v64)> {
    let ab = tr.v[1] - tr.v[0];
    let ac = tr.v[2] - tr.v[0];
    let n = cross(ab, ac);
    // a + alpha ab + beta ac = ray.origin + t * ray.direction
    let t = dot(tr.v[0] - ray.origin(), n) / dot(ray.dir(), n);
    if !(0.0 <= t && t <= max_t) {
        return None;
    }
    let point = ray.at(t);
    let local_coords: [f64; 3] = {
        let ort_ac = cross(ac, n);
        let ort_ab = cross(ab, n);
        let point = point - tr.v[0];
        let alpha = dot(point, ort_ac) / dot(ab, ort_ac);
        let beta = dot(point, ort_ab) / dot(ac, ort_ab);
        let gamma = 1.0 - (alpha + beta);
        [gamma, alpha, beta]
    };
    if local_coords.map(|it| 0.0 < it && it < 1.0) != [true; 3] {
        return None;
    }
    let mut n = v64::ZERO;
    for i in 0..3 {
        n = n + tr.n[i] * local_coords[i];
    }
    Some((t, n))
}
