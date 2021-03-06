//! Growable struct-of-array types with 16-byte aligned heap allocated contents.
#![feature(alloc)]
#![feature(collections)]
#![feature(core)]

#![feature(unsafe_no_drop_flag)]
#![feature(filling_drop)]

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
