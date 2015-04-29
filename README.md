SoA
=====

Vector types, but instead of being represented as Array-Of-Struct, data is stored
as a Struct-Of-Arrays, or SoA.

[![crates.io](https://img.shields.io/crates/v/soa.svg)](https://crates.io/crates/soa/)

[![Build Status](https://travis-ci.org/cgaebel/soa.svg?branch=master)](https://travis-ci.org/cgaebel/soa)

Data stored in SoA is meant to be processed with SIMD operations, and as such,
all arrays are aligned to 16 bytes.

A large subset of the `std::Vec` interface is supported, as well as some extras
to make writing efficient code more natural.

Documentation
--------------

See the very thorough [API Docs](https://cgaebel.github.io/soa/).