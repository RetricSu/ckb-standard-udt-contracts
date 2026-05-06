use alloc::vec::Vec;

use ckb_std::{ckb_constants::Source, error::SysError, high_level::load_cell_data};
use standard_udt_types::metadata::AccessListShard;

use crate::{error::Error, mode::AccessMode};

const FULL_START: [u8; 32] = [0u8; 32];
const FULL_END: [u8; 32] = [0xffu8; 32];

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

    let input_start = shard_start(input_shards.first().ok_or(Error::InvalidShardSet)?);
    let input_end = shard_end(input_shards.last().ok_or(Error::InvalidShardSet)?);
    let output_start = shard_start(output_shards.first().ok_or(Error::InvalidShardSet)?);
    let output_end = shard_end(output_shards.last().ok_or(Error::InvalidShardSet)?);

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

    let mut expected_start = shard_start(&shards[0]);
    for shard in shards {
        if shard_start(shard) != expected_start {
            return Err(Error::InvalidShardSet);
        }

        let Some(next_start) = increment_byte32(&shard_end(shard)) else {
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
            .all(|(input, output)| {
                shard_start(input) == shard_start(output) && shard_end(input) == shard_end(output)
            })
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
        if shard_start(&input_shards[input_index]) != shard_start(&output_shards[output_index]) {
            return Err(Error::InvalidShardSet);
        }

        if shard_end(&input_shards[input_index]) == shard_end(&output_shards[output_index]) {
            input_index += 1;
            output_index += 1;
            continue;
        }

        let input_start = input_index;
        let output_start = output_index;
        let mut input_end = shard_end(&input_shards[input_index]);
        let mut output_end = shard_end(&output_shards[output_index]);

        loop {
            match input_end.cmp(&output_end) {
                core::cmp::Ordering::Less => {
                    input_index += 1;
                    if input_index >= input_shards.len() {
                        return Err(Error::InvalidShardSet);
                    }
                    input_end = shard_end(&input_shards[input_index]);
                }
                core::cmp::Ordering::Greater => {
                    output_index += 1;
                    if output_index >= output_shards.len() {
                        return Err(Error::InvalidShardSet);
                    }
                    output_end = shard_end(&output_shards[output_index]);
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
    AccessListShard::from_slice(data).map_err(|_| Error::InvalidShardData)
}

fn validate_ordered_non_overlapping(shards: &[AccessListShard]) -> Result<(), Error> {
    for pair in shards.windows(2) {
        if shard_start(&pair[1]) <= shard_end(&pair[0]) {
            return Err(Error::InvalidShardSet);
        }
    }
    Ok(())
}

fn validate_full_domain(shards: &[AccessListShard]) -> Result<(), Error> {
    if shards.is_empty() || shard_start(&shards[0]) != FULL_START {
        return Err(Error::InvalidShardSet);
    }

    let mut expected_start = FULL_START;
    for shard in shards {
        if shard_start(shard) != expected_start {
            return Err(Error::InvalidShardSet);
        }

        let Some(next_start) = increment_byte32(&shard_end(shard)) else {
            return if shard_end(shard) == FULL_END {
                Ok(())
            } else {
                Err(Error::InvalidShardSet)
            };
        };
        expected_start = next_start;
    }

    Err(Error::InvalidShardSet)
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

fn shard_start(shard: &AccessListShard) -> [u8; 32] {
    shard.range.start
}

fn shard_end(shard: &AccessListShard) -> [u8; 32] {
    shard.range.end
}
