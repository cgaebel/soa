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

mod unadorned;
#[cfg(test)] mod test;

pub mod soa2;

pub use soa2::Soa2;
