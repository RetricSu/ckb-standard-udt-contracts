use alloc::vec::Vec;

use ckb_std::{
    ckb_constants::Source,
    ckb_types::packed::Script,
    error::SysError,
    high_level::{load_cell_data, load_cell_type},
};

use crate::{
    constants::ACCESS_LIST_CODE_HASH,
    error::Error,
    meta_cell::{
        parser::{byte32_field, read_u32, table_offsets},
        token::is_token_script,
    },
};

const ACCESS_LIST_SHARD_FIELDS: usize = 2;
const MAX_ACCESSLIST_ENTRIES: usize = 4096;

pub fn has_legal_access_list_shard(meta_type_hash: &[u8; 32]) -> Result<bool, Error> {
    Ok(!collect_legal_access_list_shards(meta_type_hash)?.is_empty())
}

pub fn has_full_domain_access_list_shards(meta_type_hash: &[u8; 32]) -> Result<bool, Error> {
    let shards = collect_legal_access_list_shards(meta_type_hash)?;
    if shards.is_empty() {
        return Ok(false);
    }

    if shards[0].start != [0u8; 32] {
        return Ok(false);
    }

    let mut expected_start = [0u8; 32];
    for shard in &shards {
        if shard.start != expected_start {
            return Ok(false);
        }
        let Some(next_start) = increment_byte32(&shard.end) else {
            return Ok(shard.end == [0xffu8; 32]);
        };
        expected_start = next_start;
    }

    Ok(false)
}

fn collect_legal_access_list_shards(
    meta_type_hash: &[u8; 32],
) -> Result<Vec<AccessListShard>, Error> {
    let mut shards = Vec::new();
    let mut index = 0;
    loop {
        match load_cell_type(index, Source::Output) {
            Ok(Some(script)) if is_access_list_script(&script, meta_type_hash) => {
                let data = load_cell_data(index, Source::Output)?;
                let shard = parse_access_list_shard(&data)?;
                validate_shard_order(&shards, &shard)?;
                shards.push(shard);
                index += 1;
            }
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(shards),
            Err(error) => return Err(error.into()),
        }
    }
}

fn is_access_list_script(type_script: &Script, meta_type_hash: &[u8; 32]) -> bool {
    is_token_script(type_script, meta_type_hash, &ACCESS_LIST_CODE_HASH)
}

#[derive(Clone)]
struct AccessListShard {
    start: [u8; 32],
    end: [u8; 32],
}

fn parse_access_list_shard(data: &[u8]) -> Result<AccessListShard, Error> {
    let offsets = table_offsets(data, ACCESS_LIST_SHARD_FIELDS, false)?;
    if offsets[1] != offsets[0] + 64 {
        return Err(Error::InvalidMetaData);
    }
    let start = byte32_field(data, offsets[0], offsets[0] + 32)?;
    let end = byte32_field(data, offsets[0] + 32, offsets[1])?;
    if start > end || !is_nibble_aligned_range(&start, &end) {
        return Err(Error::InvalidMetaData);
    }

    parse_byte32_vec(&data[offsets[1]..offsets[2]])?;
    Ok(AccessListShard { start, end })
}

fn parse_byte32_vec(data: &[u8]) -> Result<Vec<[u8; 32]>, Error> {
    if data.len() < 4 {
        return Err(Error::InvalidMetaData);
    }
    let count = read_u32(data, 0)? as usize;
    if count > MAX_ACCESSLIST_ENTRIES || data.len() != 4 + count * 32 {
        return Err(Error::InvalidMetaData);
    }

    let mut entries = Vec::with_capacity(count);
    let mut prev = None;
    for index in 0..count {
        let start = 4 + index * 32;
        let entry = byte32_field(data, start, start + 32)?;
        if let Some(prev_entry) = prev {
            if entry <= prev_entry {
                return Err(Error::InvalidMetaData);
            }
        }
        prev = Some(entry);
        entries.push(entry);
    }
    Ok(entries)
}

fn validate_shard_order(
    previous_shards: &[AccessListShard],
    shard: &AccessListShard,
) -> Result<(), Error> {
    if let Some(previous) = previous_shards.last() {
        if shard.start <= previous.end {
            return Err(Error::InvalidMetaData);
        }
    }
    Ok(())
}

fn is_nibble_aligned_range(start: &[u8; 32], end: &[u8; 32]) -> bool {
    start[31] & 0x0f == 0 && end[31] & 0x0f == 0x0f
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
