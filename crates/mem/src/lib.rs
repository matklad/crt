#![no_std]
use core::{mem, ptr, slice};

pub struct Mem<'m> {
    raw: &'m mut [u8],
}

#[derive(Debug)]
pub struct Oom;

impl<'m> Mem<'m> {
    pub fn with<T>(raw: &mut [u8], f: impl FnOnce(&mut Mem<'_>) -> T) -> T {
        f(&mut Mem { raw })
    }

    pub fn with_scratch<T>(
        &mut self,
        size: usize,
        f: impl FnOnce(&mut Mem<'m>, &mut Mem<'_>) -> T,
    ) -> T {
        let raw = mem::take(&mut self.raw);
        let orig_ptr = raw as *mut [u8] as *mut u8;
        let orig_len = raw.len();
        let mid = orig_len - size;

        let (mem, scratch) = raw.split_at_mut(mid);
        self.raw = mem;
        let res = f(self, &mut Mem { raw: scratch });
        let len = self.raw.len() + size;
        self.raw = unsafe { slice::from_raw_parts_mut(orig_ptr.add(orig_len - len), len) };
        res
    }

    pub fn free(&self) -> usize {
        self.raw.len()
    }

    pub fn alloc<T>(&mut self, t: T) -> Result<&'m mut T, Oom> {
        let size = mem::size_of::<T>();
        let align = mem::align_of::<T>();
        assert!(size % align == 0);
        self.align_to(align)?;
        let res = self.alloc_bytes(size)?;
        let ptr = res as *mut u8 as *mut T;
        unsafe {
            ptr::write(ptr, t);
            Ok(&mut *ptr)
        }
    }

    pub fn alloc_array<T>(
        &mut self,
        n: usize,
        mut element: impl FnMut(usize) -> T,
    ) -> Result<&'m mut [T], Oom> {
        let size = mem::size_of::<T>();
        let align = mem::align_of::<T>();
        assert!(size % align == 0);
        self.align_to(align)?;
        let alloc_size = size.checked_mul(n).ok_or(Oom)?;
        let res = self.alloc_bytes(alloc_size)?;
        let mut ptr = res as *mut u8 as *mut T;
        let res = ptr::slice_from_raw_parts_mut(ptr, n);
        for i in 0..n {
            assert!(cfg!(panic = "abort"));
            unsafe {
                ptr::write(ptr, element(i));
                ptr = ptr.add(1);
            }
        }

        Ok(unsafe { &mut *res })
    }

    pub fn alloc_array_default<T: Default>(&mut self, n: usize) -> Result<&'m mut [T], Oom> {
        self.alloc_array(n, |_| T::default())
    }

    fn align_to(&mut self, align: usize) -> Result<(), Oom> {
        debug_assert!(align.is_power_of_two());
        let addr = self.raw.as_ptr() as usize;
        let aligned = addr.wrapping_add(align - 1) & !(align - 1);
        let waste = aligned.checked_sub(addr).ok_or(Oom)?;
        let _ = self.alloc_bytes(waste)?;
        Ok(())
    }

    fn alloc_bytes(&mut self, n: usize) -> Result<*mut [u8], Oom> {
        if self.raw.len() < n {
            return Err(Oom);
        }
        let raw = mem::take(&mut self.raw);
        let (res, raw) = raw.split_at_mut(n);
        self.raw = raw;
        Ok(res)
    }
}

#[test]
fn test_scratch() {
    let mut buf = [0u8; 4];
    Mem::with(&mut buf, |mem| {
        let x = mem.alloc(0u8).unwrap();
        let y = mem.with_scratch(2, |mem, scratch| {
            let y = mem.alloc(1u8).unwrap();
            let z = scratch.alloc(2u8).unwrap();
            assert_eq!((*x, *y, *z), (0, 1, 2));
            assert!(mem.alloc(0u8).is_err());
            y // Returning z here fails.
        });
        let z = mem.alloc(3u8).unwrap();
        assert_eq!((*x, *y, *z), (0, 1, 3));
    });
    assert_eq!(buf, [0, 1, 3, 0]);
    // Will fail to compile.
}
