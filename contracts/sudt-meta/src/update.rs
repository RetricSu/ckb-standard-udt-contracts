use crate::{
    error::Error,
    meta_cell::{CONFIG_SUPPLY_TRACKED, is_supply_tracked},
};
use standard_udt_script_utils::{
    authority::check_authority as check_runtime_authority, error::ScriptError,
};
use standard_udt_types::metadata::Authority;
use standard_udt_types::metadata::SudtMeta;

pub fn validate_update(input: &SudtMeta, output: &SudtMeta) -> Result<(), Error> {
    if input.config_flags & CONFIG_SUPPLY_TRACKED != output.config_flags & CONFIG_SUPPLY_TRACKED {
        return Err(Error::ImmutableSupplyMode);
    }

    if !is_supply_tracked(output.config_flags) && output.current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    if input.current_supply != output.current_supply
        || input.mint_authority != output.mint_authority
    {
        require_authority(input.mint_authority.as_ref())?;
    }

    if input.decimals != output.decimals
        || input.name != output.name
        || input.symbol != output.symbol
        || input.uri != output.uri
        || input.extra_data != output.extra_data
        || input.metadata_authority != output.metadata_authority
    {
        require_authority_with_mint_fallback(
            input.metadata_authority.as_ref(),
            input.mint_authority.as_ref(),
        )?;
    }

    Ok(())
}

fn require_authority(authority: Option<&Authority>) -> Result<(), Error> {
    let authority = authority.ok_or(Error::AuthorityMissing)?;
    match check_authority(authority) {
        Ok(true) => Ok(()),
        Ok(false) => Err(Error::AuthorityFailed),
        Err(error) => Err(error),
    }
}

fn require_authority_with_mint_fallback(
    authority: Option<&Authority>,
    mint_authority: Option<&Authority>,
) -> Result<(), Error> {
    match authority {
        Some(authority) if check_authority(authority)? => return Ok(()),
        Some(_) if mint_authority.is_none() => return Err(Error::AuthorityFailed),
        Some(_) | None => {}
    }
    require_authority(mint_authority)
}

fn check_authority(authority: &Authority) -> Result<bool, Error> {
    check_runtime_authority(authority).map_err(map_script_error)
}

fn map_script_error(error: ScriptError) -> Error {
    match error {
        ScriptError::AuthorityFailed | ScriptError::UnsupportedAuthorityLocation => {
            Error::AuthorityFailed
        }
        ScriptError::InvalidAuthority => Error::InvalidMetaData,
        ScriptError::SyscallUnknown => Error::SyscallUnknown,
        _ => Error::SyscallUnknown,
    }
}
