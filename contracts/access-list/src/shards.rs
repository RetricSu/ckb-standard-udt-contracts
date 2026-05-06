use alloc::vec::Vec;

use ckb_std::{ckb_constants::Source, error::SysError, high_level::load_cell_data};

use crate::{error::Error, mode::AccessMode};

const ACCESS_LIST_SHARD_FIELDS: usize = 2;
const MAX_ACCESSLIST_ENTRIES: usize = 4096;
const FULL_START: [u8; 32] = [0u8; 32];
const FULL_END: [u8; 32] = [0xffu8; 32];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessListShard {
    pub start: [u8; 32],
    pub end: [u8; 32],
    pub entries: Vec<[u8; 32]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AccessListLifecycle {
    Create,
    Update,
    Destroy,
    Replace,
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

pub fn validate_shards_for_modes(
    input_mode: AccessMode,
    output_mode: AccessMode,
    input_shards: &[AccessListShard],
    output_shards: &[AccessListShard],
) -> Result<(), Error> {
    validate_ordered_non_overlapping(input_shards)?;
    validate_ordered_non_overlapping(output_shards)?;

    match classify_lifecycle(input_mode, output_mode)? {
        AccessListLifecycle::Create => {
            if !input_shards.is_empty() {
                return Err(Error::InvalidShardSet);
            }
            validate_full_domain(output_shards)
        }
        AccessListLifecycle::Destroy => {
            validate_full_domain(input_shards)?;
            if output_shards.is_empty() {
                Ok(())
            } else {
                Err(Error::InvalidShardSet)
            }
        }
        AccessListLifecycle::Replace => {
            validate_full_domain(input_shards)?;
            validate_full_domain(output_shards)
        }
        AccessListLifecycle::Update => {
            validate_local_replacement_range(input_shards, output_shards)?;
            validate_update_diff(input_shards, output_shards)
        }
    }
}

fn classify_lifecycle(
    input_mode: AccessMode,
    output_mode: AccessMode,
) -> Result<AccessListLifecycle, Error> {
    match (is_active(input_mode), is_active(output_mode)) {
        (false, false) => Err(Error::InvalidShardSet),
        (false, true) => Ok(AccessListLifecycle::Create),
        (true, false) => Ok(AccessListLifecycle::Destroy),
        (true, true) if input_mode == output_mode => Ok(AccessListLifecycle::Update),
        (true, true) => Ok(AccessListLifecycle::Replace),
    }
}

fn is_active(mode: AccessMode) -> bool {
    mode != AccessMode::Disabled
}

fn validate_update_diff(
    input_shards: &[AccessListShard],
    output_shards: &[AccessListShard],
) -> Result<(), Error> {
    if have_identical_ranges(input_shards, output_shards) {
        return Ok(());
    }

    if !entries_equal(input_shards, output_shards) {
        return Err(Error::InvalidShardSet);
    }

    validate_split_merge_boundaries(input_shards, output_shards)
}

fn validate_local_replacement_range(
    input_shards: &[AccessListShard],
    output_shards: &[AccessListShard],
) -> Result<(), Error> {
    validate_contiguous_local_range(input_shards)?;
    validate_contiguous_local_range(output_shards)?;

    let input_start = input_shards.first().ok_or(Error::InvalidShardSet)?.start;
    let input_end = input_shards.last().ok_or(Error::InvalidShardSet)?.end;
    let output_start = output_shards.first().ok_or(Error::InvalidShardSet)?.start;
    let output_end = output_shards.last().ok_or(Error::InvalidShardSet)?.end;

    if input_start == output_start && input_end == output_end {
        Ok(())
    } else {
        Err(Error::InvalidShardSet)
    }
}

fn validate_contiguous_local_range(shards: &[AccessListShard]) -> Result<(), Error> {
    if shards.is_empty() {
        return Err(Error::InvalidShardSet);
    }

    let mut expected_start = shards[0].start;
    for shard in shards {
        if shard.start != expected_start {
            return Err(Error::InvalidShardSet);
        }

        let Some(next_start) = increment_byte32(&shard.end) else {
            return Ok(());
        };
        expected_start = next_start;
    }

    Ok(())
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

fn entries_equal(input_shards: &[AccessListShard], output_shards: &[AccessListShard]) -> bool {
    let mut input_shard_index = 0;
    let mut input_entry_index = 0;
    let mut output_shard_index = 0;
    let mut output_entry_index = 0;

    loop {
        let input = next_entry(input_shards, &mut input_shard_index, &mut input_entry_index);
        let output = next_entry(
            output_shards,
            &mut output_shard_index,
            &mut output_entry_index,
        );

        match (input, output) {
            (Some(input), Some(output)) if input == output => {}
            (None, None) => return true,
            _ => return false,
        }
    }
}

fn next_entry<'a>(
    shards: &'a [AccessListShard],
    shard_index: &mut usize,
    entry_index: &mut usize,
) -> Option<&'a [u8; 32]> {
    while *shard_index < shards.len() {
        let entries = &shards[*shard_index].entries;
        if *entry_index < entries.len() {
            let entry = &entries[*entry_index];
            *entry_index += 1;
            return Some(entry);
        }

        *shard_index += 1;
        *entry_index = 0;
    }

    None
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
        if let Some(previous_entry) = previous
            && entry <= previous_entry
        {
            return Err(Error::InvalidShardData);
        }
        previous = Some(entry);
        entries.push(entry);
    }

    Ok(entries)
}

fn is_nibble_aligned_range(start: &[u8; 32], end: &[u8; 32]) -> bool {
    is_nibble_aligned_start(start) && is_nibble_aligned_end(end)
}

fn is_nibble_aligned_start(start: &[u8; 32]) -> bool {
    start[0] & 0x0f == 0x00 && start[1..].iter().all(|byte| *byte == 0x00)
}

fn is_nibble_aligned_end(end: &[u8; 32]) -> bool {
    end[0] & 0x0f == 0x0f && end[1..].iter().all(|byte| *byte == 0xff)
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
