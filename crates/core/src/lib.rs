#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub mod detect;

#[cfg(feature = "std")]
pub use detect::*;
