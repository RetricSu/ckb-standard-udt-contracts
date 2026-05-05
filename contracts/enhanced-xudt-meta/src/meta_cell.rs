use alloc::vec::Vec;

use ckb_std::{
    ckb_constants::Source,
    ckb_types::{core::ScriptHashType, packed::Script, prelude::*},
    error::SysError,
    high_level::{load_cell_data, load_cell_lock, load_cell_type, load_script, load_script_hash},
    type_id::check_type_id,
};

use crate::{
    constants::{ACCESS_LIST_CODE_HASH, ENHANCED_XUDT_CODE_HASH},
    error::Error,
};

pub const CONFIG_SUPPLY_TRACKED: u8 = 0b0000_0001;
pub const CONFIG_ACCESS_ENABLED: u8 = 0b0000_0010;
pub const CONFIG_ACCESS_WHITELIST: u8 = 0b0000_0100;
pub const CONFIG_PAUSED: u8 = 0b0000_1000;

const XUDT_META_FIELDS: usize = 11;
const SCRIPT_ATTR_FIELDS: usize = 3;
const ACCESS_LIST_SHARD_FIELDS: usize = 2;
const XUDT_ALLOWED_CONFIG_MASK: u8 =
    CONFIG_SUPPLY_TRACKED | CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST | CONFIG_PAUSED;
const MAX_EXTENSIONS: usize = 16;
const MAX_ACCESSLIST_ENTRIES: usize = 8192;

// Current compatibility whitelist is by lock code_hash, matching enhanced-sudt.
const META_LOCK_CODE_HASH_WHITELIST: [[u8; 32]; 2] = [
    [
        0x3b, 0x52, 0x1c, 0xc4, 0xb5, 0x52, 0xf1, 0x09, 0xd0, 0x92, 0xd8, 0xcc, 0x46, 0x8a, 0x80,
        0x48, 0xac, 0xb5, 0x3c, 0x59, 0x52, 0xdb, 0xe7, 0x69, 0xd2, 0xb2, 0xf9, 0xcf, 0x6e, 0x47,
        0xf7, 0xf1,
    ],
    [
        0xe6, 0x83, 0xb0, 0x41, 0x39, 0x34, 0x47, 0x68, 0x34, 0x84, 0x99, 0xc2, 0x3e, 0xb1, 0x32,
        0x6d, 0x5a, 0x52, 0xd6, 0xdb, 0x00, 0x6c, 0x0d, 0x2f, 0xec, 0xe0, 0x0a, 0x83, 0x1f, 0x36,
        0x60, 0xd7,
    ],
];

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

pub struct MetaGroup {
    pub input: Option<XudtMeta>,
    pub output: Option<XudtMeta>,
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

pub fn validate_create(output_meta: &XudtMeta, meta_type_hash: &[u8; 32]) -> Result<(), Error> {
    if is_supply_tracked(output_meta.config_flags) {
        let initial_supply = sum_initial_udt_outputs(meta_type_hash, &ENHANCED_XUDT_CODE_HASH)?;
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

        if is_token_script(&type_script, meta_type_hash, udt_code_hash) {
            let data = load_cell_data(index, Source::Output).map_err(|_| Error::Syscall)?;
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
                Ok(Some(script))
                    if is_token_script(&script, meta_type_hash, &ENHANCED_XUDT_CODE_HASH) =>
                {
                    return Ok(true);
                }
                Ok(_) => index += 1,
                Err(SysError::IndexOutOfBound) => break,
                Err(_) => return Err(Error::Syscall),
            }
        }
    }
    Ok(false)
}

pub fn has_legal_access_list_shard(meta_type_hash: &[u8; 32]) -> Result<bool, Error> {
    let mut index = 0;
    loop {
        match load_cell_type(index, Source::Output) {
            Ok(Some(script)) if is_access_list_script(&script, meta_type_hash) => {
                let data = load_cell_data(index, Source::Output).map_err(|_| Error::Syscall)?;
                parse_access_list_shard(&data)?;
                return Ok(true);
            }
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(false),
            Err(_) => return Err(Error::Syscall),
        }
    }
}

pub fn has_full_domain_access_list_shards(meta_type_hash: &[u8; 32]) -> Result<bool, Error> {
    let mut index = 0;
    loop {
        match load_cell_type(index, Source::Output) {
            Ok(Some(script)) if is_access_list_script(&script, meta_type_hash) => {
                let data = load_cell_data(index, Source::Output).map_err(|_| Error::Syscall)?;
                let shard = parse_access_list_shard(&data)?;
                if shard.start == [0u8; 32] && shard.end == [0xffu8; 32] && shard.entries == 0 {
                    return Ok(true);
                }
                index += 1;
            }
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(false),
            Err(_) => return Err(Error::Syscall),
        }
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

fn load_group_meta(source: Source) -> Result<Option<XudtMeta>, Error> {
    let mut found = None;
    let mut index = 0;

    loop {
        match load_cell_data(index, source) {
            Ok(data) => {
                if found.is_some() {
                    return Err(Error::InvalidArgs);
                }
                validate_meta_lock(index, source)?;
                found = Some(parse_meta(&data)?);
                index += 1;
            }
            Err(SysError::IndexOutOfBound) => return Ok(found),
            Err(_) => return Err(Error::Syscall),
        }
    }
}

fn validate_meta_lock(index: usize, source: Source) -> Result<(), Error> {
    let lock = load_cell_lock(index, source).map_err(|_| Error::Syscall)?;
    let code_hash: [u8; 32] = lock.code_hash().unpack();
    if META_LOCK_CODE_HASH_WHITELIST.contains(&code_hash) {
        Ok(())
    } else {
        Err(Error::InvalidArgs)
    }
}

fn parse_meta(data: &[u8]) -> Result<XudtMeta, Error> {
    // `standard_udt_types::metadata::XudtMeta::from_slice` is intentionally not
    // used in this RISC-V binary for parity with enhanced-sudt: linking it here
    // currently pulls duplicate ckb-std atomic dummy symbols.
    let offsets = table_offsets(data, XUDT_META_FIELDS, false)?;
    let config_flags = single_byte_field(data, offsets[0], offsets[1])?;
    validate_config(config_flags)?;

    let current_supply = u128_field(data, offsets[1], offsets[2])?;
    let _decimals = single_byte_field(data, offsets[2], offsets[3])?;
    let metadata_fields = data[offsets[2]..offsets[7]].to_vec();
    let mint_authority_raw = data[offsets[7]..offsets[8]].to_vec();
    let metadata_authority_raw = data[offsets[8]..offsets[9]].to_vec();
    let access_authority_raw = data[offsets[9]..offsets[10]].to_vec();
    let extensions_raw = data[offsets[10]..offsets[11]].to_vec();
    let mint_authority = parse_script_attr_opt(&mint_authority_raw)?;
    let metadata_authority = parse_script_attr_opt(&metadata_authority_raw)?;
    let access_authority = parse_script_attr_opt(&access_authority_raw)?;
    parse_script_attr_vec(&extensions_raw)?;

    if !is_supply_tracked(config_flags) && current_supply != 0 {
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

fn validate_config(config_flags: u8) -> Result<(), Error> {
    if config_flags & !XUDT_ALLOWED_CONFIG_MASK != 0 {
        return Err(Error::InvalidMetaData);
    }
    if config_flags & CONFIG_ACCESS_WHITELIST != 0 && config_flags & CONFIG_ACCESS_ENABLED == 0 {
        return Err(Error::InvalidMetaData);
    }
    Ok(())
}

fn is_token_script(type_script: &Script, meta_type_hash: &[u8; 32], code_hash: &[u8; 32]) -> bool {
    if type_script.hash_type() != ScriptHashType::Data2.into() {
        return false;
    }
    if type_script.args().raw_data().as_ref() != meta_type_hash {
        return false;
    }

    let actual_code_hash: [u8; 32] = type_script.code_hash().unpack();
    &actual_code_hash == code_hash
}

fn is_access_list_script(type_script: &Script, meta_type_hash: &[u8; 32]) -> bool {
    is_token_script(type_script, meta_type_hash, &ACCESS_LIST_CODE_HASH)
}

pub fn is_supply_tracked(config_flags: u8) -> bool {
    config_flags & CONFIG_SUPPLY_TRACKED != 0
}

pub fn access_enabled(config_flags: u8) -> bool {
    config_flags & CONFIG_ACCESS_ENABLED != 0
}

pub fn whitelist_mode(config_flags: u8) -> bool {
    config_flags & CONFIG_ACCESS_WHITELIST != 0
}

pub fn paused(config_flags: u8) -> bool {
    config_flags & CONFIG_PAUSED != 0
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

#[derive(Clone, Copy)]
struct AccessListShard {
    start: [u8; 32],
    end: [u8; 32],
    entries: usize,
}

fn parse_access_list_shard(data: &[u8]) -> Result<AccessListShard, Error> {
    let offsets = table_offsets(data, ACCESS_LIST_SHARD_FIELDS, true)?;
    if offsets[1] != offsets[0] + 64 {
        return Err(Error::InvalidMetaData);
    }
    let start = byte32_field(data, offsets[0], offsets[0] + 32)?;
    let end = byte32_field(data, offsets[0] + 32, offsets[1])?;
    let entries = parse_byte32_vec_count(&data[offsets[1]..offsets[2]])?;
    Ok(AccessListShard {
        start,
        end,
        entries,
    })
}

fn parse_byte32_vec_count(data: &[u8]) -> Result<usize, Error> {
    if data.len() < 4 {
        return Err(Error::InvalidMetaData);
    }
    let count = read_u32(data, 0)? as usize;
    if count > MAX_ACCESSLIST_ENTRIES || data.len() != 4 + count * 32 {
        return Err(Error::InvalidMetaData);
    }
    Ok(count)
}

fn table_offsets(
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

fn read_u32(data: &[u8], start: usize) -> Result<u32, Error> {
    if start + 4 > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 4];
    raw.copy_from_slice(&data[start..start + 4]);
    Ok(u32::from_le_bytes(raw))
}
