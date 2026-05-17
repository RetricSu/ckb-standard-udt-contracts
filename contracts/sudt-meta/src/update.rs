use crate::{
    constants::SUDT_CODE_HASH,
    error::Error,
    state::{has_bound_sudt_outputs, is_supply_tracked},
};
use standard_udt_script_utils::{
    authority::AuthorityVerifier, error::ScriptError, supply::apply_supply_delta,
    token::transaction_token_delta,
};
use standard_udt_types::metadata::SudtMeta;

pub fn validate_update(
    input: &SudtMeta,
    output: &SudtMeta,
    meta_type_hash: &[u8; 32],
) -> Result<(), Error> {
    if supply_mode_changed(input.config_flags, output.config_flags) {
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

    let mut verifier = AuthorityVerifier::new();
    verifier
        .require_with_fallback(
            input.metadata_authority.as_ref(),
            input.mint_authority.as_ref(),
        )
        .map_err(map_script_error)?;

    let supply_or_mint_authority_changed = input.current_supply != output.current_supply
        || input.mint_authority != output.mint_authority;
    if supply_or_mint_authority_changed {
        verifier
            .require(input.mint_authority.as_ref())
            .map_err(map_script_error)?;
    }

    Ok(())
}

pub fn validate_destroy(input: &SudtMeta, meta_type_hash: &[u8; 32]) -> Result<(), Error> {
    if !is_supply_tracked(input.config_flags) || input.current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    if has_bound_sudt_outputs(meta_type_hash)? {
        return Err(Error::InvalidSupply);
    }

    let mut verifier = AuthorityVerifier::new();
    verifier
        .require(input.mint_authority.as_ref())
        .map_err(map_script_error)
}

fn map_script_error(error: ScriptError) -> Error {
    match error {
        ScriptError::AuthorityMissing => Error::AuthorityMissing,
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

fn supply_mode_changed(input_flags: u8, output_flags: u8) -> bool {
    is_supply_tracked(input_flags) != is_supply_tracked(output_flags)
}
