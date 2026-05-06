use ckb_std::{
    ckb_constants::Source,
    ckb_types::{core::ScriptHashType, packed::Script, prelude::*},
    error::SysError,
    high_level::{load_cell_data, load_cell_type},
};

use crate::{
    constants::XUDT_CODE_HASH,
    error::Error,
    meta_cell::{config, parser::ParsedXudtMeta},
};

pub fn validate_create(
    output_meta: &ParsedXudtMeta,
    meta_type_hash: &[u8; 32],
) -> Result<(), Error> {
    if config::is_supply_tracked(output_meta.config_flags) {
        let initial_supply = sum_initial_udt_outputs(meta_type_hash, &XUDT_CODE_HASH)?;
        if output_meta.current_supply != initial_supply {
            return Err(Error::InvalidSupply);
        }
    } else if output_meta.current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    Ok(())
}

pub fn sum_initial_udt_outputs(
    meta_type_hash: &[u8; 32],
    udt_code_hash: &[u8; 32],
) -> Result<u128, Error> {
    let mut total = 0u128;
    let mut index = 0;

    loop {
        let type_script = match load_cell_type(index, Source::Output) {
            Ok(Some(script)) => script,
            Ok(None) => {
                index += 1;
                continue;
            }
            Err(SysError::IndexOutOfBound) => return Ok(total),
            Err(error) => return Err(error.into()),
        };

        if is_token_script(&type_script, meta_type_hash, udt_code_hash) {
            let data = load_cell_data(index, Source::Output)?;
            let amount = decode_amount(&data)?;
            total = total.checked_add(amount).ok_or(Error::InvalidSupply)?;
        }

        index += 1;
    }
}

pub fn has_same_token_cells(meta_type_hash: &[u8; 32]) -> Result<bool, Error> {
    for source in [Source::Input, Source::Output] {
        let mut index = 0;
        loop {
            match load_cell_type(index, source) {
                Ok(Some(script)) if is_token_script(&script, meta_type_hash, &XUDT_CODE_HASH) => {
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

pub(crate) fn is_token_script(
    type_script: &Script,
    meta_type_hash: &[u8; 32],
    code_hash: &[u8; 32],
) -> bool {
    if type_script.hash_type() != ScriptHashType::Data2.into() {
        return false;
    }
    if type_script.args().raw_data().as_ref() != meta_type_hash {
        return false;
    }

    let actual_code_hash: [u8; 32] = type_script.code_hash().unpack();
    &actual_code_hash == code_hash
}

fn decode_amount(data: &[u8]) -> Result<u128, Error> {
    if data.len() < 16 {
        return Err(Error::InvalidSupply);
    }

    let mut raw = [0u8; 16];
    raw.copy_from_slice(&data[..16]);
    Ok(u128::from_le_bytes(raw))
}
