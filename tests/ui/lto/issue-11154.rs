//@ build-fail
//@ compile-flags: -C lto -C prefer-dynamic
//@ needs-lto-support

fn main() {}

//~? ERROR cannot prefer dynamic linking when performing LTO
