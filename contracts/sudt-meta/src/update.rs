use crate::{
    error::Error,
    meta_cell::{CONFIG_SUPPLY_TRACKED, ParsedSudtMeta, ScriptAttr, is_supply_tracked},
};
use ckb_std::{
    ckb_constants::Source,
    error::SysError,
    high_level::{load_cell_lock_hash, load_cell_type_hash},
};

pub fn validate_update(input: &ParsedSudtMeta, output: &ParsedSudtMeta) -> Result<(), Error> {
    if input.config_flags & CONFIG_SUPPLY_TRACKED != output.config_flags & CONFIG_SUPPLY_TRACKED {
        return Err(Error::ImmutableSupplyMode);
    }

    if !is_supply_tracked(output.config_flags) && output.current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    if input.current_supply != output.current_supply {
        require_authority(input.mint_authority.as_ref())?;
    }

    if input.mint_authority_raw != output.mint_authority_raw {
        require_authority(input.mint_authority.as_ref())?;
    } else if input.mint_authority.is_none() && output.mint_authority.is_some() {
        return Err(Error::AuthorityMissing);
    }

    if input.metadata_fields != output.metadata_fields
        || input.metadata_authority_raw != output.metadata_authority_raw
    {
        require_authority(input.metadata_authority.as_ref())?;
    } else if input.metadata_authority.is_none() && output.metadata_authority.is_some() {
        return Err(Error::AuthorityMissing);
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
