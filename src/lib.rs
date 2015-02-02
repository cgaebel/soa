#![allow(unused_features)]

#![feature(alloc)]
#![feature(collections)]
#![feature(core)]
#![feature(hash)]
#![feature(test)]

#![feature(unsafe_destructor)]

extern crate alloc;
extern crate collections;
extern crate core;

#[cfg(test)] extern crate test;

use collections::vec;

use core::cmp::Ordering;
use core::default::Default;
use core::fmt::{Debug, Formatter, Result};
use core::hash::{self, Hash};
use core::iter::{self, repeat};
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
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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

    // TODO: drain

    #[inline]
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    // TODO: map_in_place

    pub fn extend<I0, I1>(&mut self, i0: I0, i1: I1)
        where I0: Iterator<Item=A>, I1: Iterator<Item=B> {
        unsafe {
            let (lower, _) = i0.size_hint();
            let space = unadorned::calc_reserve_space(&self.e, lower);

            let d0u = self.d0.extend(&self.e, &space, i0);
            let d1u = self.d1.extend(&self.e, &space, i1);

            unadorned::extend_update(&[d0u, d1u], &mut self.e);
        }
    }

    pub fn from_iters<I0, I1>(i0: I0, i1: I1) -> Soa2<A, B>
        where I0: Iterator<Item=A>, I1: Iterator<Item=B> {
        let mut v = Soa2::new();
        v.extend(i0, i1);
        v
    }

    // TODO: dedup
}

impl<A: Clone, B: Clone> Soa2<A, B> {
    #[inline]
    pub fn resize(&mut self, new_len: usize, value: (A, B)) {
        let len = self.len();

        if new_len > len {
            self.extend(repeat(value.0).take(new_len - len), repeat(value.1).take(new_len - len));
        } else {
            self.truncate(new_len);
        }
    }

    #[inline]
    pub fn push_all(&mut self, x0: &[A], x1: &[B]) {
        unsafe {
            assert_eq!(x0.len(), x1.len());

            let space = unadorned::calc_reserve_space(&self.e, x0.len());

            let d0u = self.d0.push_all(x0, &self.e, &space);
            let d1u = self.d1.push_all(x1, &self.e, &space);

            unadorned::push_all_update(&[d0u, d1u], &mut self.e, x0.len(), space);
        }
    }
}

impl<A: Clone, B: Clone> Clone for Soa2<A, B> {
    #[inline]
    fn clone(&self) -> Soa2<A, B> {
        let mut ret = Soa2::new();
        let (d0, d1) = self.as_slices();
        ret.push_all(d0, d1);
        ret
    }

    fn clone_from(&mut self, other: &Soa2<A, B>) {
        // TODO: cleanup

        if self.len() > other.len() {
            self.truncate(other.len());
        }

        let (od0, od1) = other.as_slices();

        let (s0, s1) = {
            let (sd0, sd1) = self.as_mut_slices();

            for (place, thing) in sd0.iter_mut().zip(od0.iter()) {
                place.clone_from(thing);
            }

            for (place, thing) in sd1.iter_mut().zip(od1.iter()) {
                place.clone_from(thing);
            }

            let s0 = &od0[sd0.len()..];
            let s1 = &od1[sd1.len()..];

            (s0, s1)
        };

        self.push_all(s0, s1);
    }
}

impl<S: hash::Writer + hash::Hasher, A: Hash<S>, B: Hash<S>> Hash<S> for Soa2<A, B> {
    #[inline]
    fn hash(&self, state: &mut S) {
        self.as_slices().hash(state)
    }
}

impl<A0, B0, A1, B1> PartialEq<Soa2<A1, B1>> for Soa2<A0, B0>
  where A0: PartialEq<A1>, B0: PartialEq<B1> {
    #[inline]
    fn eq(&self, other: &Soa2<A1, B1>) -> bool {
        let (a0, b0) = self.as_slices();
        let (a1, b1) = other.as_slices();

        PartialEq::eq(a0, a1) && PartialEq::eq(b0, b1)
    }

    #[inline]
    fn ne(&self, other: &Soa2<A1, B1>) -> bool {
        let (a0, b0) = self.as_slices();
        let (a1, b1) = other.as_slices();

        PartialEq::ne(a0, a1) || PartialEq::ne(b0, b1)
    }
}

impl<A0, B0, A1, B1> PartialEq<Vec<(A1, B1)>> for Soa2<A0, B0>
  where A0: PartialEq<A1>, B0: PartialEq<B1> {
    #[inline]
    fn eq(&self, other: &Vec<(A1, B1)>) -> bool {
        self.len() == other.len()
        && self.iter().zip(other.iter()).all(
            |((a0, b0), &(ref a1, ref b1))| a0 == a1 && b0 == b1)
    }

    #[inline]
    fn ne(&self, other: &Vec<(A1, B1)>) -> bool {
        self.len() != other.len()
        || self.iter().zip(other.iter()).any(
            |((a0, b0), &(ref a1, ref b1))| a0 != a1 || b0 != b1)
    }
}

impl<'b, A0, B0, A1, B1> PartialEq<&'b [(A1, B1)]> for Soa2<A0, B0>
  where A0: PartialEq<A1>, B0: PartialEq<B1> {
    #[inline]
    fn eq(&self, other: &&'b [(A1, B1)]) -> bool {
        self.len() == other.len()
        && self.iter().zip(other.iter()).all(
            |((a0, b0), &(ref a1, ref b1))| a0 == a1 && b0 == b1)
    }

    #[inline]
    fn ne(&self, other: &&'b [(A1, B1)]) -> bool {
        self.len() != other.len()
        || self.iter().zip(other.iter()).any(
            |((a0, b0), &(ref a1, ref b1))| a0 != a1 || b0 != b1)
    }
}

impl<'b, A0, B0, A1, B1> PartialEq<&'b mut [(A1, B1)]> for Soa2<A0, B0>
  where A0: PartialEq<A1>, B0: PartialEq<B1> {
    #[inline]
    fn eq(&self, other: &&'b mut [(A1, B1)]) -> bool {
        self.len() == other.len()
        && self.iter().zip(other.iter()).all(
            |((a0, b0), &(ref a1, ref b1))| a0 == a1 && b0 == b1)
    }

    #[inline]
    fn ne(&self, other: &&'b mut [(A1, B1)]) -> bool {
        self.len() != other.len()
        || self.iter().zip(other.iter()).any(
            |((a0, b0), &(ref a1, ref b1))| a0 != a1 || b0 != b1)
    }
}

impl<A: PartialOrd, B: PartialOrd> PartialOrd for Soa2<A, B> {
    #[inline]
    fn partial_cmp(&self, other: &Soa2<A, B>) -> Option<Ordering> {
        iter::order::partial_cmp(self.iter(), other.iter())
    }
}

impl<A: Eq, B: Eq> Eq for Soa2<A, B> {}

impl<A: Ord, B: Ord> Ord for Soa2<A, B> {
    #[inline]
    fn cmp(&self, other: &Soa2<A, B>) -> Ordering {
        iter::order::cmp(self.iter(), other.iter())
    }
}

impl<A, B> Default for Soa2<A, B> {
    fn default() -> Soa2<A, B> { Soa2::new() }
}

impl<A: Debug, B: Debug> Debug for Soa2<A, B> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        Debug::fmt(&self.as_slices(), f)
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

#[cfg(test)]
mod tests {
    use super::Soa2;

    struct DropCounter<'a> {
        count: &'a mut i32,
    }

    #[unsafe_destructor]
    impl<'a> Drop for DropCounter<'a> {
        fn drop(&mut self) {
            *self.count += 1;
        }
    }

    #[test]
    fn test_double_drop() {
        struct TwoVec<T> {
            x: Soa2<T, T>,
            y: Soa2<T, T>,
        }

        let (mut c0, mut c1, mut c2, mut c3) = (0, 0, 0, 0);

        {
            let mut tv =
                TwoVec {
                    x: Soa2::new(),
                    y: Soa2::new(),
                };

            tv.x.push(
                (DropCounter { count: &mut c0 },
                 DropCounter { count: &mut c1 }));
            tv.y.push(
                (DropCounter { count: &mut c2 },
                 DropCounter { count: &mut c3 }));

            drop(tv.x);
        }

        assert_eq!(c0, 1);
        assert_eq!(c1, 1);
        assert_eq!(c2, 1);
        assert_eq!(c3, 1);
    }

    #[test]
    fn test_reserve() {
        let mut v = Soa2::new();
        assert_eq!(v.capacity(), 0);

        v.reserve(2);
        assert!(v.capacity() >= 2);

        for i in range(0is, 16) {
            v.push((i, i));
        }

        assert!(v.capacity() >= 16);
        v.reserve(16);
        assert!(v.capacity() >= 32);

        v.push((16, 16));

        v.reserve(16);
        assert!(v.capacity() >= 33);
    }

    #[test]
    fn test_extend() {
        let mut v = Soa2::new();
        let mut w = Soa2::new();

        v.extend(range(0is, 3), range(4is, 7));
        for i in range(0is, 3).zip(range(4is, 7)) {
            w.push(i);
        }

        assert_eq!(v, w);

        v.extend(range(3is, 10), range(7is, 14));
        for i in range(3is, 10).zip(range(7is, 14)) {
            w.push(i);
        }

        assert_eq!(v, w);
    }
}
