use alloc::vec::Vec;

use ckb_std::{
    ckb_constants::Source,
    ckb_types::{core::ScriptHashType, packed::Script, prelude::*},
    error::SysError,
    high_level::{load_cell_data, load_cell_lock, load_cell_type, load_script, load_script_hash},
    type_id::check_type_id,
};
use standard_udt_types::metadata::{Authority as TypeAuthority, SudtMeta};

use crate::{constants::SUDT_CODE_HASH, error::Error};

pub const CONFIG_SUPPLY_TRACKED: u8 = 0b0000_0001;
const SUDT_META_FIELDS: usize = 9;
const ALWAYS_SUCCESS_LOCK_CODE_HASH_WHITELIST: [[u8; 32]; 1] = [[
    0x3b, 0x52, 0x1c, 0xc4, 0xb5, 0x52, 0xf1, 0x09, 0xd0, 0x92, 0xd8, 0xcc, 0x46, 0x8a, 0x80, 0x48,
    0xac, 0xb5, 0x3c, 0x59, 0x52, 0xdb, 0xe7, 0x69, 0xd2, 0xb2, 0xf9, 0xcf, 0x6e, 0x47, 0xf7, 0xf1,
]];

#[cfg(debug_assertions)]
const TESTTOOL_ALWAYS_SUCCESS_LOCK_CODE_HASH: [u8; 32] = [
    0xe6, 0x83, 0xb0, 0x41, 0x39, 0x34, 0x47, 0x68, 0x34, 0x84, 0x99, 0xc2, 0x3e, 0xb1, 0x32, 0x6d,
    0x5a, 0x52, 0xd6, 0xdb, 0x00, 0x6c, 0x0d, 0x2f, 0xec, 0xe0, 0x0a, 0x83, 0x1f, 0x36, 0x60, 0xd7,
];

fn is_allowed_always_success_lock_code_hash(code_hash: &[u8; 32]) -> bool {
    if ALWAYS_SUCCESS_LOCK_CODE_HASH_WHITELIST.contains(code_hash) {
        return true;
    }

    #[cfg(debug_assertions)]
    {
        code_hash == &TESTTOOL_ALWAYS_SUCCESS_LOCK_CODE_HASH
    }

    #[cfg(not(debug_assertions))]
    {
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedSudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub metadata_fields: Vec<u8>,
    pub mint_authority_raw: Vec<u8>,
    pub metadata_authority_raw: Vec<u8>,
    pub mint_authority: Option<ParsedAuthority>,
    pub metadata_authority: Option<ParsedAuthority>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedAuthority {
    pub authority_type: u8,
    pub script_hash: [u8; 32],
    pub script: Option<Script>,
}

pub struct MetaGroup {
    pub input: Option<ParsedSudtMeta>,
    pub output: Option<ParsedSudtMeta>,
    pub meta_type_hash: [u8; 32],
}

pub fn load_meta_group() -> Result<MetaGroup, Error> {
    Ok(MetaGroup {
        input: load_group_meta(Source::GroupInput)?,
        output: load_group_meta(Source::GroupOutput)?,
        meta_type_hash: load_script_hash().map_err(Error::from)?,
    })
}

pub fn validate_type_args() -> Result<(), Error> {
    let script = load_script().map_err(Error::from)?;
    if script.args().raw_data().len() != 32 {
        return Err(Error::InvalidArgs);
    }
    Ok(())
}

pub fn validate_create_type_id() -> Result<(), Error> {
    check_type_id(0, 32).map_err(|_| Error::InvalidTypeId)
}

pub fn validate_create(
    output_meta: &ParsedSudtMeta,
    meta_type_hash: &[u8; 32],
) -> Result<(), Error> {
    if is_supply_tracked(output_meta.config_flags) {
        let initial_supply = sum_initial_udt_outputs(meta_type_hash, &SUDT_CODE_HASH)?;
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

        if is_initial_udt_script(&type_script, meta_type_hash, udt_code_hash) {
            let data = load_cell_data(index, Source::Output).map_err(Error::from)?;
            let amount = decode_amount(&data)?;
            total = total.checked_add(amount).ok_or(Error::InvalidSupply)?;
        }

        index += 1;
    }
}

fn decode_amount(data: &[u8]) -> Result<u128, Error> {
    if data.len() < 16 {
        return Err(Error::InvalidSupply);
    }

    let mut raw = [0u8; 16];
    raw.copy_from_slice(&data[..16]);
    Ok(u128::from_le_bytes(raw))
}

fn load_group_meta(source: Source) -> Result<Option<ParsedSudtMeta>, Error> {
    let mut found = None;
    let mut index = 0;

    loop {
        match load_cell_data(index, source) {
            Ok(data) => {
                if found.is_some() {
                    return Err(Error::InvalidArgs);
                }
                if source == Source::GroupOutput {
                    validate_meta_lock(index)?;
                }
                found = Some(parse_meta(&data)?);
                index += 1;
            }
            Err(SysError::IndexOutOfBound) => return Ok(found),
            Err(error) => return Err(error.into()),
        }
    }
}

fn validate_meta_lock(index: usize) -> Result<(), Error> {
    let lock = load_cell_lock(index, Source::GroupOutput).map_err(Error::from)?;
    let code_hash: [u8; 32] = lock.code_hash().unpack();
    if is_allowed_always_success_lock_code_hash(&code_hash) {
        Ok(())
    } else {
        Err(Error::InvalidArgs)
    }
}

fn parse_meta(data: &[u8]) -> Result<ParsedSudtMeta, Error> {
    let offsets = table_offsets(data, SUDT_META_FIELDS)?;
    let meta = SudtMeta::from_slice(data).map_err(Error::from)?;

    let metadata_fields = data[offsets[2]..offsets[7]].to_vec();
    let mint_authority_raw = data[offsets[7]..offsets[8]].to_vec();
    let metadata_authority_raw = data[offsets[8]..offsets[9]].to_vec();

    Ok(ParsedSudtMeta {
        config_flags: meta.config_flags,
        current_supply: meta.current_supply,
        metadata_fields,
        mint_authority_raw,
        metadata_authority_raw,
        mint_authority: meta.mint_authority.map(parsed_authority),
        metadata_authority: meta.metadata_authority.map(parsed_authority),
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

fn parsed_authority(authority: TypeAuthority) -> ParsedAuthority {
    ParsedAuthority {
        authority_type: authority.authority_type.into(),
        script_hash: authority.script_hash,
        script: authority.script,
    }
}

fn read_u32(data: &[u8], start: usize) -> Result<u32, Error> {
    if start + 4 > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 4];
    raw.copy_from_slice(&data[start..start + 4]);
    Ok(u32::from_le_bytes(raw))
}
