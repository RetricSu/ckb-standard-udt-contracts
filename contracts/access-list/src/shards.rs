use alloc::vec::Vec;

use ckb_std::{ckb_constants::Source, error::SysError, high_level::load_cell_data};

use crate::{error::Error, mode::AccessMode};

const ACCESS_LIST_SHARD_FIELDS: usize = 2;
const MAX_ACCESSLIST_ENTRIES: usize = 8192;
const FULL_START: [u8; 32] = [0u8; 32];
const FULL_END: [u8; 32] = [0xffu8; 32];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessListShard {
    pub start: [u8; 32],
    pub end: [u8; 32],
    pub entries: Vec<[u8; 32]>,
    raw: Vec<u8>,
}

pub fn collect_group_shards(source: Source) -> Result<Vec<AccessListShard>, Error> {
    let mut shards = Vec::new();
    let mut index = 0;

    loop {
        match load_cell_data(index, source) {
            Ok(data) => {
                shards.push(parse_access_list_shard(&data)?);
                index += 1;
            }
            Err(SysError::IndexOutOfBound) => return Ok(shards),
            Err(error) => return Err(error.into()),
        }
    }
}

pub fn validate_shards_for_mode(
    mode: AccessMode,
    input_shards: &[AccessListShard],
    output_shards: &[AccessListShard],
) -> Result<(), Error> {
    validate_ordered_non_overlapping(input_shards)?;
    validate_ordered_non_overlapping(output_shards)?;

    match mode {
        AccessMode::Disabled => {
            if output_shards.is_empty() {
                Ok(())
            } else {
                Err(Error::InvalidShardSet)
            }
        }
        AccessMode::Blacklist => {
            if !input_shards.is_empty() {
                validate_full_domain(input_shards)?;
            }
            validate_full_domain(output_shards)?;
            validate_blacklist_diff(input_shards, output_shards)
        }
        AccessMode::Whitelist => {
            if output_shards.is_empty() {
                Err(Error::InvalidShardSet)
            } else {
                Ok(())
            }
        }
    }
}

fn validate_blacklist_diff(
    input_shards: &[AccessListShard],
    output_shards: &[AccessListShard],
) -> Result<(), Error> {
    if input_shards.is_empty() || have_identical_ranges(input_shards, output_shards) {
        return Ok(());
    }

    if flatten_entries(input_shards) != flatten_entries(output_shards) {
        return Err(Error::InvalidShardSet);
    }

    validate_split_merge_boundaries(input_shards, output_shards)
}

fn have_identical_ranges(
    input_shards: &[AccessListShard],
    output_shards: &[AccessListShard],
) -> bool {
    input_shards.len() == output_shards.len()
        && input_shards
            .iter()
            .zip(output_shards)
            .all(|(input, output)| input.start == output.start && input.end == output.end)
}

fn flatten_entries(shards: &[AccessListShard]) -> Vec<[u8; 32]> {
    let mut entries = Vec::new();
    for shard in shards {
        entries.extend_from_slice(&shard.entries);
    }
    entries
}

fn validate_split_merge_boundaries(
    input_shards: &[AccessListShard],
    output_shards: &[AccessListShard],
) -> Result<(), Error> {
    let mut input_index = 0;
    let mut output_index = 0;

    while input_index < input_shards.len() && output_index < output_shards.len() {
        if input_shards[input_index].start != output_shards[output_index].start {
            return Err(Error::InvalidShardSet);
        }

        if input_shards[input_index].end == output_shards[output_index].end {
            input_index += 1;
            output_index += 1;
            continue;
        }

        let input_start = input_index;
        let output_start = output_index;
        let mut input_end = input_shards[input_index].end;
        let mut output_end = output_shards[output_index].end;

        loop {
            match input_end.cmp(&output_end) {
                core::cmp::Ordering::Less => {
                    input_index += 1;
                    if input_index >= input_shards.len() {
                        return Err(Error::InvalidShardSet);
                    }
                    input_end = input_shards[input_index].end;
                }
                core::cmp::Ordering::Greater => {
                    output_index += 1;
                    if output_index >= output_shards.len() {
                        return Err(Error::InvalidShardSet);
                    }
                    output_end = output_shards[output_index].end;
                }
                core::cmp::Ordering::Equal => break,
            }
        }

        let input_count = input_index - input_start + 1;
        let output_count = output_index - output_start + 1;
        let pure_split = input_count == 1 && output_count > 1;
        let pure_merge = input_count > 1 && output_count == 1;
        if !pure_split && !pure_merge {
            return Err(Error::InvalidShardSet);
        }

        input_index += 1;
        output_index += 1;
    }

    if input_index == input_shards.len() && output_index == output_shards.len() {
        Ok(())
    } else {
        Err(Error::InvalidShardSet)
    }
}

fn parse_access_list_shard(data: &[u8]) -> Result<AccessListShard, Error> {
    let offsets = table_offsets(data, ACCESS_LIST_SHARD_FIELDS)?;
    if offsets[1] != offsets[0] + 64 {
        return Err(Error::InvalidShardData);
    }

    let start = byte32_field(data, offsets[0], offsets[0] + 32)?;
    let end = byte32_field(data, offsets[0] + 32, offsets[1])?;
    if start > end || !is_nibble_aligned_range(&start, &end) {
        return Err(Error::InvalidShardData);
    }

    let entries = parse_byte32_vec(&data[offsets[1]..offsets[2]])?;
    for entry in &entries {
        if entry < &start || entry > &end {
            return Err(Error::InvalidShardData);
        }
    }

    Ok(AccessListShard {
        start,
        end,
        entries,
        raw: data.to_vec(),
    })
}

fn validate_ordered_non_overlapping(shards: &[AccessListShard]) -> Result<(), Error> {
    for pair in shards.windows(2) {
        if pair[1].start <= pair[0].end {
            return Err(Error::InvalidShardSet);
        }
    }
    Ok(())
}

fn validate_full_domain(shards: &[AccessListShard]) -> Result<(), Error> {
    if shards.is_empty() || shards[0].start != FULL_START {
        return Err(Error::InvalidShardSet);
    }

    let mut expected_start = FULL_START;
    for shard in shards {
        if shard.start != expected_start {
            return Err(Error::InvalidShardSet);
        }

        let Some(next_start) = increment_byte32(&shard.end) else {
            return if shard.end == FULL_END {
                Ok(())
            } else {
                Err(Error::InvalidShardSet)
            };
        };
        expected_start = next_start;
    }

    Err(Error::InvalidShardSet)
}

fn parse_byte32_vec(data: &[u8]) -> Result<Vec<[u8; 32]>, Error> {
    if data.len() < 4 {
        return Err(Error::InvalidShardData);
    }

    let count = read_u32(data, 0)? as usize;
    if count > MAX_ACCESSLIST_ENTRIES || data.len() != 4 + count * 32 {
        return Err(Error::InvalidShardData);
    }

    let mut entries = Vec::with_capacity(count);
    let mut previous = None;
    for index in 0..count {
        let start = 4 + index * 32;
        let entry = byte32_field(data, start, start + 32)?;
        if let Some(previous_entry) = previous {
            if entry <= previous_entry {
                return Err(Error::InvalidShardData);
            }
        }
        previous = Some(entry);
        entries.push(entry);
    }

    Ok(entries)
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

fn table_offsets(data: &[u8], fields: usize) -> Result<Vec<usize>, Error> {
    if data.len() < 4 + fields * 4 {
        return Err(Error::InvalidShardData);
    }

    let total_size = read_u32(data, 0)? as usize;
    if total_size != data.len() {
        return Err(Error::InvalidShardData);
    }

    let first_offset = read_u32(data, 4)? as usize;
    if first_offset != 4 + fields * 4 {
        return Err(Error::InvalidShardData);
    }

    let mut offsets = Vec::with_capacity(fields + 1);
    for index in 0..fields {
        offsets.push(read_u32(data, 4 + index * 4)? as usize);
    }
    offsets.push(total_size);

    for index in 1..offsets.len() {
        if offsets[index] < offsets[index - 1] || offsets[index] > total_size {
            return Err(Error::InvalidShardData);
        }
    }

    Ok(offsets)
}

fn byte32_field(data: &[u8], start: usize, end: usize) -> Result<[u8; 32], Error> {
    if end != start + 32 || end > data.len() {
        return Err(Error::InvalidShardData);
    }

    let mut raw = [0u8; 32];
    raw.copy_from_slice(&data[start..end]);
    Ok(raw)
}

fn read_u32(data: &[u8], start: usize) -> Result<u32, Error> {
    if start + 4 > data.len() {
        return Err(Error::InvalidShardData);
    }

    let mut raw = [0u8; 4];
    raw.copy_from_slice(&data[start..start + 4]);
    Ok(u32::from_le_bytes(raw))
}
