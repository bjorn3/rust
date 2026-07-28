//! Implementation of Rust panics via process aborts
//!
//! When compared to the implementation via unwinding, this crate is *much*
//! simpler! That being said, it's not quite as versatile, but here goes!

#![no_std]
#![unstable(feature = "panic_abort", issue = "32837")]
#![doc(issue_tracker_base_url = "https://github.com/rust-lang/rust/issues/")]
#![panic_runtime]
#![feature(panic_runtime)]
#![feature(std_internals)]
#![feature(staged_api)]
#![feature(rustc_attrs)]
#![allow(internal_features)]

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "zkvm")]
mod zkvm;

use alloc::boxed::Box;
use alloc::panicking::{PanicPayload, UnwindResult};
use core::any::Any;

#[rustc_std_internal_symbol]
pub unsafe fn __rust_panic_cleanup(_: *mut u8) -> Box<dyn Any + Send + 'static> {
    unreachable!()
}

// "Leak" the payload and shim to the relevant abort on the platform in question.
#[rustc_std_internal_symbol]
pub fn __rust_start_panic(_payload: &mut dyn PanicPayload) -> UnwindResult {
    // FIXME move to libstd to allow reusing from panic_unwind
    // Android has the ability to attach a message as part of the abort.
    #[cfg(target_os = "android")]
    android::android_set_abort_message(_payload);
    #[cfg(target_os = "zkvm")]
    zkvm::zkvm_set_abort_message(_payload);

    UnwindResult::PanicAbort
}
