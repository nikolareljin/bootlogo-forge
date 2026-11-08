#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "std")]
pub mod detect;

#[cfg(feature = "std")]
pub use detect::*;

#[cfg(feature = "image")]
pub mod image;

#[cfg(feature = "image")]
pub use image::*;

pub mod bgrt;

pub use bgrt::*;
