use ckb_std::{
    ckb_constants::Source,
    ckb_types::packed::Script,
    error::SysError,
    high_level::{load_cell_data, load_cell_type},
};
use standard_udt_types::metadata::AccessListShard;

use crate::{
    constants::ACCESS_LIST_CODE_HASH, error::Error, state::token::matches_bound_type_script,
};

const FULL_START: [u8; 32] = [0u8; 32];
const FULL_END: [u8; 32] = [0xffu8; 32];

pub fn has_full_domain_access_list_inputs(meta_type_hash: &[u8; 32]) -> Result<bool, Error> {
    has_full_domain_access_list_cells(meta_type_hash, Source::Input)
}

pub fn has_full_domain_access_list_outputs(meta_type_hash: &[u8; 32]) -> Result<bool, Error> {
    has_full_domain_access_list_cells(meta_type_hash, Source::Output)
}

pub fn has_bound_access_list_outputs(meta_type_hash: &[u8; 32]) -> Result<bool, Error> {
    has_bound_access_list_cells(meta_type_hash, Source::Output)
}

fn has_bound_access_list_cells(meta_type_hash: &[u8; 32], source: Source) -> Result<bool, Error> {
    let mut index = 0;

    loop {
        match load_cell_type(index, source) {
            Ok(Some(script)) if is_access_list_script(&script, meta_type_hash) => return Ok(true),
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(false),
            Err(error) => return Err(error.into()),
        }
    }
}

fn has_full_domain_access_list_cells(
    meta_type_hash: &[u8; 32],
    source: Source,
) -> Result<bool, Error> {
    let mut ranges = alloc::vec::Vec::new();
    let mut index = 0;

    loop {
        match load_cell_type(index, source) {
            Ok(Some(script)) if is_access_list_script(&script, meta_type_hash) => {
                let data = load_cell_data(index, source)?;
                ranges.push(parse_access_list_range(&data)?);
                index += 1;
            }
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => break,
            Err(error) => return Err(error.into()),
        }
    }

    ranges.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    Ok(covers_full_domain(&ranges))
}

fn is_access_list_script(type_script: &Script, meta_type_hash: &[u8; 32]) -> bool {
    matches_bound_type_script(type_script, meta_type_hash, &ACCESS_LIST_CODE_HASH)
}

fn parse_access_list_range(data: &[u8]) -> Result<([u8; 32], [u8; 32]), Error> {
    let shard = AccessListShard::from_slice(data).map_err(|_| Error::AccessListRequired)?;
    Ok((shard.range.start, shard.range.end))
}

fn covers_full_domain(ranges: &[([u8; 32], [u8; 32])]) -> bool {
    if ranges.is_empty() || ranges[0].0 != FULL_START {
        return false;
    }

    let mut expected_start = FULL_START;
    for (start, end) in ranges {
        if *start != expected_start {
            return false;
        }

        let Some(next_start) = increment_byte32(end) else {
            return *end == FULL_END;
        };
        expected_start = next_start;
    }

    false
}

fn increment_byte32(value: &[u8; 32]) -> Option<[u8; 32]> {
    let mut next = *value;
    for byte in next.iter_mut().rev() {
        if *byte == 0xff {
            *byte = 0;
        } else {
            *byte += 1;
            return Some(next);
        }
    }
    None
}
