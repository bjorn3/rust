//! This crate defines various extension traits and macros to backport newer
//! standard library features to older rustc versions to allow bootstrapping
//! rustc from older rustc versions.

#![feature(decl_macro)]

pub mod prelude {
    pub use crate::PtrExt as _;
}

// Copied from library/core/src/macros.rs of rustc 1.88.0
pub macro cfg_match {
    ({ $($tt:tt)* }) => {{
        $crate::cfg_match! { $($tt)* }
    }},
    (_ => { $($output:tt)* }) => {
        $($output)*
    },
    (
        $cfg:meta => $output:tt
        $($( $rest:tt )+)?
    ) => {
        #[cfg($cfg)]
        $crate::cfg_match! { _ => $output }
        $(
            #[cfg(not($cfg))]
            $crate::cfg_match! { $($rest)+ }
        )?
    },
}

pub trait PointeeSized {}
impl<T: ?Sized> PointeeSized for T {}

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
