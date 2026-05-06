use alloc::vec::Vec;

use ckb_std::{
    ckb_constants::Source,
    ckb_types::packed::Script,
    error::SysError,
    high_level::{load_cell_data, load_cell_type_hash, load_script},
};
use standard_udt_script_utils::{
    authority::{ParsedAuthority as RuntimeAuthority, check_authority as check_runtime_authority},
    error::ScriptError,
};
use standard_udt_types::metadata::{
    Authority as TypeAuthority, Extension as TypeExtension, XudtMeta,
};

use crate::error::Error;

const UDT_AMOUNT_LEN: usize = 16;
const CONFIG_SUPPLY_TRACKED: u8 = 0b0000_0001;
const CONFIG_ACCESS_ENABLED: u8 = 0b0000_0010;
const CONFIG_ACCESS_WHITELIST: u8 = 0b0000_0100;
const CONFIG_PAUSED: u8 = 0b0000_1000;

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
    let meta = XudtMeta::from_slice(data).map_err(Error::from)?;

    Ok(ParsedXudtMeta {
        config_flags: meta.config_flags,
        current_supply: meta.current_supply,
        mint_authority: meta.mint_authority.map(parsed_authority),
        extensions: meta.extensions.into_iter().map(parsed_extension).collect(),
    })
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

fn parsed_authority(authority: TypeAuthority) -> ParsedAuthority {
    ParsedAuthority {
        authority_type: authority.authority_type.into(),
        script_hash: authority.script_hash,
        script: authority.script,
    }
}

fn parsed_extension(extension: TypeExtension) -> ParsedExtension {
    ParsedExtension {
        extension_type: extension.extension_type.into(),
        script: extension.script,
    }
}
