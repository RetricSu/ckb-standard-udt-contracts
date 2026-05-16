use crate::{
    constants::SUDT_CODE_HASH,
    error::Error,
    state::{CONFIG_SUPPLY_TRACKED, is_supply_tracked},
};
use standard_udt_script_utils::{
    authority::check_authority as check_runtime_authority, error::ScriptError,
    supply::apply_supply_delta, token::transaction_token_delta,
};
use standard_udt_types::metadata::Authority;
use standard_udt_types::metadata::SudtMeta;

pub fn validate_update(
    input: &SudtMeta,
    output: &SudtMeta,
    meta_type_hash: &[u8; 32],
) -> Result<(), Error> {
    if input.config_flags & CONFIG_SUPPLY_TRACKED != output.config_flags & CONFIG_SUPPLY_TRACKED {
        return Err(Error::ImmutableSupplyMode);
    }

    if !is_supply_tracked(output.config_flags) && output.current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    if is_supply_tracked(output.config_flags) {
        let delta =
            transaction_token_delta(meta_type_hash, &SUDT_CODE_HASH).map_err(map_token_error)?;
        let expected_supply =
            apply_supply_delta(input.current_supply, delta).map_err(map_token_error)?;
        if output.current_supply != expected_supply {
            return Err(Error::InvalidSupply);
        }
    }

    let supply_or_mint_authority_changed = input.current_supply != output.current_supply
        || input.mint_authority != output.mint_authority;
    if supply_or_mint_authority_changed {
        require_authority(input.mint_authority.as_ref())?;
    }

    let metadata_changed = input.decimals != output.decimals
        || input.name != output.name
        || input.symbol != output.symbol
        || input.uri != output.uri
        || input.extra_data != output.extra_data
        || input.metadata_authority != output.metadata_authority;
    if metadata_changed {
        require_authority_with_mint_fallback(
            input.metadata_authority.as_ref(),
            input.mint_authority.as_ref(),
        )?;
    }

    if !supply_or_mint_authority_changed && !metadata_changed {
        require_authority_with_mint_fallback(
            input.metadata_authority.as_ref(),
            input.mint_authority.as_ref(),
        )?;
    }

    Ok(())
}

pub fn validate_destroy(input: &SudtMeta) -> Result<(), Error> {
    if !is_supply_tracked(input.config_flags) || input.current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    require_authority(input.mint_authority.as_ref())
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
    if let Some(authority) = authority {
        match check_authority(authority) {
            Ok(true) => return Ok(()),
            Ok(false) | Err(Error::AuthorityFailed) => {
                if mint_authority.is_none() {
                    return Err(Error::AuthorityFailed);
                }
            }
            Err(error) => return Err(error),
        }
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

fn map_token_error(error: ScriptError) -> Error {
    match error {
        ScriptError::AmountEncoding
        | ScriptError::AmountOverflow
        | ScriptError::SupplyOverflow
        | ScriptError::SupplyUnderflow => Error::InvalidSupply,
        ScriptError::SyscallUnknown => Error::SyscallUnknown,
        _ => Error::SyscallUnknown,
    }
}
