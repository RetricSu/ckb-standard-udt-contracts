#![cfg_attr(not(feature = "library"), no_std)]
#![allow(special_module_name)]
#![allow(unused_attributes)]
extern crate alloc;

#[cfg(feature = "library")]
mod constants;
#[cfg(feature = "library")]
mod entry;
#[cfg(feature = "library")]
mod error;
#[cfg(feature = "library")]
mod meta_cell;
#[cfg(feature = "library")]
mod update;

#[cfg(feature = "library")]
pub fn program_entry() -> i8 {
    match entry::main() {
        Ok(()) => 0,
        Err(error) => error.into(),
    }
}
