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
        return require_initial_mint_output_meta(meta_type_hash);
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
            || supply_mode_changed(input_meta.config_flags, output_meta.config_flags)
        {
            return Err(Error::MetaStateMismatch);
        }
    } else if visible_meta.current_supply != 0 {
        return Err(Error::MetaStateMismatch);
    }

    Ok(())
}

fn require_initial_mint_output_meta(meta_type_hash: &[u8; 32]) -> Result<(), Error> {
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
            || supply_mode_changed(input_meta.config_flags, output_meta.config_flags)
        {
            return Err(Error::MetaStateMismatch);
        }
    } else if input_meta.current_supply != 0 {
        return Err(Error::MetaStateMismatch);
    }

    Ok(())
}

fn supply_mode_changed(input_flags: u8, output_flags: u8) -> bool {
    is_supply_tracked(input_flags) != is_supply_tracked(output_flags)
}
