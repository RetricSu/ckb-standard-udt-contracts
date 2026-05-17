use ckb_std::ckb_constants::Source;

use crate::{
    error::Error,
    meta,
    validation::{
        require_initial_mint_output_meta, validate_mint, validate_negative_delta, validate_transfer,
    },
};

pub fn main() -> Result<(), Error> {
    let meta_type_hash = meta::load_meta_type_hash_arg()?;
    let input_amount = meta::collect_group_amount(Source::GroupInput)?;
    let output_amount = meta::collect_group_amount(Source::GroupOutput)?;

    if input_amount == output_amount {
        let current_meta = meta::find_current_meta(&meta_type_hash)?.ok_or(Error::MetaMissing)?;
        validate_transfer(&meta_type_hash, &current_meta.meta)
    } else if output_amount > input_amount {
        match meta::find_current_meta(&meta_type_hash)? {
            Some(current_meta) => validate_mint(&meta_type_hash, &current_meta),
            None => require_initial_mint_output_meta(&meta_type_hash),
        }
    } else {
        validate_negative_delta(&meta_type_hash, output_amount)
    }
}
