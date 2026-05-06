use alloc::vec::Vec;

use ckb_std::ckb_types::{packed::Script, prelude::*};

use crate::{error::Error, meta_cell::config};

const XUDT_META_FIELDS: usize = 11;
const SCRIPT_ATTR_FIELDS: usize = 3;
const MAX_EXTENSIONS: usize = 16;
const MAX_METADATA_NAME_BYTES: usize = 1024;
const MAX_METADATA_SYMBOL_BYTES: usize = 128;
const MAX_METADATA_URI_BYTES: usize = 2048;
const MAX_METADATA_EXTRA_DATA_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub metadata_fields: Vec<u8>,
    pub mint_authority_raw: Vec<u8>,
    pub metadata_authority_raw: Vec<u8>,
    pub access_authority_raw: Vec<u8>,
    pub extensions_raw: Vec<u8>,
    pub mint_authority: Option<ScriptAttr>,
    pub metadata_authority: Option<ScriptAttr>,
    pub access_authority: Option<ScriptAttr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptAttr {
    pub location: u8,
    pub script_hash: [u8; 32],
}

pub(crate) fn parse_meta(data: &[u8]) -> Result<XudtMeta, Error> {
    // `standard_udt_types::metadata::XudtMeta::from_slice` is intentionally not
    // used in this RISC-V binary for parity with sudt: linking it here
    // currently pulls duplicate ckb-std atomic dummy symbols.
    let offsets = table_offsets(data, XUDT_META_FIELDS, false)?;
    let config_flags = single_byte_field(data, offsets[0], offsets[1])?;
    config::validate_config(config_flags)?;

    let current_supply = u128_field(data, offsets[1], offsets[2])?;
    let _decimals = single_byte_field(data, offsets[2], offsets[3])?;
    validate_bytes_field(data, offsets[3], offsets[4], MAX_METADATA_NAME_BYTES)?;
    validate_bytes_field(data, offsets[4], offsets[5], MAX_METADATA_SYMBOL_BYTES)?;
    validate_bytes_field(data, offsets[5], offsets[6], MAX_METADATA_URI_BYTES)?;
    validate_bytes_field(data, offsets[6], offsets[7], MAX_METADATA_EXTRA_DATA_BYTES)?;
    let metadata_fields = data[offsets[2]..offsets[7]].to_vec();
    let mint_authority_raw = data[offsets[7]..offsets[8]].to_vec();
    let metadata_authority_raw = data[offsets[8]..offsets[9]].to_vec();
    let access_authority_raw = data[offsets[9]..offsets[10]].to_vec();
    let extensions_raw = data[offsets[10]..offsets[11]].to_vec();
    let mint_authority = parse_script_attr_opt(&mint_authority_raw)?;
    let metadata_authority = parse_script_attr_opt(&metadata_authority_raw)?;
    let access_authority = parse_script_attr_opt(&access_authority_raw)?;
    parse_script_attr_vec(&extensions_raw)?;

    if !config::is_supply_tracked(config_flags) && current_supply != 0 {
        return Err(Error::InvalidSupply);
    }

    Ok(XudtMeta {
        config_flags,
        current_supply,
        metadata_fields,
        mint_authority_raw,
        metadata_authority_raw,
        access_authority_raw,
        extensions_raw,
        mint_authority,
        metadata_authority,
        access_authority,
    })
}

fn parse_script_attr_vec(data: &[u8]) -> Result<(), Error> {
    if data.len() < 4 {
        return Err(Error::InvalidMetaData);
    }
    let total_size = read_u32(data, 0)? as usize;
    if total_size != data.len() {
        return Err(Error::InvalidMetaData);
    }
    if total_size == 4 {
        return Ok(());
    }
    let first_offset = read_u32(data, 4)? as usize;
    if first_offset < 8 || first_offset % 4 != 0 || first_offset > total_size {
        return Err(Error::InvalidMetaData);
    }
    let count = first_offset / 4 - 1;
    if count > MAX_EXTENSIONS {
        return Err(Error::InvalidMetaData);
    }

    let mut offsets = Vec::with_capacity(count + 1);
    for index in 0..count {
        offsets.push(read_u32(data, 4 + index * 4)? as usize);
    }
    offsets.push(total_size);

    let mut prev_key: Option<(u8, [u8; 32])> = None;
    for pair in offsets.windows(2) {
        if pair[0] > pair[1] {
            return Err(Error::InvalidMetaData);
        }
        let attr = parse_script_attr(&data[pair[0]..pair[1]])?;
        let key = (attr.location, attr.script_hash);
        if let Some(prev) = prev_key {
            if key <= prev {
                return Err(Error::InvalidMetaData);
            }
        }
        prev_key = Some(key);
    }
    Ok(())
}

fn parse_script_attr_opt(data: &[u8]) -> Result<Option<ScriptAttr>, Error> {
    if data.is_empty() {
        return Ok(None);
    }
    parse_script_attr(data).map(Some)
}

fn parse_script_attr(data: &[u8]) -> Result<ScriptAttr, Error> {
    let offsets = table_offsets(data, SCRIPT_ATTR_FIELDS, false)?;
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

pub(crate) fn table_offsets(
    data: &[u8],
    fields: usize,
    allow_extra_fields: bool,
) -> Result<Vec<usize>, Error> {
    if data.len() < 4 + fields * 4 {
        return Err(Error::InvalidMetaData);
    }

    let total_size = read_u32(data, 0)? as usize;
    if total_size != data.len() {
        return Err(Error::InvalidMetaData);
    }

    let first_offset = read_u32(data, 4)? as usize;
    if first_offset < 4 + fields * 4 || first_offset % 4 != 0 {
        return Err(Error::InvalidMetaData);
    }
    let actual_fields = first_offset / 4 - 1;
    if actual_fields < fields || (!allow_extra_fields && actual_fields != fields) {
        return Err(Error::InvalidMetaData);
    }

    let mut offsets = Vec::with_capacity(actual_fields + 1);
    for index in 0..actual_fields {
        offsets.push(read_u32(data, 4 + index * 4)? as usize);
    }
    offsets.push(total_size);

    if offsets[0] != first_offset {
        return Err(Error::InvalidMetaData);
    }
    for index in 1..offsets.len() {
        if offsets[index] < offsets[index - 1] || offsets[index] > total_size {
            return Err(Error::InvalidMetaData);
        }
    }

    Ok(offsets)
}

pub(crate) fn single_byte_field(data: &[u8], start: usize, end: usize) -> Result<u8, Error> {
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

pub(crate) fn byte32_field(data: &[u8], start: usize, end: usize) -> Result<[u8; 32], Error> {
    if end != start + 32 || end > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 32];
    raw.copy_from_slice(&data[start..end]);
    Ok(raw)
}

fn validate_bytes_field(
    data: &[u8],
    start: usize,
    end: usize,
    max_len: usize,
) -> Result<(), Error> {
    if end < start || end > data.len() || end - start < 4 {
        return Err(Error::InvalidMetaData);
    }

    let count = read_u32(data, start)? as usize;
    if count > max_len || end - start != 4 + count {
        return Err(Error::InvalidMetaData);
    }

    Ok(())
}

pub(crate) fn read_u32(data: &[u8], start: usize) -> Result<u32, Error> {
    if start + 4 > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 4];
    raw.copy_from_slice(&data[start..start + 4]);
    Ok(u32::from_le_bytes(raw))
}
