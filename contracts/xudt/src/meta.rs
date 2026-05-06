use alloc::vec::Vec;

use ckb_std::{
    ckb_constants::Source,
    ckb_types::{packed::Script, prelude::*},
    error::SysError,
    high_level::{load_cell_data, load_cell_type_hash, load_script},
};
use standard_udt_script_utils::{
    authority::{ParsedAuthority as RuntimeAuthority, check_authority as check_runtime_authority},
    error::ScriptError,
};

use crate::error::Error;

const UDT_AMOUNT_LEN: usize = 16;
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
pub struct ParsedXudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub mint_authority: Option<ParsedAuthority>,
    pub extensions: Vec<ParsedExtension>,
}

impl ParsedXudtMeta {
    pub fn is_supply_tracked(&self) -> bool {
        self.config_flags & CONFIG_SUPPLY_TRACKED != 0
    }

    pub fn is_access_enabled(&self) -> bool {
        self.config_flags & CONFIG_ACCESS_ENABLED != 0
    }

    pub fn is_whitelist(&self) -> bool {
        self.config_flags & CONFIG_ACCESS_WHITELIST != 0
    }

    pub fn is_paused(&self) -> bool {
        self.config_flags & CONFIG_PAUSED != 0
    }
}

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

pub fn load_meta_type_hash_arg() -> Result<[u8; 32], Error> {
    let script = load_script().map_err(Error::from)?;
    let args = script.args().raw_data();
    if args.len() != 32 {
        return Err(Error::InvalidArgs);
    }

    let mut meta_type_hash = [0u8; 32];
    meta_type_hash.copy_from_slice(&args);
    Ok(meta_type_hash)
}

pub fn collect_group_amount(source: Source) -> Result<u128, Error> {
    let mut total = 0u128;
    let mut index = 0;

    loop {
        match load_cell_data(index, source) {
            Ok(data) => {
                let amount = decode_amount(&data)?;
                total = total.checked_add(amount).ok_or(Error::AmountOverflow)?;
                index += 1;
            }
            Err(SysError::IndexOutOfBound) => return Ok(total),
            Err(error) => return Err(error.into()),
        }
    }
}

pub fn find_unique_visible_meta(
    meta_type_hash: &[u8; 32],
) -> Result<Option<ParsedXudtMeta>, Error> {
    let mut found = None;
    for source in [Source::CellDep, Source::Input] {
        if let Some(meta) = find_meta_in_source(meta_type_hash, source)? {
            if found.is_some() {
                return Err(Error::MetaNotUnique);
            }
            found = Some(meta);
        }
    }
    Ok(found)
}

pub fn find_meta_in_source(
    meta_type_hash: &[u8; 32],
    source: Source,
) -> Result<Option<ParsedXudtMeta>, Error> {
    let mut found = None;
    let mut index = 0;

    loop {
        match load_cell_type_hash(index, source) {
            Ok(Some(type_hash)) if &type_hash == meta_type_hash => {
                if found.is_some() {
                    return Err(Error::MetaNotUnique);
                }
                let data = load_cell_data(index, source).map_err(Error::from)?;
                found = Some(parse_meta(&data)?);
                index += 1;
            }
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(found),
            Err(error) => return Err(error.into()),
        }
    }
}

pub fn require_authority(authority: Option<&ParsedAuthority>) -> Result<(), Error> {
    let authority = authority.ok_or(Error::AuthorityMissing)?;
    match check_authority(authority)? {
        true => Ok(()),
        false => Err(Error::AuthorityFailed),
    }
}

pub fn table_offsets(data: &[u8], fields: usize) -> Result<Vec<usize>, Error> {
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

pub fn byte32_field(data: &[u8], start: usize, end: usize) -> Result<[u8; 32], Error> {
    if end != start + 32 || end > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 32];
    raw.copy_from_slice(&data[start..end]);
    Ok(raw)
}

pub fn read_u32(data: &[u8], start: usize) -> Result<u32, Error> {
    if start + 4 > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 4];
    raw.copy_from_slice(&data[start..start + 4]);
    Ok(u32::from_le_bytes(raw))
}

fn decode_amount(data: &[u8]) -> Result<u128, Error> {
    if data.len() < UDT_AMOUNT_LEN {
        return Err(Error::AmountEncoding);
    }

    let mut raw = [0u8; UDT_AMOUNT_LEN];
    raw.copy_from_slice(&data[..UDT_AMOUNT_LEN]);
    Ok(u128::from_le_bytes(raw))
}

fn parse_meta(data: &[u8]) -> Result<ParsedXudtMeta, Error> {
    let offsets = table_offsets(data, XUDT_META_FIELDS)?;
    let config_flags = single_byte_field(data, offsets[0], offsets[1])?;
    validate_config(config_flags)?;

    let current_supply = u128_field(data, offsets[1], offsets[2])?;
    let _decimals = single_byte_field(data, offsets[2], offsets[3])?;
    validate_bytes_field(data, offsets[3], offsets[4], MAX_METADATA_NAME_BYTES)?;
    validate_bytes_field(data, offsets[4], offsets[5], MAX_METADATA_SYMBOL_BYTES)?;
    validate_bytes_field(data, offsets[5], offsets[6], MAX_METADATA_URI_BYTES)?;
    validate_bytes_field(data, offsets[6], offsets[7], MAX_METADATA_EXTRA_DATA_BYTES)?;
    let mint_authority = parse_authority_opt(&data[offsets[7]..offsets[8]])?;
    parse_authority_opt(&data[offsets[8]..offsets[9]])?;
    parse_authority_opt(&data[offsets[9]..offsets[10]])?;
    let extensions = parse_extension_vec(&data[offsets[10]..offsets[11]])?;

    if config_flags & CONFIG_SUPPLY_TRACKED == 0 && current_supply != 0 {
        return Err(Error::InvalidMetaData);
    }

    Ok(ParsedXudtMeta {
        config_flags,
        current_supply,
        mint_authority,
        extensions,
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
        ScriptError::AuthorityFailed => Error::AuthorityFailed,
        ScriptError::UnsupportedAuthorityLocation => Error::UnsupportedAuthorityLocation,
        ScriptError::InvalidAuthority => Error::InvalidMetaData,
        ScriptError::SyscallUnknown => Error::SyscallUnknown,
        _ => Error::SyscallUnknown,
    }
}

fn parse_authority_opt(data: &[u8]) -> Result<Option<ParsedAuthority>, Error> {
    if data.is_empty() {
        return Ok(None);
    }
    parse_authority(data).map(Some)
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

    let mut extensions = Vec::with_capacity(count);
    let mut previous_key: Option<(u8, [u8; 32])> = None;
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
