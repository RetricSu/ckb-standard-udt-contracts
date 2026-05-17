use ckb_std::ckb_constants::Source;

use crate::{
    error::Error,
    meta::{
        authority::require_authority,
        cells::{find_current_meta, find_meta_in_source},
        parser::is_supply_tracked,
    },
};

pub fn validate_mint(meta_type_hash: &[u8; 32]) -> Result<(), Error> {
    let Some(current_meta) = find_current_meta(meta_type_hash)? else {
        return require_initial_mint_output_meta(meta_type_hash);
    };

    if is_supply_tracked(current_meta.meta.config_flags) {
        if current_meta.source != Source::Input {
            return Err(Error::MetaInputMissing);
        }
    } else {
        require_authority(current_meta.meta.mint_authority.as_ref())?;
        if current_meta.meta.current_supply != 0 {
            return Err(Error::MetaStateMismatch);
        }
    }

    Ok(())
}

fn require_initial_mint_output_meta(meta_type_hash: &[u8; 32]) -> Result<(), Error> {
    let _output_meta =
        find_meta_in_source(meta_type_hash, Source::Output)?.ok_or(Error::MetaMissing)?;

    Ok(())
}

pub fn validate_burn_or_destruction(meta_type_hash: &[u8; 32]) -> Result<(), Error> {
    let Some(input_meta) = find_meta_in_source(meta_type_hash, Source::Input)? else {
        return Ok(());
    };

    if !is_supply_tracked(input_meta.config_flags) && input_meta.current_supply != 0 {
        return Err(Error::MetaStateMismatch);
    }

    Ok(())
}
