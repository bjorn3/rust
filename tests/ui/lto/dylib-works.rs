//@ run-pass
//@ needs-lto-support

//@ aux-build:dylib.rs

extern crate dylib;

fn main() {
    dylib::foo(1);
}
