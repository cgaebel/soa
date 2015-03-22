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

use unadorned::{self, Unadorned, Extent};

macro_rules! first {
    ($x:expr, $($xs:expr),*) => { $x }
}


macro_rules! anyrest {
    ($x:expr, $($xs:expr),*) => { $($xs)||* }
}

macro_rules! tupleup {
    ($a:expr, $b:expr) => { ($a, $b) };
    ($a:expr, $b:expr, $($rest:expr),*) => { tupleup!(($a, $b), $($rest),*) }
}
macro_rules! zip_up {
    ($l:ident, $a:expr, $b:expr) => { Iter::Zip<$a, $b> };
    ($l:ident, $a:expr, $b:expr, $($rest:expr),*) => { zip_up!($l,Iter::Zip<slice::Iter<$l,$a>, slice::Iter<$l, $b>>, $($rest),*) }
}

macro_rules! toits {
    ($($t:ident),*) => { $(Iterator<Item=$t>),* }
}

macro_rules! gen_soa {
    ($soa:ident $soa_zip:ident $soa_mut_zip:ident | $($ty:ident),+ | $($nm:ident),+ | $($nmu:ident),+) => {
        #[unsafe_no_drop_flag]
        pub struct $soa<$($ty),+> {
            $($nm: Unadorned<$ty>),+,
            e: Extent,
        }

        pub struct $soa_zip<'a, $($ty: 'a),+> {
            parent: &'a $soa<$($ty),+>,
            i:   usize,
        }

        impl<'a, $($ty),+> Iterator for $soa_zip<'a, $($ty),+> {
            type Item = ($(&'a $ty),+);

            #[inline]
            fn next(&mut self) -> Option<($(&'a $ty),+)> {
                let i = self.i;
                if i == self.parent.e.len { return None; }

                self.i += 1;

                unsafe {
                    Some(($(&*self.parent.$nm.as_ptr().offset(i as isize)),+))
                }
            }
        }

        pub struct $soa_mut_zip<'a, $($ty: 'a),+> {
            parent: &'a mut $soa<$($ty),+>,
            i:   usize,
        }

        impl<'a, $($ty),+> Iterator for $soa_mut_zip<'a, $($ty),+> {
            type Item = ($(&'a mut $ty),+);

            #[inline]
            fn next(&mut self) -> Option<($(&'a mut $ty),+)> {
                let i = self.i;
                if i == self.parent.e.len { return None; }

                self.i += 1;

                unsafe {
                    Some(($(&mut *self.parent.$nm.as_mut_ptr().offset(i as isize)),+))
                }
            }
        }

        pub struct $soa_into_iter<$($ty),+> {
        }

        impl<$($ty),+> $soa<$($ty),+> {
            fn new() -> $soa<$($ty),+> {
                unsafe {
                    $(let ($nm, $nmu) = Unadorned::new());+;
                    let e = unadorned::new_update(&[$($nmu),+]);
                    $soa { $($nm: $nm),+ , e: e}
                }
            }

            #[inline]
            fn is_boring(&self) -> bool {
                $(self.$nm.is_boring())&&+
            }

            #[inline]
            pub fn with_capacity(capacity: usize) -> $soa<$($ty),+> {
                unsafe {
                    $(let ($nm, $nmu) = Unadorned::with_capacity(capacity));+;
                    let is_boring = $(mem::size_of::<$ty>() == 0)&&+;
                    let e = unadorned::with_capacity_update(&[$($nmu),+], is_boring, capacity);
                    $soa { $($nm: $nm),+, e: e}
                }
            }

            #[inline]
            pub unsafe fn from_raw_parts(
                // this re-use of $nm is hacky and gross.
                $($nm: *mut $ty),+, len: usize, cap: usize) -> $soa<$($ty),+> {
                $(let ($nm, $nmu) = Unadorned::from_raw_parts($nm));+;
                let e = unadorned::from_raw_parts_update(&[$($nmu),+], len, cap);
                $soa { $($nm: $nm),+, e: e}
            }

            #[inline]
            pub unsafe fn from_raw_bufs($($nm: *const $ty),+, elts: usize) -> $soa<$($ty),+> {
                $(let ($nm, $nmu) = Unadorned::from_raw_bufs($nm, elts));+;
                let e = unadorned::from_raw_bufs_update(&[$($nmu),+], elts);
                $soa { $($nm: $nm),+, e: e}
            }

            pub fn from_vecs($(mut $nm: Vec<$ty>),+) -> $soa<$($ty),+> {
                let firstlen = first!($($nm.len()),+);
                if anyrest!($(firstlen != $nm.len()),+ ) {
                    panic!("unequal lengths");
                }
                let firstcap = first!($($nm.capacity()),+);
                if anyrest!($(firstcap != $nm.capacity()),+) {
                    $($nm.shrink_to_fit());+;
                }
                let cap = first!($($nm.capacity()),+);
                unsafe {
                    let ret = $soa::from_raw_parts(
                        $($nm.as_ptr() as *mut $ty),+,
                        firstlen, cap);
                    $(mem::forget($nm));+;
                    ret
                }
            }

            #[inline]
            pub fn len(&self) -> usize { self.e.len }

            #[inline]
            pub fn is_empty(&self) -> bool { self.len() == 0 }

            #[inline]
            pub fn capacity(&self) -> usize { self.e.cap }

            pub fn reserve(&mut self, additional: usize) {
                let space = match unadorned::calc_reserve_space(&self.e, additional) {
                    None => return,
                    Some(space) => space
                };

                unsafe {
                    $(let $nmu = self.$nm.reserve(&self.e, &space));+;
                    unadorned::reserve_update(&[$($nmu),+], space, &mut self.e)
                }
            }
            pub fn reserve_exact(&mut self, additional: usize) {
                let space = match unadorned::calc_reserve_exact_space(&self.e, additional) {
                    None => return,
                    Some(space) => space
                };

                unsafe {
                    $(let $nmu = self.$nm.reserve(&self.e, &space));+;
                    unadorned::reserve_update(&[$($nmu),+], space, &mut self.e)
                }
            }

            pub fn shrink_to_fit(&mut self) {
                if self.is_boring() { return }
                unsafe {
                    $(let $nmu = self.$nm.shrink_to_fit(&self.e));+;
                    unadorned::shrink_to_fit_update(&[$($nmu),+], &mut self.e);
                }
            }

            pub fn truncate(&mut self, len: usize) {
                if self.is_boring() { return }
                unsafe {
                    $(let $nmu = self.$nm.truncate(len, &self.e));+;
                    unadorned::truncate_update(&[$($nmu),+], len, &mut self.e);
                }
            }

            #[inline]
            pub fn as_mut_slices<'a> (&'a mut self) -> ($(&'a mut [$ty]),+) {
                unsafe {
                    let len = self.e.len;
                    ($(self.$nm.as_mut_slice(len)),+)
                }
            }

            #[inline]
            pub fn as_slices<'a>(&'a self) -> ($(&'a [$ty]),+) {
                unsafe { let len = self.e.len;
                         ($(self.$nm.as_slice(len)),+) }
            }

            #[inline]
            pub fn iters(&self) -> ($(slice::Iter<$ty>),+) {
                let ($($nm),+) = self.as_slices();
                ($($nm.iter()),+)
            }

            #[inline]
            pub fn iters_mut(&mut self) -> ($(slice::IterMut<$ty>),+) {
                let ($($nm),+) = self.as_mut_slices();
                ($($nm.iter_mut()),+)
            }

            #[inline]
            pub fn into_iters(mut self) -> ($(vec::IntoIter<$ty>),+) {
                unsafe {
                    let e_copy = self.e;
                    self.e.cap = 0;
                    ($(self.$nm.shallow_copy().into_iter(&e_copy)),+)
                }
            }

            #[inline]
            pub fn into_vecs(mut self) -> ($(Vec<$ty>),+) {
                unsafe {
                    let e_copy = self.e;
                    self.e.cap = 0;
                    ($(self.$nm.shallow_copy().as_vec(&e_copy)),+)
                }
            }

            #[inline]
            pub fn as_ptrs(&self) -> ($(*const $ty),+) {
                let ($($nm),+) = self.as_slices();
                ($($nm.as_ptr()),+)
            }

            #[inline]
            pub fn as_mut_ptrs(&mut self) -> ($(*mut $ty),+) {
                let ($($nm),+) = self.as_mut_slices();
                ($($nm.as_mut_ptr()),+)
            }

            #[inline]
            pub fn swap_remove(&mut self, index: usize) -> ($($ty),+) {
                let length = self.e.len;
                {
                    let ($($nm),+) = self.as_mut_slices();
                    $($nm.swap(index, length - 1));+;
                }
                self.pop().unwrap()
            }

            pub fn insert(&mut self, index: usize, element: ($($ty),+)) {
                unsafe {
                    assert!(index < self.e.len);
                    let space = unadorned::calc_reserve_space(&self.e, 1);
                    let ($($nm),+) = element;
                    $(let $nmu = self.$nm.insert(index, $nm, &self.e, &space));+;
                    unadorned::insert_update(&[$($nmu),+], space, &mut self.e);
                }
            }

            pub fn remove(&mut self, index: usize) -> ($($ty),+) {
                unsafe {
                    assert!(index < self.e.len);
                    $(let ($nm, $nmu) = self.$nm.remove(index, &self.e));+;
                    unadorned::remove_update(&[$($nmu),+], &mut self.e);
                    ($($nm),+)
                }
            }

            pub fn retain<Fun>(&mut self, mut f: Fun) where Fun: FnMut(($(&$ty),+)) -> bool {
                let len = self.len();
                let mut del = 0us;

                {
                    let ($($nm),+) = self.as_mut_slices();
                    for i in range(0us, len) {
                        if !f(($(&$nm[i]),+)) {
                            del += 1;
                        } else if del > 0 {
                            $($nm.swap(i-del, i));+;
                        }
                    }
                }

                self.truncate(len - del);
            }

            #[inline]
            pub fn push(&mut self, value: ($($ty),+)) {
                if self.is_boring() {
                    self.e.len = self.e.len.checked_add(1).expect("length overflow");
                    unsafe { mem::forget(value) }
                    return
                }
                unsafe {
                    let ($($nm),+) = value;
                    $(let $nmu = self.$nm.push($nm, &self.e));+;
                    unadorned::push_update(&[$($nmu),+], &mut self.e);
                }
            }

            #[inline]
            pub fn pop(&mut self) -> Option<($($ty),+)> {
                if self.e.len == 0 {
                    None
                } else {
                    unsafe {
                        self.e.len -= 1;
                        let len = self.e.len;
                        let ($($nm),+) = self.as_mut_slices();
                        Some(($(ptr::read($nm.get_unchecked(len))),+))
                    }
                }
            }

            #[inline]
            pub fn append(&mut self, other: &mut Self) {
                if self.is_boring() {
                    self.e.len = self.e.len.checked_add(other.len()).expect("length overflow");
                    other.e.len = 0;
                    return ;
                }
                unsafe {
                    let space = unadorned::calc_reserve_space(&self.e, 1);
                    $(let $nmu = self.$nm.append(&self.e, &other.$nm, &other.e, &space));+;
                    unadorned::append_update(&[$($nmu),+], &mut self.e, &mut other.e, space);
                }
            }

            #[inline]
            pub fn clear(&mut self) { self.truncate(0) }

            // abusing type/value namespace separation.
            // nothing else works, for serious.
            // in particular, "extend<$(Iterator<Item=$ty>),+>" and
            // "extend<$(concat_idents!(I,$ty),+)>" both don't work.
            #[allow(non_camel_case_types)]
            pub fn extend<$($nm),+>(&mut self, $($nm: $nm),+)
                where $($nm: Iterator<Item=$ty>),+
            {
                unsafe {
                    let (lower, _) = first!($($nm.size_hint()),+);
                    let space = unadorned::calc_reserve_space(&self.e, lower);
                    $(let $nmu = self.$nm.extend(&self.e, &space, $nm));+;
                    unadorned::extend_update(&[$($nmu),+], &mut self.e);
                }
            }

            #[allow(non_camel_case_types)]
            pub fn from_iters<$($nm),+>($($nm:$nm),+) -> $soa<$($ty),+>
                where $($nm: Iterator<Item=$ty>),+
            {
                let mut v = $soa::new();
                v.extend($($nm),+);
                v
            }
        }

        impl<$($ty: Clone),+> $soa<$($ty),+> {
            #[inline]
            pub fn resize(&mut self, new_len: usize, value: ($($ty),+)) {
                let len = self.len();
                if new_len > len {
                    let ($($nm),+) = value;
                    self.extend($(repeat($nm).take(new_len - len)),+);
                } else {
                    self.truncate(new_len)
                }
            }

            #[inline]
            pub fn push_all(&mut self, $($nm: &[$ty]),+) {
                unsafe {
                    let firstlen = first!($($nm.len()),+);
                    if anyrest!($(firstlen != $nm.len()),+) {
                        panic!("lengths not equal")
                    }
                    let space = unadorned::calc_reserve_space(&self.e, firstlen);
                    $(let $nmu = self.$nm.push_all($nm, &self.e, &space));+;
                    unadorned::push_all_update(&[$($nmu),+], &mut self.e, firstlen, space);
                }
            }
        }

        impl<$($ty: Clone),+> Clone for $soa<$($ty),+> {
            #[inline]
            fn clone(&self) -> $soa<$($ty),+> {
                let mut ret = $soa::new();
                let ($($nm),+) = self.as_slices();
                ret.push_all($($nm),+);
                ret
            }
            // clone_from requires figuring out zip
        }
        impl<S: hash::Writer + hash::Hasher, $($ty: Hash<S>),+> Hash<S> for $soa<$($ty),+> {
            #[inline]
            fn hash(&self, state: &mut S) {
                self.as_slices().hash(state)
            }
        }

        #[allow(non_camel_case_types)]
        impl<$($nm),+, $($nmu),+> PartialEq<$soa<$($nmu),+>> for $soa<$($nm),+>
            where $($nm: PartialEq<$nmu>),+
        {
            #[inline]
            fn eq(&self, other: &$soa<$($nmu),+>) -> bool {
                let ($($nm),+) = self.as_slices();
                let ($($nmu),+) = other.as_slices();
                $(PartialEq::eq($nm, $nmu))&&+
            }
            #[inline]
            fn ne(&self, other: &$soa<$($nmu),+>) -> bool {
                let ($($nm),+) = self.as_slices();
                let ($($nmu),+) = other.as_slices();
                $(PartialEq::ne($nm, $nmu))||+
            }
        }
    }
}


// the need to write out these names is also gross.
gen_soa!(Soa5 Soa5ZipIter Soa5ZipIterMut | A, B, C, D, E    | d1, d2, d3, d4, d5     | d1u, d2u, d3u, d4u, d5u);
gen_soa!(Soa6 Soa6ZipIter Soa6ZipIterMut | A, B, C, D, E, F | d1, d2, d3, d4, d5, d6 | du1, du2, du3, du4, du5, du6);

// couldn't quite figure out a way to get the macro transcriber to like doing
// complicated things to iter::Map's template in zip_iter, on which a number of further
// things depend---oh well!
