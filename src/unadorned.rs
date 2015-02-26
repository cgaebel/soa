use alloc::heap::{EMPTY, allocate, reallocate, deallocate};
use collections::vec;
use core::cmp::max;
use core::mem;
use core::num::{Int, UnsignedInt};
use core::nonzero::NonZero;
use core::ptr;
use core::slice;
use core::usize;

#[derive(Debug, PartialEq, Eq)]
pub struct Extent {
    pub len: usize,
    pub cap: usize,
}

impl Copy for Extent {}

fn byte_length_of<A>(capacity: usize) -> usize {
    mem::size_of::<A>().checked_mul(capacity).expect("capacity overflow")
}

unsafe fn my_alloc<A>(capacity: usize) -> NonZero<*mut A> {
    if mem::size_of::<A>() == 0 {
        NonZero::new(EMPTY as *mut A)
    } else {
        let ptr = allocate(byte_length_of::<A>(capacity), mem::align_of::<A>());
        if ptr.is_null() { ::alloc::oom() }
        NonZero::new(ptr as *mut A)
    }
}

#[inline(never)]
unsafe fn alloc_or_realloc<A>(ptr: *mut A, old_size: usize, size: usize) -> NonZero<*mut A> {
    let ret =
        if old_size == 0 {
            allocate(size, mem::min_align_of::<A>())
        } else {
            reallocate(ptr as *mut u8, old_size, size, mem::min_align_of::<A>())
        };

    if ret.is_null() { ::alloc::oom() }

    NonZero::new(ret as *mut A)
}

#[inline]
unsafe fn dealloc<A>(ptr: *mut A, cap: usize) {
    if mem::size_of::<A>() == 0 { return }

    deallocate(
        ptr as *mut u8,
        cap * mem::size_of::<A>(),
        mem::min_align_of::<A>());
}

#[must_use]
struct NewUpdate;

#[inline]
pub fn new_update(_: &[NewUpdate]) -> Extent {
    Extent { len: 0, cap: 0 }
}

#[must_use]
struct WithCapUpdate;

#[inline]
pub fn with_capacity_update(_: &[WithCapUpdate], is_boring: bool, new_cap: usize) -> Extent {
    let len = 0;
    let cap =
        if is_boring {
            usize::MAX
        } else {
            new_cap
        };

    Extent { len: len, cap: cap }
}

#[must_use]
struct FromRawPartsUpdate;

#[inline]
pub fn from_raw_parts_update(_: &[FromRawPartsUpdate], len: usize, cap: usize) -> Extent {
    Extent { len: len, cap: cap }
}

#[must_use]
struct FromRawBufsUpdate;

#[inline]
pub fn from_raw_bufs_update(_: &[FromRawBufsUpdate], elts: usize) -> Extent {
    Extent { len: elts, cap: elts }
}

#[must_use]
#[derive(Clone)]
struct ReserveCalc(usize);

#[inline]
pub fn calc_reserve_space(e: &Extent, additional: usize) -> Option<ReserveCalc> {
    if e.cap - e.len >= additional { return None }

    Some(ReserveCalc(
        e.len
        .checked_add(additional)
        .and_then(|base_len| base_len.checked_next_power_of_two())
        .expect("`usize` overflow")))
}

#[must_use]
struct ReserveUpdate;

#[inline]
pub fn reserve_update(_: &[ReserveUpdate], calc: ReserveCalc, e: &mut Extent) {
    e.cap = calc.0;
}

#[inline]
pub fn calc_reserve_exact_space(e: &Extent, additional: usize) -> Option<ReserveCalc> {
    if e.cap - e.len >= additional { return None }

    Some(ReserveCalc(
        e.len
        .checked_add(additional)
        .expect("`usize` overflow")))
}

#[must_use]
struct ShrinkToFitUpdate;

#[inline]
pub fn shrink_to_fit_update(_: &[ShrinkToFitUpdate], e: &mut Extent) {
    e.cap = e.len;
}

#[must_use]
struct TruncateUpdate;

#[inline]
pub fn truncate_update(_: &[TruncateUpdate], new_len: usize, e: &mut Extent) {
    e.len = new_len;
}

#[must_use]
struct InsertUpdate;

#[inline]
pub fn insert_update(_: &[InsertUpdate], calc: Option<ReserveCalc>, e: &mut Extent) {
    e.len += 1;
    calc.map(|calc| { e.cap = calc.0; });
}

#[must_use]
struct RemoveUpdate;

#[inline]
pub fn remove_update(_: &[RemoveUpdate], e: &mut Extent) {
    e.len -= 1;
}

#[must_use]
struct PushUpdate;

#[inline]
pub fn push_update(_: &[PushUpdate], e: &mut Extent) {
    if e.len == e.cap {
        e.cap = max(e.cap, 2) * 2;
    }

    e.len += 1;
}

#[must_use]
struct AppendUpdate;

#[inline]
pub fn append_update(_: &[AppendUpdate], e: &mut Extent, other_e: &mut Extent, space: Option<ReserveCalc>) {
    e.len += other_e.len;
    space.map(|calc| { e.cap = calc.0 });

    other_e.len = 0;
}

#[must_use]
struct PushAllUpdate;

#[inline]
pub fn push_all_update(_: &[PushAllUpdate], e: &mut Extent, len: usize, space: Option<ReserveCalc>) {
    e.len += len;
    space.map(|calc| { e.cap = calc.0 });
}

#[must_use]
#[derive(Debug)]
struct ExtendUpdate(Extent);

pub fn extend_update(extents: &[ExtendUpdate], e: &mut Extent) {
    let first_ext = extents[0].0;

    if extents.iter().any(|e| e.0 != first_ext) {
        // TODO: Clean up the excess elements added. This is a little tricky, since
        // the pointers need to be passed to extend_update, without slowing down
        // the fast path.
        panic!("`extend` called with iterators with unequal size: {:?}. Memory has been leaked!", extents);
    }

    *e = first_ext;
}

pub struct Unadorned<T> {
    ptr: NonZero<*mut T>,
}

unsafe impl<T: Send> Send for Unadorned<T> {}
unsafe impl<T: Sync> Sync for Unadorned<T> {}

impl<T> Unadorned<T> {
    pub fn is_boring(&self) -> bool {
        mem::size_of::<T>() == 0
    }

    #[inline]
    pub fn shallow_copy(&self) -> Self {
        Unadorned { ptr: self.ptr }
    }

    #[inline]
    pub unsafe fn new() -> (Unadorned<T>, NewUpdate) {
        (Unadorned {
            ptr: NonZero::new(EMPTY as *mut T),
        }, NewUpdate)
    }

    #[inline]
    pub unsafe fn with_capacity(cap: usize) -> (Unadorned<T>, WithCapUpdate) {
        (Unadorned {
            ptr: my_alloc::<T>(cap),
        }, WithCapUpdate)
    }

    #[inline]
    pub unsafe fn from_raw_parts(ptr: *mut T) -> (Unadorned<T>, FromRawPartsUpdate) {
        assert!(!ptr.is_null());
        (Unadorned {
            ptr: NonZero::new(ptr),
        }, FromRawPartsUpdate)
    }

    #[inline]
    pub unsafe fn as_vec(&self, e: &Extent) -> Vec<T> {
        Vec::from_raw_parts(*self.ptr, e.len, e.cap)
    }

    #[inline]
    pub fn as_ptr(&self) -> *const T {
        *self.ptr as *const T
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        *self.ptr
    }

    pub unsafe fn from_raw_bufs(src: *const T, elts: usize) -> (Unadorned<T>, FromRawBufsUpdate) {
        let dst = my_alloc::<T>(elts);
        ptr::copy_nonoverlapping(*dst, src, elts);
        (Unadorned {
            ptr: dst,
        }, FromRawBufsUpdate)
    }

    #[inline]
    pub unsafe fn reserve(&mut self, e: &Extent, space_needed: &ReserveCalc) -> ReserveUpdate {
        let old_cap = e.cap;
        let new_cap = space_needed.0;

        if self.is_boring() { return ReserveUpdate }

        let size = byte_length_of::<T>(new_cap);
        self.ptr = alloc_or_realloc(*self.ptr, old_cap * mem::size_of::<T>(), size);

        ReserveUpdate
    }

    pub unsafe fn shrink_to_fit(&mut self, e: &Extent) -> ShrinkToFitUpdate {
        if self.is_boring() { return ShrinkToFitUpdate }

        if e.len == 0 {
            if e.cap != 0 {
                dealloc(*self.ptr, e.cap);
            }
        } else {
            let new_ptr =
                reallocate(*self.ptr as *mut u8,
                           e.cap * mem::size_of::<T>(),
                           e.len * mem::size_of::<T>(),
                           mem::min_align_of::<T>()) as *mut T;
            if new_ptr.is_null() { ::alloc::oom() }
            self.ptr = NonZero::new(new_ptr);
        }

        ShrinkToFitUpdate
    }

    pub unsafe fn truncate(&mut self, len: usize, e: &Extent) -> TruncateUpdate {
        if self.is_boring() { return TruncateUpdate }

        let mut real_len = e.len;

        while len < real_len {
            real_len -= 1;
            ptr::read(self.ptr.offset(real_len as isize));
        }

        TruncateUpdate
    }

    #[inline]
    pub unsafe fn as_slice<'a>(&'a self, len: usize) -> &'a [T] {
        let p: &'a *const T = mem::transmute(&*self.ptr);
        slice::from_raw_parts(*p, len)
    }

    #[inline]
    pub unsafe fn as_mut_slice<'a>(&'a mut self, len: usize) -> &'a mut [T] {
        slice::from_raw_parts_mut(*self.ptr, len)
    }

    #[inline]
    pub unsafe fn into_iter(self, e: &Extent) -> vec::IntoIter<T> {
        let r = Vec::from_raw_parts(*self.ptr, e.len, e.cap);
        r.into_iter()
    }

    pub unsafe fn insert(&mut self,
                  index: usize, x: T,
                  e: &Extent, space_needed: &Option<ReserveCalc>) -> InsertUpdate {
        let _ = space_needed.as_ref().map(|space| self.reserve(e, space));

        let p = self.ptr.offset(index as isize);
        ptr::copy(p.offset(1), &*p, e.len - index);
        ptr::write(&mut *p, x);

        InsertUpdate
    }

    pub unsafe fn remove(&mut self, index: usize, e: &Extent) -> (T, RemoveUpdate) {
        let ptr = self.ptr.offset(index as isize);
        let ret = ptr::read(ptr);
        ptr::copy(ptr, &*ptr.offset(1), e.len - index - 1);
        (ret, RemoveUpdate)
    }

    unsafe fn make_room_for_one(&mut self, e: &Extent) {
        if self.is_boring() { return }

        let old_size = e.cap * mem::size_of::<T>();
        let size = max(old_size, 2 * mem::size_of::<T>()) * 2;
        if old_size > size { panic!("capacity overflow") }
        self.ptr = alloc_or_realloc(*self.ptr, old_size, size);
    }

    #[inline]
    pub unsafe fn push(&mut self, value: T, e: &Extent) -> PushUpdate {
        if e.len == e.cap {
            self.make_room_for_one(e);
        }

        ptr::write(self.ptr.offset(e.len as isize), value);
        PushUpdate
    }

    #[inline]
    pub unsafe fn append(&mut self, self_e: &Extent, other: &Self, other_e: &Extent, space: &Option<ReserveCalc>) -> AppendUpdate {
        space.as_ref().map(|space| self.reserve(self_e, space));
        ptr::copy_nonoverlapping(*self.ptr, *other.ptr, other_e.len);

        AppendUpdate
    }

    pub unsafe fn extend<I: Iterator<Item=T>>(&mut self, e: &Extent, space: &Option<ReserveCalc>, i: I) -> ExtendUpdate {
        let mut this_extent: Extent = *e;

        space.as_ref().map(|space| {
            let ru = self.reserve(e, space);
            reserve_update(&[ru], (*space).clone(), &mut this_extent);
        });

        for x in i {
            let u = self.push(x, &this_extent);
            push_update(&[u], &mut this_extent);
        }

        ExtendUpdate(this_extent)
    }

    pub unsafe fn drop(&self, e: &Extent) {
        for x in self.as_slice(e.len) {
            drop(ptr::read(x));
        }
        dealloc(*self.ptr, e.cap);
    }
}

impl<T: Clone> Unadorned<T> {
    #[inline]
    pub unsafe fn push_all(&mut self, x: &[T], e: &Extent, space: &Option<ReserveCalc>) -> PushAllUpdate {
        space.as_ref().map(|space| self.reserve(e, space));

        let mut len = e.len;

        for i in range(0, x.len()) {
            // LLVM is easily confused. This is carefully constructed such that
            // Copy types get a memcpy.
            ptr::write(self.ptr.offset(len as isize), x.get_unchecked(i).clone());
            len += 1;
        }

        PushAllUpdate
    }
}
