#!/usr/bin/env bash

set -eou pipefail

version=$(cat src/version | tr -d '\n')
host=$(rustc --print host-tuple | tr -d '\n')

./x.py build library --stage 1
echo 'fn main() { println!("Hello World!"); }' | CFG_COMPILER_HOST_TRIPLE="${host}" RUSTC_INSTALL_BINDIR=bin CFG_RELEASE_CHANNEL=dev CFG_VERSION="${version}-dev" CFG_RELEASE="${version}-dev" RUSTFLAGS="--check-cfg cfg(bootstrap)" RUSTC_BOOTSTRAP=1 MIRIFLAGS="-Zmiri-disable-isolation" time cargo +nightly miri run --manifest-path compiler/rustc/Cargo.toml -- --sysroot build/host/stage1 - -Zcodegen-backend=dummy
