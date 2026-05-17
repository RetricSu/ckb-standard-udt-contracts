use ckb_std::{ckb_constants::Source, error::SysError, high_level::load_cell_type};
pub(crate) use standard_udt_script_utils::token::matches_bound_type_script;
use standard_udt_script_utils::{error::ScriptError, token::sum_token_amount};

use crate::{constants::XUDT_CODE_HASH, error::Error};
use standard_udt_types::metadata::{XudtMeta, access_enabled, is_supply_tracked};

use super::access_list::has_full_domain_access_list_outputs;

pub fn validate_create(output_meta: &XudtMeta, meta_type_hash: &[u8; 32]) -> Result<(), Error> {
    if is_supply_tracked(output_meta.config_flags) {
        let initial_supply = sum_token_amount(Source::Output, meta_type_hash, &XUDT_CODE_HASH)
            .map_err(map_supply_error)?;
        if output_meta.current_supply != initial_supply {
            return Err(Error::InvalidSupply);
        }
    } else if output_meta.current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    if access_enabled(output_meta.config_flags)
        && !has_full_domain_access_list_outputs(meta_type_hash)?
    {
        return Err(Error::AccessListRequired);
    }

    Ok(())
}

pub fn has_bound_xudt_cells(meta_type_hash: &[u8; 32]) -> Result<bool, Error> {
    for source in [Source::Input, Source::Output] {
        let mut index = 0;
        loop {
            match load_cell_type(index, source) {
                Ok(Some(script))
                    if matches_bound_type_script(&script, meta_type_hash, &XUDT_CODE_HASH) =>
                {
                    return Ok(true);
                }
                Ok(_) => index += 1,
                Err(SysError::IndexOutOfBound) => break,
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(false)
}

pub fn has_bound_xudt_outputs(meta_type_hash: &[u8; 32]) -> Result<bool, Error> {
    let mut index = 0;
    loop {
        match load_cell_type(index, Source::Output) {
            Ok(Some(script))
                if matches_bound_type_script(&script, meta_type_hash, &XUDT_CODE_HASH) =>
            {
                return Ok(true);
            }
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(false),
            Err(error) => return Err(error.into()),
        }
    }
}

fn map_supply_error(error: ScriptError) -> Error {
    match error {
        ScriptError::AmountEncoding | ScriptError::AmountOverflow => Error::InvalidSupply,
        ScriptError::SyscallUnknown => Error::SyscallUnknown,
        _ => Error::SyscallUnknown,
    }
}
