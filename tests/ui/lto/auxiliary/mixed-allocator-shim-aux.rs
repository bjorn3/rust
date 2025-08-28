//@ compile-flags: --crate-type staticlib,dylib -Zstaticlib-prefer-dynamic -Zdylib-lto -Clto=fat
//@ no-prefer-dynamic

pub fn foo() {}
