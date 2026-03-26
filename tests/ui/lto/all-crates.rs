//@ run-pass

//@ compile-flags: -Clto=thin
//@ no-prefer-dynamic
//@ needs-lto-support
//@ ignore-backends: gcc

fn main() {
    println!("hello!");
}
