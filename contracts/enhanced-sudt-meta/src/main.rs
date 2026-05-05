#![cfg_attr(not(any(feature = "library", test)), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(any(feature = "library", test))]
extern crate alloc;

mod constants;
mod error;
mod meta_cell;
mod update;

#[cfg(not(any(feature = "library", test)))]
ckb_std::entry!(program_entry);
#[cfg(not(any(feature = "library", test)))]
// By default, the following heap configuration is used:
// * 16KB fixed heap
// * 1.2MB(rounded up to be 16-byte aligned) dynamic heap
// * Minimal memory block in dynamic heap is 64 bytes
// For more details, please refer to ckb-std's default_alloc macro
// and the buddy-alloc alloc implementation.
ckb_std::default_alloc!(16384, 1258306, 64);

pub fn program_entry() -> i8 {
    match run() {
        Ok(()) => 0,
        Err(error) => error.into(),
    }
}

fn run() -> Result<(), error::Error> {
    meta_cell::validate_type_args()?;
    let group = meta_cell::load_meta_group()?;

    match (group.input.as_ref(), group.output.as_ref()) {
        (None, Some(output)) => {
            meta_cell::validate_create_type_id()?;
            meta_cell::validate_create(output, &group.meta_type_hash)
        }
        (Some(input), Some(output)) => update::validate_update(input, output),
        _ => Err(error::Error::InvalidArgs),
    }
}
