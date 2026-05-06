use alloc::vec::Vec;

use ckb_std::ckb_types::{packed::Script, prelude::*};

use crate::error::Error;

use super::ParsedXudtMeta;

const XUDT_META_FIELDS: usize = 11;
const AUTHORITY_FIELDS: usize = 3;
const EXTENSION_FIELDS: usize = 2;
const CONFIG_SUPPLY_TRACKED: u8 = 0b0000_0001;
const CONFIG_ACCESS_ENABLED: u8 = 0b0000_0010;
const CONFIG_ACCESS_WHITELIST: u8 = 0b0000_0100;
const CONFIG_PAUSED: u8 = 0b0000_1000;
const XUDT_ALLOWED_CONFIG_MASK: u8 =
    CONFIG_SUPPLY_TRACKED | CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST | CONFIG_PAUSED;
const MAX_EXTENSIONS: usize = 16;
const MAX_METADATA_NAME_BYTES: usize = 1024;
const MAX_METADATA_SYMBOL_BYTES: usize = 128;
const MAX_METADATA_URI_BYTES: usize = 2048;
const MAX_METADATA_EXTRA_DATA_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedAuthority {
    pub authority_type: u8,
    pub script_hash: [u8; 32],
    pub script: Option<Script>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedExtension {
    pub extension_type: u8,
    pub script: Script,
}

pub(super) fn parse_meta(data: &[u8]) -> Result<ParsedXudtMeta, Error> {
    let offsets = table_offsets(data, XUDT_META_FIELDS)?;
    let config_flags = single_byte_field(data, offsets[0], offsets[1])?;
    validate_config(config_flags)?;

    let current_supply = u128_field(data, offsets[1], offsets[2])?;
    let _decimals = single_byte_field(data, offsets[2], offsets[3])?;
    validate_bytes_field(data, offsets[3], offsets[4], MAX_METADATA_NAME_BYTES)?;
    validate_bytes_field(data, offsets[4], offsets[5], MAX_METADATA_SYMBOL_BYTES)?;
    validate_bytes_field(data, offsets[5], offsets[6], MAX_METADATA_URI_BYTES)?;
    validate_bytes_field(data, offsets[6], offsets[7], MAX_METADATA_EXTRA_DATA_BYTES)?;
    parse_authority_opt(&data[offsets[7]..offsets[8]])?;
    parse_authority_opt(&data[offsets[8]..offsets[9]])?;
    let access_authority = parse_authority_opt(&data[offsets[9]..offsets[10]])?;
    parse_extension_vec(&data[offsets[10]..offsets[11]])?;

    if config_flags & CONFIG_SUPPLY_TRACKED == 0 && current_supply != 0 {
        return Err(Error::InvalidMetaData);
    }

    Ok(ParsedXudtMeta {
        config_flags,
        access_authority,
    })
}

fn validate_config(config_flags: u8) -> Result<(), Error> {
    if config_flags & !XUDT_ALLOWED_CONFIG_MASK != 0 {
        return Err(Error::InvalidMetaData);
    }
    if config_flags & CONFIG_ACCESS_WHITELIST != 0 && config_flags & CONFIG_ACCESS_ENABLED == 0 {
        return Err(Error::InvalidMetaData);
    }
    Ok(())
}

fn parse_extension_vec(data: &[u8]) -> Result<Vec<ParsedExtension>, Error> {
    if data.len() < 4 {
        return Err(Error::InvalidMetaData);
    }
    let total_size = read_u32(data, 0)? as usize;
    if total_size != data.len() {
        return Err(Error::InvalidMetaData);
    }
    if total_size == 4 {
        return Ok(Vec::new());
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

    let mut previous_key: Option<(u8, [u8; 32])> = None;
    let mut extensions = Vec::with_capacity(count);
    for pair in offsets.windows(2) {
        if pair[0] > pair[1] {
            return Err(Error::InvalidMetaData);
        }
        let extension = parse_extension(&data[pair[0]..pair[1]])?;
        let script_hash: [u8; 32] = extension.script.calc_script_hash().unpack();
        let key = (extension.extension_type, script_hash);
        if let Some(previous) = previous_key {
            if key <= previous {
                return Err(Error::InvalidMetaData);
            }
        }
        previous_key = Some(key);
        extensions.push(extension);
    }
    Ok(extensions)
}

fn parse_authority_opt(data: &[u8]) -> Result<Option<ParsedAuthority>, Error> {
    if data.is_empty() {
        return Ok(None);
    }
    parse_authority(data).map(Some)
}

fn parse_authority(data: &[u8]) -> Result<ParsedAuthority, Error> {
    let offsets = table_offsets(data, AUTHORITY_FIELDS)?;
    let authority_type = single_byte_field(data, offsets[0], offsets[1])?;
    let script_hash = byte32_field(data, offsets[1], offsets[2])?;
    let script_opt = &data[offsets[2]..offsets[3]];

    let script = match authority_type {
        0..=2 if script_opt.is_empty() => None,
        3 | 4 if !script_opt.is_empty() => {
            let script = Script::from_slice(script_opt).map_err(|_| Error::InvalidMetaData)?;
            let parsed_hash: [u8; 32] = script.calc_script_hash().unpack();
            if parsed_hash != script_hash {
                return Err(Error::InvalidMetaData);
            }
            Some(script)
        }
        0..=4 => return Err(Error::InvalidMetaData),
        _ => return Err(Error::InvalidMetaData),
    };

    Ok(ParsedAuthority {
        authority_type,
        script_hash,
        script,
    })
}

fn parse_extension(data: &[u8]) -> Result<ParsedExtension, Error> {
    let offsets = table_offsets(data, EXTENSION_FIELDS)?;
    let extension_type = single_byte_field(data, offsets[0], offsets[1])?;
    if extension_type > 1 {
        return Err(Error::InvalidMetaData);
    }
    let script =
        Script::from_slice(&data[offsets[1]..offsets[2]]).map_err(|_| Error::InvalidMetaData)?;

    Ok(ParsedExtension {
        extension_type,
        script,
    })
}

fn table_offsets(data: &[u8], fields: usize) -> Result<Vec<usize>, Error> {
    if data.len() < 4 + fields * 4 {
        return Err(Error::InvalidMetaData);
    }

    let total_size = read_u32(data, 0)? as usize;
    if total_size != data.len() {
        return Err(Error::InvalidMetaData);
    }

    let first_offset = read_u32(data, 4)? as usize;
    if first_offset != 4 + fields * 4 {
        return Err(Error::InvalidMetaData);
    }

    let mut offsets = Vec::with_capacity(fields + 1);
    for index in 0..fields {
        offsets.push(read_u32(data, 4 + index * 4)? as usize);
    }
    offsets.push(total_size);

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

fn byte32_field(data: &[u8], start: usize, end: usize) -> Result<[u8; 32], Error> {
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

fn read_u32(data: &[u8], start: usize) -> Result<u32, Error> {
    if start + 4 > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 4];
    raw.copy_from_slice(&data[start..start + 4]);
    Ok(u32::from_le_bytes(raw))
}
