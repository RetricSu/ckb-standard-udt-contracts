use alloc::vec::Vec;

use standard_udt_types::metadata::XudtMeta;

use crate::{error::Error, meta_cell::config};

pub(crate) fn parse_meta(data: &[u8]) -> Result<XudtMeta, Error> {
    let meta = XudtMeta::from_slice(data).map_err(Error::from)?;
    config::validate_config(meta.config_flags)?;
    Ok(meta)
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
