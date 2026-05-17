use alloc::vec::Vec;

use ckb_std::{
    ckb_constants::Source,
    ckb_types::{core::ScriptHashType, prelude::*},
    error::SysError,
    high_level::{load_cell_data, load_cell_lock_hash, load_cell_type},
};

use crate::{config::ACCESS_LIST_CODE_HASH, error::Error, meta};
use standard_udt_types::metadata::{AccessListShard, XudtMeta};

const SINGLE_BATCH_LOCK_LIMIT: usize = 64;
const LOCK_BATCH_SIZE: usize = 64;

#[derive(Clone, Copy)]
pub enum CheckedLocks {
    Outputs,
    InputsAndOutputs,
}

#[derive(Clone, Copy)]
struct ShardIndex {
    start: [u8; 32],
    end: [u8; 32],
    dep_index: usize,
}

pub fn validate_if_enabled(
    meta_type_hash: &[u8; 32],
    meta_data: &XudtMeta,
    checked_locks: CheckedLocks,
) -> Result<(), Error> {
    reject_same_meta_access_list_state_cells(meta_type_hash)?;
    if !meta::is_access_enabled(meta_data) {
        return Ok(());
    }

    validate_checked_locks(meta_type_hash, meta::is_whitelist(meta_data), checked_locks)
}

pub fn reject_same_meta_access_list_state_cells(meta_type_hash: &[u8; 32]) -> Result<(), Error> {
    for source in [Source::Input, Source::Output] {
        let mut index = 0;
        loop {
            match load_cell_type(index, source) {
                Ok(Some(type_script)) if is_access_list_script(&type_script, meta_type_hash) => {
                    return Err(Error::InvalidShardData);
                }
                Ok(_) => index += 1,
                Err(SysError::IndexOutOfBound) => break,
                Err(error) => return Err(error.into()),
            }
        }
    }

    Ok(())
}

fn validate_checked_locks(
    meta_type_hash: &[u8; 32],
    whitelist: bool,
    checked_locks: CheckedLocks,
) -> Result<(), Error> {
    let shard_index = build_shard_index(meta_type_hash)?;
    let lock_count = count_checked_locks(checked_locks)?;
    if lock_count <= SINGLE_BATCH_LOCK_LIMIT {
        let mut locks = Vec::new();
        collect_checked_locks(checked_locks, &mut locks)?;
        return validate_lock_batch(whitelist, &mut locks, &shard_index);
    }

    let mut locks = Vec::new();
    collect_checked_locks_batched(checked_locks, whitelist, &shard_index, &mut locks)
}

fn count_checked_locks(checked_locks: CheckedLocks) -> Result<usize, Error> {
    let mut count = 0;
    for source in checked_sources(checked_locks).iter().copied() {
        let mut index = 0;
        loop {
            match load_cell_lock_hash(index, source) {
                Ok(_) => {
                    count += 1;
                    index += 1;
                }
                Err(SysError::IndexOutOfBound) => break,
                Err(error) => return Err(error.into()),
            }
        }
    }

    Ok(count)
}

fn collect_checked_locks(
    checked_locks: CheckedLocks,
    locks: &mut Vec<[u8; 32]>,
) -> Result<(), Error> {
    for source in checked_sources(checked_locks).iter().copied() {
        collect_locks_from_source(source, locks)?;
    }

    Ok(())
}

fn collect_checked_locks_batched(
    checked_locks: CheckedLocks,
    whitelist: bool,
    shard_index: &[ShardIndex],
    locks: &mut Vec<[u8; 32]>,
) -> Result<(), Error> {
    for source in checked_sources(checked_locks).iter().copied() {
        let mut index = 0;
        loop {
            match load_cell_lock_hash(index, source) {
                Ok(lock_hash) => {
                    locks.push(lock_hash);
                    if locks.len() == LOCK_BATCH_SIZE {
                        validate_lock_batch(whitelist, locks, shard_index)?;
                        locks.clear();
                    }
                    index += 1;
                }
                Err(SysError::IndexOutOfBound) => break,
                Err(error) => return Err(error.into()),
            }
        }
    }

    validate_lock_batch(whitelist, locks, shard_index)
}

fn checked_sources(checked_locks: CheckedLocks) -> &'static [Source] {
    match checked_locks {
        CheckedLocks::Outputs => &[Source::GroupOutput],
        CheckedLocks::InputsAndOutputs => &[Source::GroupInput, Source::GroupOutput],
    }
}

fn collect_locks_from_source(source: Source, locks: &mut Vec<[u8; 32]>) -> Result<(), Error> {
    let mut index = 0;
    loop {
        match load_cell_lock_hash(index, source) {
            Ok(lock_hash) => {
                locks.push(lock_hash);
                index += 1;
            }
            Err(SysError::IndexOutOfBound) => return Ok(()),
            Err(error) => return Err(error.into()),
        }
    }
}

fn validate_lock_batch(
    whitelist: bool,
    locks: &mut Vec<[u8; 32]>,
    shard_index: &[ShardIndex],
) -> Result<(), Error> {
    if locks.is_empty() {
        return Ok(());
    }

    locks.sort();
    locks.dedup();

    let mut lock_index = 0;
    let mut shard_cursor = 0;

    while lock_index < locks.len() {
        let lock_hash = locks[lock_index];
        while shard_cursor < shard_index.len() && shard_index[shard_cursor].end < lock_hash {
            shard_cursor += 1;
        }

        if shard_cursor >= shard_index.len() || lock_hash < shard_index[shard_cursor].start {
            return if whitelist {
                Err(Error::AccessDenied)
            } else {
                Err(Error::InvalidShardData)
            };
        }

        let current = shard_index[shard_cursor];
        let data = load_cell_data(current.dep_index, Source::CellDep).map_err(Error::from)?;
        let shard = parse_access_list_shard(&data)?;
        while lock_index < locks.len() && shard_covers_index(&current, &locks[lock_index]) {
            let member = shard.entries.binary_search(&locks[lock_index]).is_ok();
            if whitelist && !member {
                return Err(Error::AccessDenied);
            }
            if !whitelist && member {
                return Err(Error::AccessDenied);
            }
            lock_index += 1;
        }
    }

    Ok(())
}

fn shard_covers_index(shard: &ShardIndex, lock_hash: &[u8; 32]) -> bool {
    shard.start <= *lock_hash && *lock_hash <= shard.end
}

fn build_shard_index(meta_type_hash: &[u8; 32]) -> Result<Vec<ShardIndex>, Error> {
    let mut indexes = Vec::new();
    let mut previous_end = None;
    let mut index = 0;

    loop {
        match load_cell_type(index, Source::CellDep) {
            Ok(Some(type_script)) if is_access_list_script(&type_script, meta_type_hash) => {
                let data = load_cell_data(index, Source::CellDep).map_err(Error::from)?;
                let (start, end) = parse_access_list_range(&data)?;
                if let Some(previous) = previous_end {
                    if start <= previous {
                        return Err(Error::InvalidShardData);
                    }
                }
                previous_end = Some(end);
                indexes.push(ShardIndex {
                    start,
                    end,
                    dep_index: index,
                });
                index += 1;
            }
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(indexes),
            Err(error) => return Err(error.into()),
        }
    }
}

fn is_access_list_script(
    type_script: &ckb_std::ckb_types::packed::Script,
    meta_type_hash: &[u8; 32],
) -> bool {
    let code_hash: [u8; 32] = type_script.code_hash().unpack();
    type_script.hash_type() == ScriptHashType::Data2.into()
        && type_script.args().raw_data().as_ref() == meta_type_hash
        && code_hash == ACCESS_LIST_CODE_HASH
}

fn parse_access_list_shard(data: &[u8]) -> Result<AccessListShard, Error> {
    AccessListShard::from_slice(data).map_err(|_| Error::InvalidShardData)
}

fn parse_access_list_range(data: &[u8]) -> Result<([u8; 32], [u8; 32]), Error> {
    if data.len() < 76 {
        return Err(Error::InvalidShardData);
    }

    let total_size = read_u32(data, 0)? as usize;
    let range_offset = read_u32(data, 4)? as usize;
    let entries_offset = read_u32(data, 8)? as usize;
    if total_size != data.len()
        || range_offset < 12
        || entries_offset < range_offset
        || entries_offset - range_offset != 64
        || entries_offset > data.len()
    {
        return Err(Error::InvalidShardData);
    }

    let mut start = [0u8; 32];
    start.copy_from_slice(&data[range_offset..range_offset + 32]);
    let mut end = [0u8; 32];
    end.copy_from_slice(&data[range_offset + 32..range_offset + 64]);
    if start > end {
        return Err(Error::InvalidShardData);
    }

    Ok((start, end))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, Error> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or(Error::InvalidShardData)?;
    let mut value = [0u8; 4];
    value.copy_from_slice(bytes);
    Ok(u32::from_le_bytes(value))
}
