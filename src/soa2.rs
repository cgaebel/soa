use collections::vec;

use core::cmp::Ordering; use core::default::Default; use core::fmt::{Debug,
Formatter, Result}; use core::hash::{Hash, Hasher}; use core::iter::{self,
repeat}; use core::mem; use core::num::Int; use core::ptr; use core::slice;

use unadorned::{self, Unadorned, Extent};

/// A growable struct-of-2-arrays type, with heap allocated contents.
///
/// This structure is analogous to a `Vec<(A, B)>`, but instead of laying out
/// the tuples sequentially in memory, each row gets its own allocation. For
/// example, an `Soa2<f32, i64>` will contain two inner arrays: one of `f32`s,
/// and one of `i64`s.
#[unsafe_no_drop_flag]
pub struct Soa2<A, B> {
    d0: Unadorned<A>,
    d1: Unadorned<B>,
    e:  Extent,
}

impl<A, B> Soa2<A, B> {
    /// Constructs a new, empty `Soa2`.
    ///
    /// The SoA will not allocate until elements are pushed onto it.
    pub fn new() -> Soa2<A, B> {
        unsafe {
            let (d0, d0u) = Unadorned::new();
            let (d1, d1u) = Unadorned::new();

            let e = unadorned::new_update(&[d0u, d1u]);

            Soa2 { d0: d0, d1: d1, e: e }
        }
    }

    /// Returns `true` if all our elements are zero-sized types.
    #[inline]
    fn is_boring(&self) -> bool {
        self.d0.is_boring() && self.d1.is_boring()
    }

    /// Constructs a new, empty `Soa2` with the specified capacity.
    ///
    /// The SoA will be able to hold exactly `capacity` tuples of elements
    /// without reallocating.
    ///
    /// If `capacity` is 0, the SoA will not allocate.
    ///
    /// It is important to note that this function does not specify the *length*
    /// of the soa, but only the *capacity*.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Soa2<A, B> {
        unsafe {
            let (d0, d0u) = Unadorned::with_capacity(capacity);
            let (d1, d1u) = Unadorned::with_capacity(capacity);

            let is_boring =
                mem::size_of::<A>() == 0
             && mem::size_of::<B>() == 0;

            let e = unadorned::with_capacity_update(&[d0u, d1u], is_boring, capacity);

            Soa2 { d0: d0, d1: d1, e: e }
        }
    }

    /// Constructs a `Soa2` directly from the raw components of another.
    ///
    /// This is highly unsafe, and no invariants are checked.
    #[inline]
    pub unsafe fn from_raw_parts(
        ptra: *mut A, ptrb: *mut B, len: usize, cap: usize) -> Soa2<A, B> {
        let (d0, d0u) = Unadorned::from_raw_parts(ptra);
        let (d1, d1u) = Unadorned::from_raw_parts(ptrb);

        let e = unadorned::from_raw_parts_update(&[d0u, d1u], len, cap);

        Soa2 { d0: d0, d1: d1, e: e }
    }

    /// Constructs a `Soa2` by copying the elements from raw pointers.
    ///
    /// This function will copy `elts` contiguous elements from each of the
    /// pointers into a new allocation owned by the returned `Soa2`. The elements
    /// of the buffer are copied without cloning, as if `ptr::read()` were called
    /// on them.
    #[inline]
    pub unsafe fn from_raw_bufs(ptra: *const A, ptrb: *const B, elts: usize) -> Soa2<A, B> {
        let (d0, d0u) = Unadorned::from_raw_bufs(ptra, elts);
        let (d1, d1u) = Unadorned::from_raw_bufs(ptrb, elts);

        let e = unadorned::from_raw_bufs_update(&[d0u, d1u], elts);

        Soa2 { d0: d0, d1: d1, e: e }
    }

    /// Constructs a `Soa2` directly from vectors of its components.
    ///
    /// This function will panic if the lengths of the vectors don't match.
    ///
    /// If the capacity of the vectors don't match they will be reallocated to
    /// have matching capacities.
    ///
    /// Otherwise, no allocation will be performed and the SoA will only take
    /// ownership of the elements in the vectors.
    pub fn from_vecs(mut v0: Vec<A>, mut v1: Vec<B>) -> Soa2<A, B> {
        assert_eq!(v0.len(), v1.len());
        if v0.capacity() != v1.capacity() {
            v0.shrink_to_fit();
            v1.shrink_to_fit();
        }
        let len = v0.len();
        let cap = v0.capacity();

        unsafe {
            let ret = Soa2::from_raw_parts(
                v0.as_ptr() as *mut A,
                v1.as_ptr() as *mut B,
                len, cap);
            mem::forget(v0);
            mem::forget(v1);
            ret
        }
    }

    /// Returns the number of tuples stored in the SoA.
    #[inline]
    pub fn len(&self) -> usize {
        self.e.len
    }

    /// Returns `true` if the SoA contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Sets the length of a vector.
    ///
    /// This will explicitly set the size of the soa, without actually
    /// modifying its buffers, so it is up to the caller to ensure that the
    /// SoA is actually the specified size.
    #[inline]
    pub unsafe fn set_len(&mut self, len: usize) {
        self.e.len = len;
    }

    /// Returns the number of elements the SoA can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.e.cap
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given SoA. The collection may reserve more space to avoid frequent
    /// reallocations.
    ///
    /// Panics if the new capacity overflows `usize`.
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

    /// Reserves the minimum capacity for exactly `additional` more elements to
    /// be inserted in the given SoA. Does nothing if the capacity is already
    /// sufficient.
    ///
    /// Note that the allocator may give the collection more space than it
    /// requests. Therefore, capacity can not be relied upon to be precisely
    /// minimal. Prefer `reserve` if future insertions are expected.
    ///
    /// Panics if the new capacity overflows `usize`.
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

    /// Shrinks the capacity of the SoA as much as possible.
    ///
    /// It will drop down as close as possible to the length, but the allocator
    /// may still inform the SoA that there is space for a few more elements.
    pub fn shrink_to_fit(&mut self) {
        if self.is_boring() { return }

        unsafe {
            let d0u = self.d0.shrink_to_fit(&self.e);
            let d1u = self.d1.shrink_to_fit(&self.e);

            unadorned::shrink_to_fit_update(&[d0u, d1u], &mut self.e);
        }
    }

    /// Shorten a SoA, dropping excess elements.
    ///
    /// If `len` is greater than the soa's current length, this has no effect.
    pub fn truncate(&mut self, len: usize) {
        if self.is_boring() { return }

        unsafe {
            let d0u = self.d0.truncate(len, &self.e);
            let d1u = self.d1.truncate(len, &self.e);

            unadorned::truncate_update(&[d0u, d1u], len, &mut self.e);
        }
    }

    /// Returns mutable slices over the SoA's elements.
    #[inline]
    pub fn as_mut_slices<'a>(&'a mut self) -> (&'a mut [A], &'a mut [B]) {
        unsafe {
            let len = self.e.len;
            (self.d0.as_mut_slice(len), self.d1.as_mut_slice(len))
        }
    }

    /// Returns slices over the SoA's elements.
    #[inline]
    pub fn as_slices<'a>(&'a self) -> (&'a [A], &'a [B]) {
        unsafe {
            let len = self.e.len;
            (self.d0.as_slice(len), self.d1.as_slice(len))
        }
    }

    /// Returns iterators over the SoA's elements.
    #[inline]
    pub fn iters(&self) -> (slice::Iter<A>, slice::Iter<B>) {
        let (d0, d1) = self.as_slices();
        (d0.iter(), d1.iter())
    }

    /// Returns a single iterator over the SoA's elements, zipped up.
    #[inline]
    pub fn zip_iter(&self) -> iter::Zip<slice::Iter<A>, slice::Iter<B>> {
        let (d0, d1) = self.iters();
        d0.zip(d1)
    }

    /// Returns mutable iterators over the SoA's elements.
    #[inline]
    pub fn iters_mut(&mut self) -> (slice::IterMut<A>, slice::IterMut<B>) {
        let (d0, d1) = self.as_mut_slices();
        (d0.iter_mut(), d1.iter_mut())
    }

    /// Returns a single iterator over the SoA's elements, zipped up.
    #[inline]
    pub fn zip_iter_mut(&mut self) -> iter::Zip<slice::IterMut<A>, slice::IterMut<B>> {
        let (d0, d1) = self.iters_mut();
        d0.zip(d1)
    }

    /// Converts an SoA into iterators for each of its arrays.
    #[inline]
    pub fn into_iters(mut self) -> (vec::IntoIter<A>, vec::IntoIter<B>) {
        unsafe {
            let e_copy = self.e;
            self.e.cap = 0; // Will skip the drop. into_iter will handle it.
            (self.d0.shallow_copy().into_iter(&e_copy),
             self.d1.shallow_copy().into_iter(&e_copy))
        }
    }

    /// Converts an SoA into a pair of `Vec`s. This will neither allocator nor
    /// copy.
    #[inline]
    pub fn into_vecs(mut self) -> (Vec<A>, Vec<B>) {
        unsafe {
            let e_copy = self.e;
            self.e.cap = 0;
            (self.d0.shallow_copy().as_vec(&e_copy),
             self.d1.shallow_copy().as_vec(&e_copy))
        }
    }

    /// Returns a pair of pointers to the start of the data in an SoA.
    #[inline]
    pub fn as_ptrs(&self) -> (*const A, *const B) {
        (self.d0.as_ptr(), self.d1.as_ptr())
    }

    /// Returns a pair of pointers to the start of the mutable data in an SoA.
    #[inline]
    pub fn as_mut_ptrs(&mut self) -> (*mut A, *mut B) {
        (self.d0.as_mut_ptr(), self.d1.as_mut_ptr())
    }

    /// Removes an element from anywhere in the SoA and returns it, replacing it
    /// with the last element.
    ///
    /// This does not preserve ordering, but is O(1).
    ///
    /// Panics if `index` is out of bounds.
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

    /// Inserts an element at position `index` within the vector, shifting all
    /// elements after position `index` one position to the right.
    ///
    /// Panics if `index` is not between `0` and the SoA's length, inclusive.
    pub fn insert(&mut self, index: usize, element: (A, B)) {
        unsafe {
            assert!(index < self.e.len);

            let space = unadorned::calc_reserve_space(&self.e, 1);

            let d0u = self.d0.insert(index, element.0, &self.e, &space);
            let d1u = self.d1.insert(index, element.1, &self.e, &space);

            unadorned::insert_update(&[d0u, d1u], space, &mut self.e);
        }
    }

    /// Removes and returns the elements at position `index` within the SoA,
    /// shifting all elements after position `index` one position to the left.
    ///
    /// Panics if `index` is out of bounds.
    pub fn remove(&mut self, index: usize) -> (A, B) {
        unsafe {
            assert!(index < self.e.len);

            let (x0, d0u) = self.d0.remove(index, &self.e);
            let (x1, d1u) = self.d1.remove(index, &self.e);

            unadorned::remove_update(&[d0u, d1u], &mut self.e);
            (x0, x1)
        }
    }

    /// Returns only the element specified by the predicate.
    ///
    /// In other words, remove all elements `e` such that `f(&e)` returns false.
    /// This method operates in place and preserves the order of the retained
    /// elements.
    pub fn retain<F>(&mut self, mut f: F) where F: FnMut((&A, &B)) -> bool {
        let len = self.len();
        let mut del = 0;

        {
            let (d0, d1) = self.as_mut_slices();

            for i in range(0, len) {
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

    /// Appends an element to the back of a collection.
    ///
    /// Panics if the number of elements in the SoA overflows a `usize`.
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

    /// Removes the last element from a SoA and returns it, or `None` if empty.
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

    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    ///
    /// Panics if the number of elements in the SoA overflows a `usize`.
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

    /// Clears the SoA, removing all values.
    #[inline]
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    // TODO: map_in_place

    /// Extends the SoA with the elements yielded by arbitrary iterators.
    ///
    /// Panics (and leaks memory!) if the iterators yield a different number of
    /// elements.
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

    /// Constructs an `Soa2` with elements yielded by arbitrary iterators.
    ///
    /// Panics (and leaks memory!) if the iterators yield a different number of
    /// elements.
    pub fn from_iters<I0, I1>(i0: I0, i1: I1) -> Soa2<A, B>
        where I0: Iterator<Item=A>, I1: Iterator<Item=B> {
        let mut v = Soa2::new();
        v.extend(i0, i1);
        v
    }

    // TODO: dedup
}

impl<A: Clone, B: Clone> Soa2<A, B> {
    /// Resizes the SoA in-place so that `len()` is equal to `new_len`.
    ///
    /// Calls either `extend()` or `truncate()` depending on whether `new_len` is
    /// larger than the current value of `len()` or not.
    #[inline]
    pub fn resize(&mut self, new_len: usize, value: (A, B)) {
        let len = self.len();

        if new_len > len {
            self.extend(repeat(value.0).take(new_len - len), repeat(value.1).take(new_len - len));
        } else {
            self.truncate(new_len);
        }
    }

    /// Appends all elements in slices to the SoA.
    ///
    /// Iterates over the slices, clones each element, and then appends them to
    /// this SoA. The slices are traversed one at a time, in order.
    ///
    /// Panics if the slices are of different lengths.
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
            let self_len = self.len();
            let (sd0, sd1) = self.iters_mut();

            for (place, thing) in sd0.zip(od0.iter()) {
                place.clone_from(thing);
            }

            for (place, thing) in sd1.zip(od1.iter()) {
                place.clone_from(thing);
            }

            let s0 = &od0[self_len..];
            let s1 = &od1[self_len..];

            (s0, s1)
        };

        self.push_all(s0, s1);
    }
}

impl<A: Hash, B: Hash> Hash for Soa2<A, B> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
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
        && self.zip_iter().zip(other.iter()).all(
            |((a0, b0), &(ref a1, ref b1))| a0 == a1 && b0 == b1)
    }

    #[inline]
    fn ne(&self, other: &Vec<(A1, B1)>) -> bool {
        self.len() != other.len()
        || self.zip_iter().zip(other.iter()).any(
            |((a0, b0), &(ref a1, ref b1))| a0 != a1 || b0 != b1)
    }
}

impl<'b, A0, B0, A1, B1> PartialEq<&'b [(A1, B1)]> for Soa2<A0, B0>
  where A0: PartialEq<A1>, B0: PartialEq<B1> {
    #[inline]
    fn eq(&self, other: &&'b [(A1, B1)]) -> bool {
        self.len() == other.len()
        && self.zip_iter().zip(other.iter()).all(
            |((a0, b0), &(ref a1, ref b1))| a0 == a1 && b0 == b1)
    }

    #[inline]
    fn ne(&self, other: &&'b [(A1, B1)]) -> bool {
        self.len() != other.len()
        || self.zip_iter().zip(other.iter()).any(
            |((a0, b0), &(ref a1, ref b1))| a0 != a1 || b0 != b1)
    }
}

impl<'b, A0, B0, A1, B1> PartialEq<&'b mut [(A1, B1)]> for Soa2<A0, B0>
  where A0: PartialEq<A1>, B0: PartialEq<B1> {
    #[inline]
    fn eq(&self, other: &&'b mut [(A1, B1)]) -> bool {
        self.len() == other.len()
        && self.zip_iter().zip(other.iter()).all(
            |((a0, b0), &(ref a1, ref b1))| a0 == a1 && b0 == b1)
    }

    #[inline]
    fn ne(&self, other: &&'b mut [(A1, B1)]) -> bool {
        self.len() != other.len()
        || self.zip_iter().zip(other.iter()).any(
            |((a0, b0), &(ref a1, ref b1))| a0 != a1 || b0 != b1)
    }
}

impl<A: PartialOrd, B: PartialOrd> PartialOrd for Soa2<A, B> {
    #[inline]
    fn partial_cmp(&self, other: &Soa2<A, B>) -> Option<Ordering> {
        iter::order::partial_cmp(self.zip_iter(), other.zip_iter())
    }
}

impl<A: Eq, B: Eq> Eq for Soa2<A, B> {}

impl<A: Ord, B: Ord> Ord for Soa2<A, B> {
    #[inline]
    fn cmp(&self, other: &Soa2<A, B>) -> Ordering {
        iter::order::cmp(self.zip_iter(), other.zip_iter())
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
