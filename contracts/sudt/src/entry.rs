use ckb_std::ckb_constants::Source;

use crate::{error::Error, meta};

pub fn main() -> Result<(), Error> {
    let meta_type_hash = meta::load_meta_type_hash_arg()?;
    let input_amount = meta::collect_group_amount(Source::GroupInput)?;
    let output_amount = meta::collect_group_amount(Source::GroupOutput)?;

    if output_amount > input_amount {
        return meta::validate_mint(&meta_type_hash);
    }

    Ok(())
}
