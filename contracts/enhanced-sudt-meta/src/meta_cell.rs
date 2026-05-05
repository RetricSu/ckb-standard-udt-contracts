use alloc::vec::Vec;

use ckb_std::{
    ckb_constants::Source,
    ckb_types::{core::ScriptHashType, packed::Script, prelude::*},
    error::SysError,
    high_level::{load_cell_data, load_cell_type, load_script, load_script_hash},
    type_id::check_type_id,
};

use crate::{constants::ENHANCED_SUDT_CODE_HASH, error::Error};

pub const CONFIG_SUPPLY_TRACKED: u8 = 0b0000_0001;
const SUDT_META_FIELDS: usize = 9;
const SUDT_ALLOWED_CONFIG_MASK: u8 = CONFIG_SUPPLY_TRACKED;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub metadata_fields: Vec<u8>,
    pub mint_authority_raw: Vec<u8>,
    pub metadata_authority_raw: Vec<u8>,
    pub mint_authority: Option<ScriptAttr>,
    pub metadata_authority: Option<ScriptAttr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptAttr {
    pub location: u8,
    pub script_hash: [u8; 32],
}

pub struct MetaGroup {
    pub input: Option<SudtMeta>,
    pub output: Option<SudtMeta>,
    pub meta_type_hash: [u8; 32],
}

pub fn load_meta_group() -> Result<MetaGroup, Error> {
    Ok(MetaGroup {
        input: load_group_meta(Source::GroupInput)?,
        output: load_group_meta(Source::GroupOutput)?,
        meta_type_hash: load_script_hash().map_err(|_| Error::Syscall)?,
    })
}

pub fn validate_type_args() -> Result<(), Error> {
    let script = load_script().map_err(|_| Error::Syscall)?;
    if script.args().raw_data().len() != 32 {
        return Err(Error::InvalidArgs);
    }
    Ok(())
}

pub fn validate_create_type_id() -> Result<(), Error> {
    check_type_id(0, 32).map_err(|_| Error::InvalidTypeId)
}

pub fn validate_create(output_meta: &SudtMeta, meta_type_hash: &[u8; 32]) -> Result<(), Error> {
    if is_supply_tracked(output_meta.config_flags) {
        let initial_supply = sum_initial_udt_outputs(meta_type_hash, &ENHANCED_SUDT_CODE_HASH)?;
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
            Err(_) => return Err(Error::Syscall),
        };

        if is_initial_udt_script(&type_script, meta_type_hash, udt_code_hash) {
            let data = load_cell_data(index, Source::Output).map_err(|_| Error::Syscall)?;
            let amount = decode_amount(&data)?;
            total = total.checked_add(amount).ok_or(Error::InvalidSupply)?;
        }

        index += 1;
    }
}

fn decode_amount(data: &[u8]) -> Result<u128, Error> {
    if data.len() != 16 {
        return Err(Error::InvalidSupply);
    }

    let mut raw = [0u8; 16];
    raw.copy_from_slice(data);
    Ok(u128::from_le_bytes(raw))
}

fn load_group_meta(source: Source) -> Result<Option<SudtMeta>, Error> {
    let mut found = None;
    let mut index = 0;

    loop {
        match load_cell_data(index, source) {
            Ok(data) => {
                if found.is_some() {
                    return Err(Error::InvalidArgs);
                }
                found = Some(parse_meta(&data)?);
                index += 1;
            }
            Err(SysError::IndexOutOfBound) => return Ok(found),
            Err(_) => return Err(Error::Syscall),
        }
    }
}

fn parse_meta(data: &[u8]) -> Result<SudtMeta, Error> {
    let offsets = table_offsets(data, SUDT_META_FIELDS)?;
    let config_flags = single_byte_field(data, offsets[0], offsets[1])?;
    if config_flags & !SUDT_ALLOWED_CONFIG_MASK != 0 {
        return Err(Error::InvalidMetaData);
    }

    let current_supply = u128_field(data, offsets[1], offsets[2])?;
    let _decimals = single_byte_field(data, offsets[2], offsets[3])?;
    let metadata_fields = data[offsets[2]..offsets[7]].to_vec();
    let mint_authority_raw = data[offsets[7]..offsets[8]].to_vec();
    let metadata_authority_raw = data[offsets[8]..offsets[9]].to_vec();
    let mint_authority = parse_script_attr_opt(&mint_authority_raw)?;
    let metadata_authority = parse_script_attr_opt(&metadata_authority_raw)?;

    if !is_supply_tracked(config_flags) && current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    Ok(SudtMeta {
        config_flags,
        current_supply,
        metadata_fields,
        mint_authority_raw,
        metadata_authority_raw,
        mint_authority,
        metadata_authority,
    })
}

fn is_initial_udt_script(
    type_script: &ckb_std::ckb_types::packed::Script,
    meta_type_hash: &[u8; 32],
    udt_code_hash: &[u8; 32],
) -> bool {
    if type_script.hash_type() != ScriptHashType::Data2.into() {
        return false;
    }
    if type_script.args().raw_data().as_ref() != meta_type_hash {
        return false;
    }

    let code_hash: [u8; 32] = type_script.code_hash().unpack();
    &code_hash == udt_code_hash
}

pub fn is_supply_tracked(config_flags: u8) -> bool {
    config_flags & CONFIG_SUPPLY_TRACKED != 0
}

fn table_offsets(data: &[u8], fields: usize) -> Result<Vec<usize>, Error> {
    if data.len() < 4 + fields * 4 {
        return Err(Error::InvalidMetaData);
    }

    let total_size = read_u32(data, 0)? as usize;
    if total_size != data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut offsets = Vec::with_capacity(fields + 1);
    for index in 0..fields {
        offsets.push(read_u32(data, 4 + index * 4)? as usize);
    }
    offsets.push(total_size);

    let expected_header = 4 + fields * 4;
    if offsets[0] != expected_header {
        return Err(Error::InvalidMetaData);
    }
    for index in 1..offsets.len() {
        if offsets[index] < offsets[index - 1] || offsets[index] > total_size {
            return Err(Error::InvalidMetaData);
        }
    }

    Ok(offsets)
}

fn single_byte_field(data: &[u8], start: usize, end: usize) -> Result<u8, Error> {
    if end != start + 1 || end > data.len() {
        return Err(Error::InvalidMetaData);
    }
    Ok(data[start])
}

fn u128_field(data: &[u8], start: usize, end: usize) -> Result<u128, Error> {
    if end != start + 16 || end > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 16];
    raw.copy_from_slice(&data[start..end]);
    Ok(u128::from_le_bytes(raw))
}

fn parse_script_attr_opt(data: &[u8]) -> Result<Option<ScriptAttr>, Error> {
    if data.is_empty() {
        return Ok(None);
    }

    parse_script_attr(data).map(Some)
}

fn parse_script_attr(data: &[u8]) -> Result<ScriptAttr, Error> {
    let offsets = table_offsets(data, 3)?;
    let location = single_byte_field(data, offsets[0], offsets[1])?;
    let script_hash = byte32_field(data, offsets[1], offsets[2])?;
    let script_opt = &data[offsets[2]..offsets[3]];

    match location {
        0..=2 if script_opt.is_empty() => {}
        3 | 4 if !script_opt.is_empty() => {
            let script = Script::from_slice(script_opt).map_err(|_| Error::InvalidMetaData)?;
            let parsed_hash: [u8; 32] = script.calc_script_hash().unpack();
            if parsed_hash != script_hash {
                return Err(Error::InvalidMetaData);
            }
        }
        0..=4 => return Err(Error::InvalidMetaData),
        _ => return Err(Error::InvalidMetaData),
    }

    Ok(ScriptAttr {
        location,
        script_hash,
    })
}

fn byte32_field(data: &[u8], start: usize, end: usize) -> Result<[u8; 32], Error> {
    if end != start + 32 || end > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 32];
    raw.copy_from_slice(&data[start..end]);
    Ok(raw)
}

fn read_u32(data: &[u8], start: usize) -> Result<u32, Error> {
    if start + 4 > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 4];
    raw.copy_from_slice(&data[start..start + 4]);
    Ok(u32::from_le_bytes(raw))
}
