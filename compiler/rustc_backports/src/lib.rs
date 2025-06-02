//! This crate defines various extension traits and macros to backport newer
//! standard library features to older rustc versions to allow bootstrapping
//! rustc from older rustc versions.

#![allow(internal_features)]
#![feature(decl_macro)]
#![feature(sized_hierarchy)]
#![feature(staged_api)]

pub mod prelude {
    pub use crate::PtrExt as _;
}

// Copied from library/core/src/macros.rs of rustc 1.88.0
#[unstable(feature = "cfg_select", issue = "none")]
pub macro cfg_select {
    ({ $($tt:tt)* }) => {{
        $crate::cfg_select! { $($tt)* }
    }},
    (_ => { $($output:tt)* }) => {
        $($output)*
    },
    (
        $cfg:meta => $output:tt
        $($( $rest:tt )+)?
    ) => {
        #[cfg($cfg)]
        $crate::cfg_select! { _ => $output }
        $(
            #[cfg(not($cfg))]
            $crate::cfg_select! { $($rest)+ }
        )?
    },
}

#[cfg(bootstrap)]
#[unstable(feature = "sized_hierarchy", issue = "none")]
pub trait PointeeSized {}
#[cfg(bootstrap)]
impl<T: ?Sized> PointeeSized for T {}

#[cfg(not(bootstrap))]
pub use std::marker::PointeeSized;

pub trait PtrExt<T> {
    unsafe fn offset_from_unsigned(self, origin: *const T) -> usize;
}

impl<T> PtrExt<T> for *const T {
    unsafe fn offset_from_unsigned(self, origin: *const T) -> usize {
        unsafe { usize::try_from(self.offset_from(origin)).unwrap_unchecked() }
    }
}

impl<T> PtrExt<T> for *mut T {
    unsafe fn offset_from_unsigned(self, origin: *const T) -> usize {
        unsafe { usize::try_from(self.offset_from(origin)).unwrap_unchecked() }
    }
}
