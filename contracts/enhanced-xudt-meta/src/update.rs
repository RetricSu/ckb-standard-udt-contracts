use crate::{
    error::Error,
    meta_cell::{
        CONFIG_SUPPLY_TRACKED, ScriptAttr, XudtMeta, access_enabled,
        has_full_domain_access_list_shards, has_legal_access_list_shard, has_same_token_cells,
        is_supply_tracked, paused, whitelist_mode,
    },
};
use ckb_std::{
    ckb_constants::Source,
    error::SysError,
    high_level::{load_cell_lock_hash, load_cell_type_hash},
};

pub fn validate_update(
    input: &XudtMeta,
    output: &XudtMeta,
    meta_type_hash: &[u8; 32],
) -> Result<(), Error> {
    if input.config_flags & CONFIG_SUPPLY_TRACKED != output.config_flags & CONFIG_SUPPLY_TRACKED {
        return Err(Error::ImmutableSupplyMode);
    }

    if !is_supply_tracked(output.config_flags) && output.current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    let access_state_changed = access_enabled(input.config_flags)
        != access_enabled(output.config_flags)
        || whitelist_mode(input.config_flags) != whitelist_mode(output.config_flags)
        || paused(input.config_flags) != paused(output.config_flags)
        || input.access_authority_raw != output.access_authority_raw;
    if access_state_changed {
        require_authority(input.access_authority.as_ref())?;
    } else if input.access_authority.is_none() && output.access_authority.is_some() {
        return Err(Error::AuthorityMissing);
    }

    if input.extensions_raw != output.extensions_raw {
        require_authority(input.mint_authority.as_ref())?;
    }

    if input.metadata_fields != output.metadata_fields
        || input.metadata_authority_raw != output.metadata_authority_raw
    {
        require_authority(input.metadata_authority.as_ref())?;
    } else if input.metadata_authority.is_none() && output.metadata_authority.is_some() {
        return Err(Error::AuthorityMissing);
    }

    if input.current_supply != output.current_supply
        || input.mint_authority_raw != output.mint_authority_raw
    {
        require_authority(input.mint_authority.as_ref())?;
    } else if input.mint_authority.is_none() && output.mint_authority.is_some() {
        return Err(Error::AuthorityMissing);
    }

    validate_access_mode_transition(input.config_flags, output.config_flags, meta_type_hash)?;

    Ok(())
}

fn validate_access_mode_transition(
    input_flags: u8,
    output_flags: u8,
    meta_type_hash: &[u8; 32],
) -> Result<(), Error> {
    let input_enabled = access_enabled(input_flags);
    let output_enabled = access_enabled(output_flags);
    let input_whitelist = whitelist_mode(input_flags);
    let output_whitelist = whitelist_mode(output_flags);

    if input_enabled == output_enabled && input_whitelist == output_whitelist {
        return Ok(());
    }

    if has_same_token_cells(meta_type_hash)? {
        return Err(Error::AccessModeTokenCells);
    }

    match (
        input_enabled,
        input_whitelist,
        output_enabled,
        output_whitelist,
    ) {
        (false, false, true, false) | (true, true, true, false) => {
            if !has_full_domain_access_list_shards(meta_type_hash)? {
                return Err(Error::AccessListRequired);
            }
        }
        (false, false, true, true) | (true, false, true, true) => {
            if !has_legal_access_list_shard(meta_type_hash)? {
                return Err(Error::AccessListRequired);
            }
        }
        _ => {}
    }

    Ok(())
}

fn require_authority(authority: Option<&ScriptAttr>) -> Result<(), Error> {
    let authority = authority.ok_or(Error::AuthorityMissing)?;
    match check_authority(authority) {
        Ok(true) => Ok(()),
        Ok(false) => Err(Error::AuthorityFailed),
        Err(error) => Err(error),
    }
}

fn check_authority(authority: &ScriptAttr) -> Result<bool, Error> {
    match authority.location {
        0 => has_input_lock_hash(&authority.script_hash),
        1 => has_type_hash(&authority.script_hash, Source::Input),
        2 => has_type_hash(&authority.script_hash, Source::Output),
        3 | 4 => Err(Error::AuthorityFailed),
        _ => Err(Error::InvalidMetaData),
    }
}

fn has_input_lock_hash(target: &[u8; 32]) -> Result<bool, Error> {
    let mut index = 0;
    loop {
        match load_cell_lock_hash(index, Source::Input) {
            Ok(candidate) if &candidate == target => return Ok(true),
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(false),
            Err(error) => return Err(error.into()),
        }
    }
}

fn has_type_hash(target: &[u8; 32], source: Source) -> Result<bool, Error> {
    let mut index = 0;
    loop {
        match load_cell_type_hash(index, source) {
            Ok(Some(candidate)) if &candidate == target => return Ok(true),
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(false),
            Err(error) => return Err(error.into()),
        }
    }
}
