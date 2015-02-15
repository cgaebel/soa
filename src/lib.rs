//! Growable struct-of-array types with heap allocated contents.
#![allow(unused_features)]

#![feature(alloc)]
#![feature(collections)]
#![feature(core)]
#![feature(hash)]
#![feature(test)]

#![feature(unsafe_destructor)]
#![feature(unsafe_no_drop_flag)]

extern crate alloc;
extern crate collections;
extern crate core;

pub mod soa2;
pub mod soa3;
pub mod soa4;

mod unadorned;
#[cfg(test)] mod test;

pub use soa2::Soa2;
pub use soa3::Soa3;
pub use soa4::Soa4;
