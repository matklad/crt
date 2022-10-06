mod color;

use core::{
    marker::PhantomData,
    ops, slice,
    sync::atomic::{AtomicU32, Ordering::SeqCst},
};

pub use self::color::{Color, ParseColorError};

pub type Idx = [u32; 2];

pub struct Buf<'m> {
    dim: [u32; 2],
    buf: &'m mut [Color],
}

impl<'m> Buf<'m> {
    pub fn new(dim @ [dx, dy]: Idx, buf: &'m mut [Color]) -> Buf<'m> {
        assert!(dx * dy == buf.len() as u32);
        Buf { dim, buf }
    }
    pub fn by_row(&self) -> impl Iterator<Item = Idx> {
        let [dx, dy] = self.dim;
        (0..dy).flat_map(move |y| (0..dx).map(move |x| [x, y]))
    }
    pub fn buf(&self) -> &[Color] {
        &*self.buf
    }
    pub fn buf_mut(&mut self) -> &mut [Color] {
        &mut *self.buf
    }
    pub fn dim(&self) -> Idx {
        self.dim
    }
    pub fn width(&self) -> u32 {
        self.dim[0]
    }
    pub fn height(&self) -> u32 {
        self.dim[1]
    }
    pub(crate) fn partition(&mut self) -> BufPartition<'_, 'm> {
        BufPartition {
            p: PhantomData,
            buf: self.buf.as_mut_ptr(),
            dim: self.dim,
            next_row: AtomicU32::new(0),
        }
    }
    fn linear(&self, idx: Idx) -> Option<usize> {
        if !(idx[0] < self.dim[0] && idx[1] < self.dim[1]) {
            return None;
        }
        Some((idx[0] + idx[1] * self.dim[0]) as usize)
    }
}

impl<'m> ops::Index<Idx> for Buf<'m> {
    type Output = Color;

    fn index(&self, index: Idx) -> &Self::Output {
        let l = self.linear(index).unwrap();
        &self.buf()[l]
    }
}

impl<'m> ops::IndexMut<Idx> for Buf<'m> {
    fn index_mut(&mut self, index: Idx) -> &mut Self::Output {
        let l = self.linear(index).unwrap();
        &mut self.buf_mut()[l]
    }
}

pub(crate) struct BufPartition<'a, 'm> {
    p: PhantomData<&'a mut Buf<'m>>,
    dim: Idx,
    buf: *mut Color,
    next_row: AtomicU32,
}

unsafe impl Send for BufPartition<'_, '_> {}
unsafe impl Sync for BufPartition<'_, '_> {}

pub(crate) struct Row<'a> {
    pub(crate) y: u32,
    pub(crate) buf: &'a mut [Color],
}

impl<'a, 'm> BufPartition<'a, 'm> {
    pub fn next_row(&self) -> Option<Row<'a>> {
        let y = self.next_row.fetch_add(1, SeqCst);
        if y >= self.dim[1] {
            self.next_row.fetch_sub(1, SeqCst);
            return None;
        }
        let start = (y * self.dim[0]) as usize;
        let end = ((y + 1) * self.dim[0]) as usize;
        let buf = unsafe {
            let data = self.buf.add(start);
            let len = end - start;
            slice::from_raw_parts_mut(data, len)
        };
        Some(Row { y, buf })
    }
}
