#![cfg_attr(not(any(feature = "library", test)), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(any(feature = "library", test))]
extern crate alloc;

mod error;
mod meta;

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
    let meta_type_hash = meta::load_meta_type_hash_arg()?;
    let input_amount = meta::collect_group_amount(ckb_std::ckb_constants::Source::GroupInput)?;
    let output_amount = meta::collect_group_amount(ckb_std::ckb_constants::Source::GroupOutput)?;

    if input_amount == output_amount {
        return Ok(());
    }

    if output_amount > input_amount {
        let delta = output_amount
            .checked_sub(input_amount)
            .ok_or(error::Error::AmountOverflow)?;
        return meta::validate_mint(&meta_type_hash, delta);
    }

    let delta = input_amount
        .checked_sub(output_amount)
        .ok_or(error::Error::AmountOverflow)?;
    meta::validate_burn_or_destruction(&meta_type_hash, delta)
}
