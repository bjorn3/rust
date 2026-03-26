//@ compile-flags: -Clinker-plugin-lto -Copt-level=z
//@ build-pass
//@ no-prefer-dynamic
//@ needs-lto-support

#![crate_type = "rlib"]

pub fn foo() {}
