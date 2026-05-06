use crate::{
    error::Error,
    meta_cell::{CONFIG_SUPPLY_TRACKED, ParsedAuthority, ParsedSudtMeta, is_supply_tracked},
};
use standard_udt_script_utils::{
    authority::{ParsedAuthority as RuntimeAuthority, check_authority as check_runtime_authority},
    error::ScriptError,
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

fn require_authority(authority: Option<&ParsedAuthority>) -> Result<(), Error> {
    let authority = authority.ok_or(Error::AuthorityMissing)?;
    match check_authority(authority) {
        Ok(true) => Ok(()),
        Ok(false) => Err(Error::AuthorityFailed),
        Err(error) => Err(error),
    }
}

fn check_authority(authority: &ParsedAuthority) -> Result<bool, Error> {
    check_runtime_authority(&RuntimeAuthority {
        authority_type: authority.authority_type,
        script_hash: authority.script_hash,
        script: authority.script.clone(),
    })
    .map_err(map_script_error)
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
