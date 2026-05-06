use ckb_std::ckb_constants::Source;

use crate::{error::Error, meta};

pub fn main() -> Result<(), Error> {
    let meta_type_hash = meta::load_meta_type_hash_arg()?;
    let input_amount = meta::collect_group_amount(Source::GroupInput)?;
    let output_amount = meta::collect_group_amount(Source::GroupOutput)?;

    if input_amount == output_amount {
        return Ok(());
    }

    if output_amount > input_amount {
        let delta = output_amount
            .checked_sub(input_amount)
            .ok_or(Error::AmountOverflow)?;
        return meta::validate_mint(&meta_type_hash, delta);
    }

    let delta = input_amount
        .checked_sub(output_amount)
        .ok_or(Error::AmountOverflow)?;
    meta::validate_burn_or_destruction(&meta_type_hash, delta)
}
