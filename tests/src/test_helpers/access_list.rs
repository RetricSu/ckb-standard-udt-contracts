use crate::metadata_builders::build_access_list_shard_bytes;
use ckb_testtool::ckb_types::bytes::Bytes;

pub fn full_domain_shard(entries: Vec<[u8; 32]>) -> Bytes {
    build_access_list_shard_bytes([0u8; 32], [0xffu8; 32], entries)
}

pub fn empty_full_domain_shard() -> Bytes {
    full_domain_shard(Vec::new())
}

pub fn custom_shard(start: [u8; 32], end: [u8; 32], entries: Vec<[u8; 32]>) -> Bytes {
    build_access_list_shard_bytes(start, end, entries)
}

pub fn exact_shard(lock_hash: [u8; 32], entries: Vec<[u8; 32]>) -> Bytes {
    let mut start = [0u8; 32];
    start[0] = lock_hash[0] & 0xf0;
    let mut end = [0xffu8; 32];
    end[0] = start[0] | 0x0f;
    custom_shard(start, end, entries)
}

pub fn suffix_shard(start_last: u8, end_last: u8, entries: Vec<[u8; 32]>) -> Bytes {
    let mut start = [0u8; 32];
    let mut end = [0xffu8; 32];
    start[31] = start_last;
    end[31] = end_last;
    build_access_list_shard_bytes(start, end, entries)
}

pub fn bounded_suffix_shard(start_last: u8, end_last: u8, entries: Vec<[u8; 32]>) -> Bytes {
    let mut start = [0u8; 32];
    let mut end = [0u8; 32];
    start[31] = start_last;
    end[31] = end_last;
    build_access_list_shard_bytes(start, end, entries)
}

pub fn tail_suffix_shard(start_last: u8, entries: Vec<[u8; 32]>) -> Bytes {
    let mut start = [0u8; 32];
    start[31] = start_last;
    build_access_list_shard_bytes(start, [0xffu8; 32], entries)
}

pub fn prefix_start(first: u8) -> [u8; 32] {
    let mut start = [0u8; 32];
    start[0] = first;
    start
}

pub fn prefix_end(first: u8) -> [u8; 32] {
    let mut end = [0xffu8; 32];
    end[0] = first;
    end
}

pub fn entry(last: u8) -> [u8; 32] {
    let mut value = [0u8; 32];
    value[31] = last;
    value
}

pub fn prefix_entry(first: u8) -> [u8; 32] {
    let mut value = [0u8; 32];
    value[0] = first;
    value
}

pub fn numbered_entries(count: u16) -> Vec<[u8; 32]> {
    (0..count)
        .map(|number| {
            let mut value = [0u8; 32];
            value[..2].copy_from_slice(&number.to_be_bytes());
            value
        })
        .collect()
}
