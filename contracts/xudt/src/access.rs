use alloc::vec::Vec;

use ckb_std::{
    ckb_constants::Source,
    ckb_types::{core::ScriptHashType, prelude::*},
    error::SysError,
    high_level::{load_cell_data, load_cell_lock_hash, load_cell_type},
};

use crate::{
    config::ACCESS_LIST_CODE_HASH,
    error::Error,
    meta::{self, ParsedXudtMeta},
};

const ACCESS_LIST_SHARD_FIELDS: usize = 2;
const MAX_ACCESSLIST_ENTRIES: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq)]
struct AccessListShard {
    start: [u8; 32],
    end: [u8; 32],
    entries: Vec<[u8; 32]>,
}

pub fn validate_if_enabled(meta_type_hash: &[u8; 32], meta: &ParsedXudtMeta) -> Result<(), Error> {
    if !meta.is_access_enabled() {
        return Ok(());
    }

    let shards = collect_visible_shards(meta_type_hash)?;

    let mut index = 0;
    loop {
        match load_cell_lock_hash(index, Source::GroupInput) {
            Ok(lock_hash) => validate_lock_hash(meta.is_whitelist(), lock_hash, &shards)?,
            Err(SysError::IndexOutOfBound) => return Ok(()),
            Err(error) => return Err(error.into()),
        }
        index += 1;
    }
}

fn validate_lock_hash(
    whitelist: bool,
    lock_hash: [u8; 32],
    shards: &[AccessListShard],
) -> Result<(), Error> {
    if whitelist {
        validate_membership_proof(&lock_hash, shards)
    } else {
        validate_non_membership_proof(&lock_hash, shards)
    }
}

fn validate_non_membership_proof(
    lock_hash: &[u8; 32],
    shards: &[AccessListShard],
) -> Result<(), Error> {
    let mut covered = false;
    for shard in shards {
        if !shard_covers(shard, lock_hash) {
            continue;
        }
        covered = true;
        if shard.entries.binary_search(lock_hash).is_ok() {
            return Err(Error::AccessDenied);
        }
    }

    if covered {
        Ok(())
    } else {
        Err(Error::InvalidShardData)
    }
}

fn validate_membership_proof(
    lock_hash: &[u8; 32],
    shards: &[AccessListShard],
) -> Result<(), Error> {
    for shard in shards {
        if shard_covers(shard, lock_hash) && shard.entries.binary_search(lock_hash).is_ok() {
            return Ok(());
        }
    }
    Err(Error::AccessDenied)
}

fn shard_covers(shard: &AccessListShard, lock_hash: &[u8; 32]) -> bool {
    shard.start <= *lock_hash && *lock_hash <= shard.end
}

fn collect_visible_shards(meta_type_hash: &[u8; 32]) -> Result<Vec<AccessListShard>, Error> {
    let mut shards = Vec::new();
    for source in [Source::CellDep, Source::Input] {
        collect_shards_from_source(meta_type_hash, source, &mut shards)?;
    }
    shards.sort_by(|left, right| left.start.cmp(&right.start).then(left.end.cmp(&right.end)));
    Ok(shards)
}

fn collect_shards_from_source(
    meta_type_hash: &[u8; 32],
    source: Source,
    shards: &mut Vec<AccessListShard>,
) -> Result<(), Error> {
    let mut index = 0;

    loop {
        match load_cell_type(index, source) {
            Ok(Some(type_script)) if is_access_list_script(&type_script, meta_type_hash) => {
                let data = load_cell_data(index, source).map_err(Error::from)?;
                shards.push(parse_access_list_shard(&data)?);
                index += 1;
            }
            Ok(_) => index += 1,
            Err(SysError::IndexOutOfBound) => return Ok(()),
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
    let offsets = meta::table_offsets(data, ACCESS_LIST_SHARD_FIELDS)?;
    if offsets[1] != offsets[0] + 64 {
        return Err(Error::InvalidShardData);
    }

    let start = meta::byte32_field(data, offsets[0], offsets[0] + 32)?;
    let end = meta::byte32_field(data, offsets[0] + 32, offsets[1])?;
    if start > end {
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

fn parse_byte32_vec(data: &[u8]) -> Result<Vec<[u8; 32]>, Error> {
    if data.len() < 4 {
        return Err(Error::InvalidShardData);
    }

    let count = meta::read_u32(data, 0)? as usize;
    if count > MAX_ACCESSLIST_ENTRIES || data.len() != 4 + count * 32 {
        return Err(Error::InvalidShardData);
    }

    let mut entries = Vec::with_capacity(count);
    let mut previous = None;
    for index in 0..count {
        let start = 4 + index * 32;
        let entry = meta::byte32_field(data, start, start + 32)?;
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
