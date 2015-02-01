#![feature(alloc)]
#![feature(collections)]
#![feature(core)]

#![feature(unsafe_destructor)]

extern crate alloc;
extern crate collections;
extern crate core;

use collections::vec;
use core::iter;
use core::mem;
use core::num::Int;
use core::ptr;
use core::slice;

use unadorned::{Unadorned, Extent};

mod unadorned;

#[unsafe_no_drop_flag]
pub struct Soa2<A, B> {
    d0: Unadorned<A>,
    d1: Unadorned<B>,
    e:  Extent,
}

impl<A, B> Soa2<A, B> {
    pub fn new() -> Soa2<A, B> {
        unsafe {
            let (d0, d0u) = Unadorned::new();
            let (d1, d1u) = Unadorned::new();

            let e = unadorned::new_update(&[d0u, d1u]);

            Soa2 { d0: d0, d1: d1, e: e }
        }
    }

    #[inline]
    fn is_boring(&self) -> bool {
        self.d0.is_boring() && self.d1.is_boring()
    }

    pub fn with_capacity(capacity: usize) -> Soa2<A, B> {
        unsafe {
            let (d0, d0u) = Unadorned::with_capacity(capacity);
            let (d1, d1u) = Unadorned::with_capacity(capacity);

            let is_boring = mem::size_of::<A>() == 0 && mem::size_of::<B>() == 0;

            let e = unadorned::with_capacity_update(&[d0u, d1u], is_boring, capacity);

            Soa2 { d0: d0, d1: d1, e: e }
        }
    }

    pub unsafe fn from_raw_parts(
        ptra: *mut A, ptrb: *mut B, len: usize, cap: usize) -> Soa2<A, B> {
        let (d0, d0u) = Unadorned::from_raw_parts(ptra);
        let (d1, d1u) = Unadorned::from_raw_parts(ptrb);

        let e = unadorned::from_raw_parts_update(&[d0u, d1u], len, cap);

        Soa2 { d0: d0, d1: d1, e: e }
    }

    #[inline]
    pub unsafe fn from_raw_bufs(ptra: *const A, ptrb: *const B, elts: usize) -> Soa2<A, B> {
        let (d0, d0u) = Unadorned::from_raw_bufs(ptra, elts);
        let (d1, d1u) = Unadorned::from_raw_bufs(ptrb, elts);

        let e = unadorned::from_raw_bufs_update(&[d0u, d1u], elts);

        Soa2 { d0: d0, d1: d1, e: e }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.e.len
    }

    #[inline]
    pub unsafe fn set_len(&mut self, len: usize) {
        self.e.len = len;
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.e.cap
    }

    pub fn reserve(&mut self, additional: usize) {
        let space =
            match unadorned::calc_reserve_space(&self.e, additional) {
                None        => return,
                Some(space) => space,
            };

        unsafe {
            let d0u = self.d0.reserve(&self.e, &space);
            let d1u = self.d1.reserve(&self.e, &space);

            unadorned::reserve_update(&[d0u, d1u], space, &mut self.e);
        }
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        let space =
            match unadorned::calc_reserve_exact_space(&self.e, additional) {
                None        => return,
                Some(space) => space,
            };

        unsafe {
            let d0u = self.d0.reserve(&self.e, &space);
            let d1u = self.d1.reserve(&self.e, &space);

            unadorned::reserve_update(&[d0u, d1u], space, &mut self.e);
        }
    }

    pub fn shrink_to_fit(&mut self) {
        if self.is_boring() { return }

        unsafe {
            let d0u = self.d0.shrink_to_fit(&self.e);
            let d1u = self.d1.shrink_to_fit(&self.e);

            unadorned::shrink_to_fit_update(&[d0u, d1u], &mut self.e);
        }
    }

    pub fn truncate(&mut self, len: usize) {
        if self.is_boring() { return }

        unsafe {
            let d0u = self.d0.truncate(len, &self.e);
            let d1u = self.d1.truncate(len, &self.e);

            unadorned::truncate_update(&[d0u, d1u], len, &mut self.e);
        }
    }

    #[inline]
    pub fn as_mut_slices<'a>(&'a mut self) -> (&'a mut [A], &'a mut [B]) {
        unsafe {
            let len = self.e.len;
            (self.d0.as_mut_slice(len), self.d1.as_mut_slice(len))
        }
    }

    #[inline]
    pub fn as_slices<'a>(&'a self) -> (&'a [A], &'a [B]) {
        unsafe {
            let len = self.e.len;
            (self.d0.as_slice(len), self.d1.as_slice(len))
        }
    }

    #[inline]
    pub fn iter(&self) -> iter::Zip<slice::Iter<A>, slice::Iter<B>> {
        let (d0, d1) = self.as_slices();
        d0.iter().zip(d1.iter())
    }

    #[inline]
    pub fn iter_mut(&mut self) -> iter::Zip<slice::IterMut<A>, slice::IterMut<B>> {
        let (d0, d1) = self.as_mut_slices();
        d0.iter_mut().zip(d1.iter_mut())
    }

    #[inline]
    pub fn into_iter(mut self) -> iter::Zip<vec::IntoIter<A>, vec::IntoIter<B>> {
        unsafe {
            let e_copy = self.e;
            self.e.cap = 0; // Will skip the drop. into_iter will handle it.
            self.d0.shallow_copy().into_iter(&e_copy).zip(self.d1.shallow_copy().into_iter(&e_copy))
        }
    }

    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> (A, B) {
        let length = self.e.len;
        {
            let (d0, d1) = self.as_mut_slices();
            d0.swap(index, length - 1);
            d1.swap(index, length - 1);
        }
        self.pop().unwrap()
    }

    pub fn insert(&mut self, index: usize, element: (A, B)) {
        unsafe {
            assert!(index < self.e.len);

            let space = unadorned::calc_reserve_space(&self.e, 1);

            let d0u = self.d0.insert(index, element.0, &self.e, &space);
            let d1u = self.d1.insert(index, element.1, &self.e, &space);

            unadorned::insert_update(&[d0u, d1u], space, &mut self.e);
        }
    }

    pub fn remove(&mut self, index: usize) -> (A, B) {
        unsafe {
            let (x0, d0u) = self.d0.remove(index, &self.e);
            let (x1, d1u) = self.d1.remove(index, &self.e);

            unadorned::remove_update(&[d0u, d1u], &mut self.e);
            (x0, x1)
        }
    }

    pub fn retain<F>(&mut self, mut f: F) where F: FnMut((&A, &B)) -> bool {
        let len = self.len();
        let mut del = 0us;

        {
            let (d0, d1) = self.as_mut_slices();

            for i in range(0us, len) {
                if !f((&d0[i], &d1[i])) {
                    del += 1;
                } else if del > 0 {
                    d0.swap(i-del, i);
                    d1.swap(i-del, i);
                }
            }
        }

        self.truncate(len - del);
    }

    #[inline]
    pub fn push(&mut self, value: (A, B)) {
        if self.is_boring() {
            // zero-size types consume no memory, so we can't rely on the
            // address space running out
            self.e.len = self.e.len.checked_add(1).expect("length overflow");
            unsafe { mem::forget(value) }
            return
        }

        unsafe {
            let d0u = self.d0.push(value.0, &self.e);
            let d1u = self.d1.push(value.1, &self.e);

            unadorned::push_update(&[d0u, d1u], &mut self.e);
        }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<(A, B)> {
        if self.e.len == 0 {
            None
        } else {
            unsafe {
                self.e.len -= 1;
                let len = self.e.len;

                let (d0, d1) = self.as_mut_slices();

                Some((ptr::read(d0.get_unchecked(len)),
                      ptr::read(d1.get_unchecked(len))))
            }
        }
    }

    #[inline]
    pub fn append(&mut self, other: &mut Self) {
        if self.is_boring() {
            // zero-size types consume no memory, so we can't rely on the address
            // space running out
            self.e.len = self.e.len.checked_add(other.len()).expect("length overflow");
            other.e.len = 0;
            return;
        }

        unsafe {
            let space = unadorned::calc_reserve_space(&self.e, 1);

            let d0u = self.d0.append(&self.e, &other.d0, &other.e, &space);
            let d1u = self.d1.append(&self.e, &other.d1, &other.e, &space);

            unadorned::append_update(&[d0u, d1u], &mut self.e, &mut other.e, space);
        }
    }
}

#[unsafe_destructor]
impl<A, B> Drop for Soa2<A, B> {
    #[inline]
    fn drop(&mut self) {
        if self.e.cap != 0 {
            unsafe {
                self.d0.drop(&self.e);
                self.d1.drop(&self.e);
            }
        }
    }
}
