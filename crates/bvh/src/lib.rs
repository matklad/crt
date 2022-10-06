#![no_std]
use geom::{v64, Ray};
use mem::{Mem, Oom};

#[derive(Default)]
pub struct Bvh<'m> {
    splits: &'m mut [BvhSplit],
    leaves: &'m mut [BvhLeaf],
}

#[derive(Default)]
struct BvhSplit {
    children: [u32; 2],
    bb: BoundingBox,
    axis: u8,
}

#[derive(Default)]
struct BvhLeaf {
    face: u32,
    bb: BoundingBox,
}

#[derive(Default, Clone, Copy)]
pub struct BoundingBox {
    lo: v64,
    hi: v64,
}

impl<'m> Bvh<'m> {
    pub fn build(
        mem: &mut Mem<'m>,
        input: &mut (dyn ExactSizeIterator<Item = BoundingBox>),
    ) -> Result<Bvh<'m>, Oom> {
        let free_mem = mem.free();
        mem.with_scratch(free_mem / 2, |mem, scratch| {
            let bbs: &mut [BoundingBox] =
                scratch.alloc_array(input.len(), |_| input.next().unwrap())?;
            let faces = scratch.alloc_array(bbs.len(), |i| i as u32)?;
            let mut res = Bvh::default();
            if !faces.is_empty() {
                let (root, n_splits, n_leaves) = bvh_recur(scratch, faces, bbs)?;
                res.splits = mem.alloc_array_default(n_splits as usize)?;
                res.leaves = mem.alloc_array_default(n_leaves as usize)?;
                fill(&mut res, &mut (0, 0), root);
            }
            Ok(res)
        })
    }

    pub fn intersect(&self, ray: &Ray, max_t: &mut f64, intersect: &mut dyn FnMut(u32, &mut f64)) {
        let mut work = [0u32; 64];
        let mut w = 0;
        if self.leaves.is_empty() {
            return;
        } else if self.splits.is_empty() {
            work[w] = LEAF_BIT;
            w += 1;
        } else {
            work[w] = 0;
            w += 1;
        }
        while w > 0 {
            w -= 1;
            let idx = work[w];
            let is_leaf = idx & LEAF_BIT == LEAF_BIT;
            let idx = (idx & !LEAF_BIT) as usize;

            let bb = if is_leaf { &self.leaves[idx].bb } else { &self.splits[idx].bb };
            if !bb.is_intersected(ray, *max_t) {
                continue;
            }
            if is_leaf {
                let leaf = &self.leaves[idx];
                intersect(leaf.face, max_t);
            } else {
                let split = &self.splits[idx];
                let (c1, c2) = if ray.dir().xyz()[split.axis as usize] < 0.0 {
                    (split.children[0], split.children[1])
                } else {
                    (split.children[1], split.children[0])
                };
                // Quite sketchy! Ideally, we'd allocate per-thread max-size
                // buffer and pass that around, but requires quite some coding.
                if w + 2 <= work.len() {
                    work[w] = c1;
                    work[w + 1] = c2;
                    w += 2;
                }
            }
        }
    }
}

impl BoundingBox {
    pub fn from_points(vs: &[v64]) -> BoundingBox {
        vs.iter().copied().map(BoundingBox::from_point).reduce(BoundingBox::union).unwrap()
    }

    pub fn from_point(vs: v64) -> BoundingBox {
        BoundingBox { lo: vs, hi: vs }
    }

    fn diag(&self) -> v64 {
        self.hi - self.lo
    }
    fn center(&self) -> v64 {
        self.lo + self.diag() / 2.0
    }
    fn longest_axis(&self) -> u8 {
        let d = self.diag();
        if d.x > d.y && d.x > d.z {
            0
        } else if d.y > d.z {
            1
        } else {
            2
        }
    }

    fn union(self, other: BoundingBox) -> BoundingBox {
        fn zip(f: impl Fn(f64, f64) -> f64, l: v64, r: v64) -> v64 {
            v64(f(l.x, r.x), f(l.y, r.y), f(l.z, r.z))
        }
        BoundingBox { lo: zip(f64::min, self.lo, other.lo), hi: zip(f64::max, self.hi, other.hi) }
    }

    fn is_intersected(&self, ray: &Ray, mut max_t: f64) -> bool {
        let mut min_t: f64 = 0.0;
        for axis in 0..3 {
            let inv_dir = 1.0 / ray.dir().xyz()[axis];
            let t1 = (self.lo.xyz()[axis] - ray.origin().xyz()[axis]) * inv_dir;
            let t2 = (self.hi.xyz()[axis] - ray.origin().xyz()[axis]) * inv_dir;
            let t_near = t1.min(t2);
            let t_far = t1.max(t2);
            min_t = min_t.max(t_near);
            max_t = max_t.min(t_far);
            if max_t < min_t {
                return false;
            }
        }
        true
    }
}

fn bvh_recur<'s>(
    scratch: &mut Mem<'s>,
    faces: &mut [u32],
    bbs: &[BoundingBox],
) -> Result<(&'s mut Node<'s>, u32, u32), Oom> {
    if faces.len() == 1 {
        let face = faces[0];
        let bb = bbs[face as usize];
        let res = scratch.alloc(Node::Leaf { face, bb })?;
        return Ok((res, 0, 1));
    }
    let bb = faces
        .iter()
        .map(|&i| bbs[i as usize].center())
        .map(BoundingBox::from_point)
        .reduce(BoundingBox::union)
        .unwrap();
    let axis = bb.longest_axis();
    let key = |i: u32| bbs[i as usize].center().xyz()[axis as usize];
    faces.sort_by(|&i, &j| key(i).total_cmp(&key(j)));
    let mid = faces.len() / 2;
    let (left, right) = faces.split_at_mut(mid);

    let (left, ls, ll) = bvh_recur(scratch, left, bbs)?;
    let (right, rs, rl) = bvh_recur(scratch, right, bbs)?;
    let bb = left.bounding_box().union(*right.bounding_box());
    let res = scratch.alloc(Node::Split { children: [left, right], bb, axis })?;
    Ok((res, ls + rs + 1, ll + rl))
}

enum Node<'s> {
    Split { children: [&'s mut Node<'s>; 2], bb: BoundingBox, axis: u8 },
    Leaf { face: u32, bb: BoundingBox },
}

impl<'s> Node<'s> {
    fn bounding_box(&self) -> &BoundingBox {
        match self {
            Node::Split { bb, .. } | Node::Leaf { bb, .. } => bb,
        }
    }
}

const LEAF_BIT: u32 = 1u32.rotate_right(1);

fn fill(res: &mut Bvh<'_>, idx: &mut (usize, usize), node: &Node<'_>) -> u32 {
    match node {
        Node::Split { children, bb, axis } => {
            let i = idx.0;
            idx.0 += 1;
            let l = fill(res, idx, &children[0]);
            let r = fill(res, idx, &children[1]);
            res.splits[i] = BvhSplit { children: [l, r], bb: *bb, axis: *axis };
            i as u32
        }
        Node::Leaf { face, bb } => {
            let i = idx.1;
            idx.1 += 1;
            res.leaves[i] = BvhLeaf { face: *face, bb: *bb };
            (i as u32) | LEAF_BIT
        }
    }
}
