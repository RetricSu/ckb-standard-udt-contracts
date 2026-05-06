use ckb_std::ckb_constants::Source;

use crate::{
    error::Error,
    meta::{
        authority::require_authority,
        cells::{find_meta_in_source, find_unique_visible_meta},
        parser::is_supply_tracked,
    },
};

pub fn validate_mint(meta_type_hash: &[u8; 32], delta: u128) -> Result<(), Error> {
    let Some(visible_meta) = find_unique_visible_meta(meta_type_hash)? else {
        return validate_initial_create_mint(meta_type_hash, delta);
    };
    require_authority(visible_meta.mint_authority.as_ref())?;

    if is_supply_tracked(visible_meta.config_flags) {
        let input_meta =
            find_meta_in_source(meta_type_hash, Source::Input)?.ok_or(Error::MetaInputMissing)?;
        let output_meta =
            find_meta_in_source(meta_type_hash, Source::Output)?.ok_or(Error::MetaOutputMissing)?;
        let expected = input_meta
            .current_supply
            .checked_add(delta)
            .ok_or(Error::SupplyOverflow)?;
        if output_meta.current_supply != expected
            || output_meta.config_flags != input_meta.config_flags
        {
            return Err(Error::MetaStateMismatch);
        }
    } else if visible_meta.current_supply != 0 {
        return Err(Error::MetaStateMismatch);
    }

    Ok(())
}

fn validate_initial_create_mint(meta_type_hash: &[u8; 32], _delta: u128) -> Result<(), Error> {
    if find_meta_in_source(meta_type_hash, Source::Input)?.is_some() {
        return Err(Error::MetaNotUnique);
    }

    let _output_meta =
        find_meta_in_source(meta_type_hash, Source::Output)?.ok_or(Error::MetaMissing)?;

    Ok(())
}

pub fn validate_burn_or_destruction(meta_type_hash: &[u8; 32], delta: u128) -> Result<(), Error> {
    let Some(input_meta) = find_meta_in_source(meta_type_hash, Source::Input)? else {
        return Ok(());
    };

    require_authority(input_meta.mint_authority.as_ref())?;

    if is_supply_tracked(input_meta.config_flags) {
        let output_meta =
            find_meta_in_source(meta_type_hash, Source::Output)?.ok_or(Error::MetaOutputMissing)?;
        let expected = input_meta
            .current_supply
            .checked_sub(delta)
            .ok_or(Error::SupplyUnderflow)?;
        if output_meta.current_supply != expected
            || output_meta.config_flags != input_meta.config_flags
        {
            return Err(Error::MetaStateMismatch);
        }
    } else if input_meta.current_supply != 0 {
        return Err(Error::MetaStateMismatch);
    }

    Ok(())
}
