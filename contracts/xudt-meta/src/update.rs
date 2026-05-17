use crate::{
    constants::XUDT_CODE_HASH,
    error::Error,
    state::{
        access_enabled, has_bound_access_list_outputs, has_bound_xudt_cells,
        has_bound_xudt_outputs, has_full_domain_access_list_inputs,
        has_full_domain_access_list_outputs, is_supply_tracked, paused, whitelist_mode,
    },
};
use standard_udt_script_utils::{
    authority::AuthorityVerifier, error::ScriptError, supply::apply_supply_delta,
    token::transaction_token_delta,
};
use standard_udt_types::metadata::XudtMeta;

pub fn validate_update(
    input: &XudtMeta,
    output: &XudtMeta,
    meta_type_hash: &[u8; 32],
) -> Result<(), Error> {
    if supply_mode_changed(input.config_flags, output.config_flags) {
        return Err(Error::ImmutableSupplyMode);
    }

    if !is_supply_tracked(output.config_flags) && output.current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    if is_supply_tracked(output.config_flags) {
        validate_supply_delta(input.current_supply, output.current_supply, meta_type_hash)?;
    }

    let mut verifier = AuthorityVerifier::new();
    verifier
        .require_any(&[
            input.metadata_authority.as_ref(),
            input.access_authority.as_ref(),
            input.mint_authority.as_ref(),
        ])
        .map_err(map_script_error)?;

    let access_state_changed = access_enabled(input.config_flags)
        != access_enabled(output.config_flags)
        || whitelist_mode(input.config_flags) != whitelist_mode(output.config_flags)
        || paused(input.config_flags) != paused(output.config_flags)
        || input.access_authority != output.access_authority;
    if access_state_changed {
        verifier
            .require_with_fallback(
                input.access_authority.as_ref(),
                input.mint_authority.as_ref(),
            )
            .map_err(map_script_error)?;
    }

    if input.extensions != output.extensions {
        verifier
            .require(input.mint_authority.as_ref())
            .map_err(map_script_error)?;
    }

    let metadata_changed = input.decimals != output.decimals
        || input.name != output.name
        || input.symbol != output.symbol
        || input.uri != output.uri
        || input.extra_data != output.extra_data
        || input.metadata_authority != output.metadata_authority;
    if metadata_changed {
        verifier
            .require_with_fallback(
                input.metadata_authority.as_ref(),
                input.mint_authority.as_ref(),
            )
            .map_err(map_script_error)?;
    }

    let supply_or_mint_authority_changed = input.current_supply != output.current_supply
        || input.mint_authority != output.mint_authority;
    if supply_or_mint_authority_changed {
        verifier
            .require(input.mint_authority.as_ref())
            .map_err(map_script_error)?;
    }

    validate_access_mode_transition(input.config_flags, output.config_flags, meta_type_hash)?;

    Ok(())
}

pub fn validate_destroy(input: &XudtMeta, meta_type_hash: &[u8; 32]) -> Result<(), Error> {
    if !is_supply_tracked(input.config_flags) || input.current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    if has_bound_xudt_outputs(meta_type_hash)? {
        return Err(Error::InvalidSupply);
    }

    if access_enabled(input.config_flags) {
        if !has_full_domain_access_list_inputs(meta_type_hash)? {
            return Err(Error::AccessListRequired);
        }

        if has_bound_access_list_outputs(meta_type_hash)? {
            return Err(Error::AccessListRequired);
        }
    }

    let mut verifier = AuthorityVerifier::new();
    verifier
        .require(input.mint_authority.as_ref())
        .map_err(map_script_error)
}

fn validate_supply_delta(
    input_supply: u128,
    output_supply: u128,
    meta_type_hash: &[u8; 32],
) -> Result<(), Error> {
    let delta =
        transaction_token_delta(meta_type_hash, &XUDT_CODE_HASH).map_err(map_supply_error)?;
    let expected = apply_supply_delta(input_supply, delta).map_err(map_supply_error)?;
    if output_supply == expected {
        Ok(())
    } else {
        Err(Error::InvalidSupply)
    }
}

fn map_supply_error(error: ScriptError) -> Error {
    match error {
        ScriptError::AmountEncoding
        | ScriptError::AmountOverflow
        | ScriptError::SupplyOverflow
        | ScriptError::SupplyUnderflow => Error::InvalidSupply,
        ScriptError::SyscallUnknown => Error::SyscallUnknown,
        _ => Error::SyscallUnknown,
    }
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

    if has_bound_xudt_cells(meta_type_hash)? {
        return Err(Error::AccessModeTokenCells);
    }

    match (
        input_enabled,
        input_whitelist,
        output_enabled,
        output_whitelist,
    ) {
        (false, false, true, false) | (false, false, true, true) => {
            if !has_full_domain_access_list_outputs(meta_type_hash)? {
                return Err(Error::AccessListRequired);
            }
        }
        (true, false, false, false) | (true, true, false, false) => {
            if !has_full_domain_access_list_inputs(meta_type_hash)? {
                return Err(Error::AccessListRequired);
            }
        }
        (true, false, true, true) | (true, true, true, false) => {
            if !has_full_domain_access_list_inputs(meta_type_hash)?
                || !has_full_domain_access_list_outputs(meta_type_hash)?
            {
                return Err(Error::AccessListRequired);
            }
        }
        _ => {}
    }

    Ok(())
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

fn supply_mode_changed(input_flags: u8, output_flags: u8) -> bool {
    is_supply_tracked(input_flags) != is_supply_tracked(output_flags)
}
