//@ run-pass
// Test that building a crate as both an rlib and a crate type that
// needs an allocator shim in the same invocation doesn't end up with
// an allocator shim in the rlib when compiling with LTO.

//@ aux-build:mixed-allocator-shim-aux.rs
//@ compile-flags: -Clto=fat
//@ no-prefer-dynamic
//@ needs-rust-lld

extern crate mixed_allocator_shim_aux;

fn main() {
    mixed_allocator_shim_aux::foo();
}
