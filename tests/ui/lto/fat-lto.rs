//@ run-pass
//@ compile-flags: -Clto=fat
//@ no-prefer-dynamic
//@ needs-lto-support
//@ ignore-backends: gcc

fn main() {
    println!("hello!");
}
