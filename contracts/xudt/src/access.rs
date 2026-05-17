use alloc::vec::Vec;

use ckb_std::{
    ckb_constants::Source,
    error::SysError,
    high_level::{load_cell_data, load_cell_lock_hash, load_cell_type_hash},
    syscalls,
};

use crate::{config::ACCESS_LIST_CODE_HASH, error::Error, meta};
use standard_udt_script_utils::cells::bound_type_hash;
use standard_udt_types::metadata::{AccessListShard, XudtMeta};

const LOCK_BATCH_SIZE: usize = 64;
const ACCESS_LIST_HEADER_SIZE: usize = 12;
const ACCESS_LIST_RANGE_SIZE: usize = 64;
const ACCESS_LIST_RANGE_PREFIX_SIZE: usize = ACCESS_LIST_HEADER_SIZE + ACCESS_LIST_RANGE_SIZE;

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
    if !meta::is_access_enabled(meta_data) {
        return Ok(());
    }

    validate_checked_locks(meta_type_hash, meta::is_whitelist(meta_data), checked_locks)
}

fn validate_checked_locks(
    meta_type_hash: &[u8; 32],
    whitelist: bool,
    checked_locks: CheckedLocks,
) -> Result<(), Error> {
    AccessVerifier::new(meta_type_hash, whitelist, checked_locks)?.validate()
}

struct AccessVerifier {
    whitelist: bool,
    checked_locks: CheckedLocks,
    shard_index: Vec<ShardIndex>,
    locks: Vec<[u8; 32]>,
    cached_shards: [Option<CachedShard>; 2],
}

struct CachedShard {
    dep_index: usize,
    shard: AccessListShard,
}

impl AccessVerifier {
    fn new(
        meta_type_hash: &[u8; 32],
        whitelist: bool,
        checked_locks: CheckedLocks,
    ) -> Result<Self, Error> {
        Ok(Self {
            whitelist,
            checked_locks,
            shard_index: build_shard_index(meta_type_hash)?,
            locks: Vec::new(),
            cached_shards: [None, None],
        })
    }

    fn validate(&mut self) -> Result<(), Error> {
        for source in self.checked_sources().iter().copied() {
            self.collect_batched_from_source(source)?;
        }

        self.flush_batch()
    }

    fn collect_batched_from_source(&mut self, source: Source) -> Result<(), Error> {
        let mut index = 0;
        loop {
            match load_cell_lock_hash(index, source) {
                Ok(lock_hash) => {
                    self.locks.push(lock_hash);
                    if self.locks.len() == LOCK_BATCH_SIZE {
                        self.flush_batch()?;
                    }
                    index += 1;
                }
                Err(SysError::IndexOutOfBound) => return Ok(()),
                Err(error) => return Err(error.into()),
            }
        }
    }

    fn flush_batch(&mut self) -> Result<(), Error> {
        self.validate_lock_batch()?;
        self.locks.clear();
        Ok(())
    }

    fn checked_sources(&self) -> &'static [Source] {
        match self.checked_locks {
            CheckedLocks::Outputs => &[Source::GroupOutput],
            CheckedLocks::InputsAndOutputs => &[Source::GroupInput, Source::GroupOutput],
        }
    }

    fn validate_lock_batch(&mut self) -> Result<(), Error> {
        if self.locks.is_empty() {
            return Ok(());
        }

        self.locks.sort();
        self.locks.dedup();

        let mut lock_index = 0;
        let mut shard_cursor = 0;

        while lock_index < self.locks.len() {
            let lock_hash = self.locks[lock_index];
            while shard_cursor < self.shard_index.len()
                && self.shard_index[shard_cursor].end < lock_hash
            {
                shard_cursor += 1;
            }

            if shard_cursor >= self.shard_index.len()
                || lock_hash < self.shard_index[shard_cursor].start
            {
                return if self.whitelist {
                    Err(Error::AccessDenied)
                } else {
                    Err(Error::InvalidShardData)
                };
            }

            let current = self.shard_index[shard_cursor];
            while lock_index < self.locks.len()
                && shard_covers_index(&current, &self.locks[lock_index])
            {
                let lock_hash = self.locks[lock_index];
                let member = self.shard_contains(current.dep_index, &lock_hash)?;
                if self.whitelist && !member {
                    return Err(Error::AccessDenied);
                }
                if !self.whitelist && member {
                    return Err(Error::AccessDenied);
                }
                lock_index += 1;
            }
        }

        Ok(())
    }

    fn shard_contains(&mut self, dep_index: usize, lock_hash: &[u8; 32]) -> Result<bool, Error> {
        if self.cached_shards[0]
            .as_ref()
            .is_some_and(|cached| cached.dep_index == dep_index)
        {
            return Ok(self.cached_shards[0]
                .as_ref()
                .unwrap()
                .shard
                .entries
                .binary_search(lock_hash)
                .is_ok());
        }
        if self.cached_shards[1]
            .as_ref()
            .is_some_and(|cached| cached.dep_index == dep_index)
        {
            self.cached_shards.swap(0, 1);
            return Ok(self.cached_shards[0]
                .as_ref()
                .unwrap()
                .shard
                .entries
                .binary_search(lock_hash)
                .is_ok());
        }

        let data = load_cell_data(dep_index, Source::CellDep).map_err(Error::from)?;
        let shard = parse_access_list_shard(&data)?;
        self.cached_shards[1] = self.cached_shards[0].take();
        self.cached_shards[0] = Some(CachedShard { dep_index, shard });
        Ok(self.cached_shards[0]
            .as_ref()
            .unwrap()
            .shard
            .entries
            .binary_search(lock_hash)
            .is_ok())
    }
}

fn shard_covers_index(shard: &ShardIndex, lock_hash: &[u8; 32]) -> bool {
    shard.start <= *lock_hash && *lock_hash <= shard.end
}

fn build_shard_index(meta_type_hash: &[u8; 32]) -> Result<Vec<ShardIndex>, Error> {
    let mut indexes = Vec::new();
    let mut previous_end = None;
    let expected_type_hash = bound_type_hash(meta_type_hash, &ACCESS_LIST_CODE_HASH);
    let mut index = 0;

    loop {
        match load_cell_type_hash(index, Source::CellDep) {
            Ok(Some(type_hash)) if type_hash == expected_type_hash => {
                let (start, end) = load_access_list_range(index)?;
                if let Some(previous) = previous_end
                    && start <= previous
                {
                    return Err(Error::InvalidShardData);
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

fn parse_access_list_shard(data: &[u8]) -> Result<AccessListShard, Error> {
    AccessListShard::from_slice(data).map_err(|_| Error::InvalidShardData)
}

fn load_access_list_range(index: usize) -> Result<([u8; 32], [u8; 32]), Error> {
    let mut prefix = [0u8; ACCESS_LIST_RANGE_PREFIX_SIZE];
    let data_len = load_access_list_range_prefix(index, &mut prefix)?;

    let total_size = read_u32(&prefix, 0)? as usize;
    let range_offset = read_u32(&prefix, 4)? as usize;
    let entries_offset = read_u32(&prefix, 8)? as usize;
    if total_size != data_len
        || range_offset < ACCESS_LIST_HEADER_SIZE
        || entries_offset < range_offset
        || entries_offset - range_offset != ACCESS_LIST_RANGE_SIZE
        || entries_offset > data_len
        || entries_offset > prefix.len()
    {
        return Err(Error::InvalidShardData);
    }

    let range = &prefix[range_offset..entries_offset];

    let mut start = [0u8; 32];
    start.copy_from_slice(&range[..32]);
    let mut end = [0u8; 32];
    end.copy_from_slice(&range[32..]);
    if start > end {
        return Err(Error::InvalidShardData);
    }

    Ok((start, end))
}

fn load_access_list_range_prefix(index: usize, buf: &mut [u8]) -> Result<usize, Error> {
    match syscalls::load_cell_data(buf, 0, index, Source::CellDep) {
        Ok(len) if len == buf.len() => Ok(len),
        Ok(_) => Err(Error::InvalidShardData),
        Err(SysError::LengthNotEnough(remaining_len)) if remaining_len >= buf.len() => {
            Ok(remaining_len)
        }
        Err(SysError::LengthNotEnough(_)) => Err(Error::InvalidShardData),
        Err(error) => Err(error.into()),
    }
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, Error> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or(Error::InvalidShardData)?;
    let mut value = [0u8; 4];
    value.copy_from_slice(bytes);
    Ok(u32::from_le_bytes(value))
}
