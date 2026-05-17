#![cfg_attr(not(feature = "library"), no_std)]
#![allow(special_module_name)]
#![allow(unused_attributes)]

extern crate alloc;

#[cfg(feature = "library")]
pub mod access;
#[cfg(feature = "library")]
pub mod config;
#[cfg(feature = "library")]
pub mod entry;
#[cfg(feature = "library")]
pub mod error;
#[cfg(feature = "library")]
pub mod extensions;
#[cfg(feature = "library")]
pub mod meta;
#[cfg(feature = "library")]
pub mod validation;

#[cfg(feature = "library")]
pub fn program_entry() -> i8 {
    match entry::main() {
        Ok(()) => 0,
        Err(error) => error.into(),
    }
}
