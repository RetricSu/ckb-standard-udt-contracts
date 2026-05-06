#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(all(feature = "std", not(feature = "no-std")))]
pub use ckb_types::molecule;
#[cfg(feature = "no-std")]
pub use molecule;

pub mod error;
pub mod generated;
pub mod metadata;
