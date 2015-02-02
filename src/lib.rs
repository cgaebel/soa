//! Growable struct-of-array types with heap allocated contents.
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

pub mod soa2;

mod unadorned;
#[cfg(test)] mod test;

pub use soa2::Soa2;
