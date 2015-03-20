use Soa2;

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

    for i in 0..16 {
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

    v.extend(0..3, 4..7);
    for i in (0..3).zip(4..7) {
        w.push(i);
    }

    assert_eq!(v, w);

    v.extend(3..10, 7..14);
    for i in (3..10).zip(7..14) {
        w.push(i);
    }

    assert_eq!(v, w);
}

#[test]
fn test_clone() {
    let v: Soa2<i32, i32> = Soa2::new();
    let mut w: Soa2<i32, i32> = Soa2::new();

    let elems = [ 1, 2, 3 ];
    w.push_all(&elems, &elems);

    assert_eq!(v, v.clone());
    let z = w.clone();
    assert_eq!(w, z);
    assert!(w.as_slices().0.as_ptr() != z.as_slices().0.as_ptr());
    assert!(w.as_slices().1.as_ptr() != z.as_slices().1.as_ptr());
}

#[test]
fn test_clone_from() {
    let mut v = Soa2::new();
    let mut three = Soa2::new();
    let three_elems = [ Box::new(1), Box::new(2), Box::new(3) ];
    three.push_all(&three_elems, &three_elems);
    let mut two = Soa2::new();
    let two_elems = [ Box::new(4), Box::new(5) ];
    two.push_all(&two_elems, &two_elems);

    v.clone_from(&three);
    assert_eq!(v, three);

    v.clone_from(&three);
    assert_eq!(v, three);

    v.clone_from(&two);
    assert_eq!(v, two);

    v.clone_from(&three);
    assert_eq!(v, three);
}

#[test]
fn test_retain() {
    let vs = [ 1i32, 2, 3, 4 ];
    let mut v = Soa2::new();
    v.push_all(&vs, &vs);
    v.retain(|(&x, _)| x % 2i32 == 0);
    assert_eq!(v.as_slices(), (&[2, 4][..], &[2, 4][..]));
}

#[test]
fn test_zero_sized_values() {
    let mut v = Soa2::new();
    assert_eq!(v.len(), 0);
    v.push(((), ()));
    assert_eq!(v.len(), 1);
    v.push(((), ()));
    assert_eq!(v.len(), 2);
    assert_eq!(v.pop(), Some(((), ())));
    assert_eq!(v.pop(), Some(((), ())));
    assert_eq!(v.pop(), None);

    assert_eq!(v.iters().0.count(), 0);
    v.push(((), ()));
    assert_eq!(v.iters().1.count(), 1);
    v.push(((), ()));
    assert_eq!(v.iters().0.count(), 2);

    for (&(), &()) in v.iters().0.zip(v.iters().1) {}

    assert_eq!(v.iters_mut().0.count(), 2);
    v.push(((), ()));
    assert_eq!(v.iters_mut().1.count(), 3);
    v.push(((), ()));
    assert_eq!(v.iters_mut().0.count(), 4);

    for (&mut (), &mut ()) in { let (a, b) = v.iters_mut(); a.zip(b) } {}
    unsafe { v.set_len(0); }
    assert_eq!(v.iters_mut().0.count(), 0);
}

#[test]
fn test_zip_unzip() {
    let (z1x, z1y) = ([ 1i32, 2, 3 ], [ 4i32, 5, 6 ]);
    let mut z1 = Soa2::new();
    z1.push_all(&z1x[..], &z1y[..]);

    let (left, right) = z1.into_vecs();
    assert_eq!(&left[..], &z1x[..]);
    assert_eq!(&right[..], &z1y[..]);
}

#[test]
fn test_unsafe_ptrs() {
    unsafe {
        let a = [1i32, 2, 3];
        let ptr = a.as_ptr();
        let b = Soa2::from_raw_bufs(ptr, ptr, 3);
        assert_eq!(b.as_slices(), (&[1, 2, 3][..], &[1, 2, 3][..]));

        let c = [1i32, 2, 3, 4, 5];
        let ptr = c.as_ptr();
        let d = Soa2::from_raw_bufs(ptr, ptr, 5);
        assert_eq!(d.as_slices(), (&c[..], &c[..]));
    }
}

#[test]
fn test_vec_truncate_drop() {
    static mut drops: usize = 0;
    #[derive(Clone)]
    struct Elem(usize);
    impl Drop for Elem {
        fn drop(&mut self) {
            unsafe { drops += 1; }
        }
    }

    let mut v = Soa2::new();
    v.push_all(
        &[Elem(1), Elem(2), Elem(3), Elem(4), Elem(5)][..],
        &[Elem(10), Elem(20), Elem(30), Elem(40), Elem(50)][..]);

    assert_eq!(unsafe { drops }, 10 + 0);
    v.truncate(3);
    assert_eq!(unsafe { drops }, 10 + 4);
    v.truncate(0);
    assert_eq!(unsafe { drops }, 10 + 10);
}

#[test]
#[should_panic]
fn test_swap_remove_empty() {
    let mut v: Soa2<i32, i32> = Soa2::new();
    v.swap_remove(0);
}

#[test]
fn test_move_iter_unwrap() {
    let mut v: Soa2<u32, u32> = Soa2::with_capacity(7);
    v.push((1, 10));
    v.push((2, 20));
    let (p0, p1) = v.as_ptrs();
    let (v0, v1) = v.into_iters();
    let v0 = v0.into_inner();
    let v1 = v1.into_inner();

    assert_eq!(v0.as_ptr(), p0);
    assert_eq!(v1.as_ptr(), p1);

    assert_eq!(v0.capacity(), 7);
    assert_eq!(v1.capacity(), 7);

    assert_eq!(v0.len(), 0);
    assert_eq!(v1.len(), 0);
}
