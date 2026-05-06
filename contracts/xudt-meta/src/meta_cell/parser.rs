use alloc::vec::Vec;

use ckb_std::ckb_types::packed::Script;
use standard_udt_types::metadata::{Authority as TypeAuthority, XudtMeta};

use crate::{error::Error, meta_cell::config};

const XUDT_META_FIELDS: usize = 11;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedXudtMeta {
    pub config_flags: u8,
    pub current_supply: u128,
    pub metadata_fields: Vec<u8>,
    pub mint_authority_raw: Vec<u8>,
    pub metadata_authority_raw: Vec<u8>,
    pub access_authority_raw: Vec<u8>,
    pub extensions_raw: Vec<u8>,
    pub mint_authority: Option<ParsedAuthority>,
    pub metadata_authority: Option<ParsedAuthority>,
    pub access_authority: Option<ParsedAuthority>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedAuthority {
    pub authority_type: u8,
    pub script_hash: [u8; 32],
    pub script: Option<Script>,
}

pub(crate) fn parse_meta(data: &[u8]) -> Result<ParsedXudtMeta, Error> {
    let offsets = table_offsets(data, XUDT_META_FIELDS, false)?;
    let meta = XudtMeta::from_slice(data).map_err(Error::from)?;
    config::validate_config(meta.config_flags)?;

    let metadata_fields = data[offsets[2]..offsets[7]].to_vec();
    let mint_authority_raw = data[offsets[7]..offsets[8]].to_vec();
    let metadata_authority_raw = data[offsets[8]..offsets[9]].to_vec();
    let access_authority_raw = data[offsets[9]..offsets[10]].to_vec();
    let extensions_raw = data[offsets[10]..offsets[11]].to_vec();

    Ok(ParsedXudtMeta {
        config_flags: meta.config_flags,
        current_supply: meta.current_supply,
        metadata_fields,
        mint_authority_raw,
        metadata_authority_raw,
        access_authority_raw,
        extensions_raw,
        mint_authority: meta.mint_authority.map(parsed_authority),
        metadata_authority: meta.metadata_authority.map(parsed_authority),
        access_authority: meta.access_authority.map(parsed_authority),
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

pub(crate) fn byte32_field(data: &[u8], start: usize, end: usize) -> Result<[u8; 32], Error> {
    if end != start + 32 || end > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 32];
    raw.copy_from_slice(&data[start..end]);
    Ok(raw)
}

pub(crate) fn read_u32(data: &[u8], start: usize) -> Result<u32, Error> {
    if start + 4 > data.len() {
        return Err(Error::InvalidMetaData);
    }

    let mut raw = [0u8; 4];
    raw.copy_from_slice(&data[start..start + 4]);
    Ok(u32::from_le_bytes(raw))
}

fn parsed_authority(authority: TypeAuthority) -> ParsedAuthority {
    ParsedAuthority {
        authority_type: authority.authority_type.into(),
        script_hash: authority.script_hash,
        script: authority.script,
    }
}
