#![cfg_attr(not(feature = "library"), no_std)]
#![allow(special_module_name)]
#![allow(unused_attributes)]
#[cfg(feature = "library")]
mod error;
#[cfg(feature = "library")]
mod main;
#[cfg(feature = "library")]
mod meta;
#[cfg(feature = "library")]
mod mode;
#[cfg(feature = "library")]
mod run;
#[cfg(feature = "library")]
mod shards;
#[cfg(feature = "library")]
pub use main::program_entry;

extern crate alloc;
