use alloc::vec::Vec;

use ckb_std::{
    ckb_constants::Source,
    ckb_types::{core::ScriptHashType, prelude::*},
    error::SysError,
    high_level::{load_cell_data, load_cell_type, load_script, load_script_hash},
    type_id::check_type_id,
};

use crate::error::Error;

pub const CONFIG_SUPPLY_TRACKED: u8 = 0b0000_0001;
const SUDT_META_FIELDS: usize = 9;
const SUDT_ALLOWED_CONFIG_MASK: u8 = CONFIG_SUPPLY_TRACKED;
// Temporary explicit UDT code-hash source until the repo has stable build-time
// constants: tracked sUDT Meta stores the expected enhanced-sudt Data2 code hash
// as the whole `extra_data` field.
const UDT_CODE_HASH_CONFIG_LEN: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub udt_code_hash: Option<[u8; 32]>,
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
        let udt_code_hash = output_meta.udt_code_hash.ok_or(Error::InvalidMetaData)?;
        let initial_supply = sum_initial_udt_outputs(meta_type_hash, &udt_code_hash)?;
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
    let udt_code_hash = udt_code_hash_config(data, offsets[6], offsets[7])?;

    if !is_supply_tracked(config_flags) && current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    Ok(SudtMeta {
        config_flags,
        current_supply,
        udt_code_hash,
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

fn udt_code_hash_config(data: &[u8], start: usize, end: usize) -> Result<Option<[u8; 32]>, Error> {
    let extra_data = molecule_bytes(data, start, end)?;
    if extra_data.is_empty() {
        return Ok(None);
    }
    if extra_data.len() != UDT_CODE_HASH_CONFIG_LEN {
        return Err(Error::InvalidMetaData);
    }

    let mut code_hash = [0u8; 32];
    code_hash.copy_from_slice(extra_data);
    Ok(Some(code_hash))
}

fn molecule_bytes(data: &[u8], start: usize, end: usize) -> Result<&[u8], Error> {
    if end < start + 4 || end > data.len() {
        return Err(Error::InvalidMetaData);
    }
    let len = read_u32(data, start)? as usize;
    let bytes_start = start + 4;
    let bytes_end = bytes_start.checked_add(len).ok_or(Error::InvalidMetaData)?;
    if bytes_end != end {
        return Err(Error::InvalidMetaData);
    }
    Ok(&data[bytes_start..bytes_end])
}

fn read_u32(data: &[u8], start: usize) -> Result<u32, Error> {
    if start + 4 > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 4];
    raw.copy_from_slice(&data[start..start + 4]);
    Ok(u32::from_le_bytes(raw))
}
