//! These tests don't really assert anything, they just exercise the API.
//! This is primarily intended to be run under miri as a sanitizer.

#![allow(clippy::borrowed_box, clippy::drop_ref)]

use ptr_union::{Builder2, Builder4};

#[repr(align(4))]
#[derive(Debug, Default, Clone)]
struct BigA([u128; 16]);
#[repr(align(4))]
#[derive(Debug, Default, Clone)]
struct BigB([u128; 16]);
#[repr(align(4))]
#[derive(Debug, Default, Clone)]
struct BigC([u128; 16]);
#[repr(align(4))]
#[derive(Debug, Default, Clone)]
struct BigD([u128; 16]);

const BIG_UNION_PROOF_2: Builder2<Box<BigA>, Box<BigB>> = unsafe { Builder2::new_unchecked() };
const BIG_UNION_PROOF_3: Builder4<Box<BigA>, Box<BigB>, Box<BigC>> =
    unsafe { Builder4::new_unchecked() };
const BIG_UNION_PROOF_4: Builder4<Box<BigA>, Box<BigB>, Box<BigC>, Box<BigD>> =
    unsafe { Builder4::new_unchecked() };

#[test]
fn smoke() {
    let _ = BIG_UNION_PROOF_2.a(Default::default());
    let _ = BIG_UNION_PROOF_2.b(Default::default());
    let _ = BIG_UNION_PROOF_3.a(Default::default());
    let _ = BIG_UNION_PROOF_3.b(Default::default());
    let _ = BIG_UNION_PROOF_3.c(Default::default());
    let _ = BIG_UNION_PROOF_4.a(Default::default());
    let _ = BIG_UNION_PROOF_4.b(Default::default());
    let _ = BIG_UNION_PROOF_4.c(Default::default());
    let _ = BIG_UNION_PROOF_4.d(Default::default());
}

#[test]
fn smoke2() {
    let a = BIG_UNION_PROOF_2.a(Default::default());
    let b = BIG_UNION_PROOF_2.b(Default::default());
    assert!(a.is_a());
    assert!(b.is_b());
    assert!(a.clone_a().is_some());
    assert!(b.clone_b().is_some());
    a.with_a(|a: &Box<BigA>| drop(dbg!(a))).unwrap();
    b.with_b(|b: &Box<BigB>| drop(dbg!(b))).unwrap();
    a.unpack().pack(BIG_UNION_PROOF_2).into_a().unwrap();
    b.unpack().pack(BIG_UNION_PROOF_2).into_b().unwrap();
}

#[test]
fn smoke3() {
    let a = BIG_UNION_PROOF_3.a(Default::default());
    let b = BIG_UNION_PROOF_3.b(Default::default());
    let c = BIG_UNION_PROOF_3.c(Default::default());
    assert!(a.is_a());
    assert!(b.is_b());
    assert!(c.is_c());
    assert!(a.clone_a().is_some());
    assert!(b.clone_b().is_some());
    assert!(c.clone_c().is_some());
    a.with_a(|a: &Box<BigA>| drop(dbg!(a))).unwrap();
    b.with_b(|b: &Box<BigB>| drop(dbg!(b))).unwrap();
    c.with_c(|c: &Box<BigC>| drop(dbg!(c))).unwrap();
    a.unpack().pack(BIG_UNION_PROOF_3).into_a().unwrap();
    b.unpack().pack(BIG_UNION_PROOF_3).into_b().unwrap();
    c.unpack().pack(BIG_UNION_PROOF_3).into_c().unwrap();
}

#[test]
fn smoke4() {
    let a = BIG_UNION_PROOF_4.a(Default::default());
    let b = BIG_UNION_PROOF_4.b(Default::default());
    let c = BIG_UNION_PROOF_4.c(Default::default());
    let d = BIG_UNION_PROOF_4.d(Default::default());
    assert!(a.is_a());
    assert!(b.is_b());
    assert!(c.is_c());
    assert!(d.is_d());
    assert!(a.clone_a().is_some());
    assert!(b.clone_b().is_some());
    assert!(c.clone_c().is_some());
    assert!(d.clone_d().is_some());
    a.with_a(|a: &Box<BigA>| drop(dbg!(a))).unwrap();
    b.with_b(|b: &Box<BigB>| drop(dbg!(b))).unwrap();
    c.with_c(|c: &Box<BigC>| drop(dbg!(c))).unwrap();
    d.with_d(|d: &Box<BigD>| drop(dbg!(d))).unwrap();
    a.unpack().pack(BIG_UNION_PROOF_4).into_a().unwrap();
    b.unpack().pack(BIG_UNION_PROOF_4).into_b().unwrap();
    c.unpack().pack(BIG_UNION_PROOF_4).into_c().unwrap();
    d.unpack().pack(BIG_UNION_PROOF_4).into_d().unwrap();
}
