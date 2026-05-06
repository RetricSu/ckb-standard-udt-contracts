use crate::{
    metadata_builders::{
        build_access_list_shard_bytes, build_xudt_meta_bytes, script_hash, DeployedScript,
    },
    Loader,
};
use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_hash::new_blake2b,
    ckb_types::{bytes::Bytes, core::ScriptHashType, packed::CellInput, prelude::*},
    context::Context,
};
use standard_udt_types::metadata::{Authority, Extension};

pub fn deploy_data2_script(
    context: &mut Context,
    binary_name: &str,
    args: Bytes,
) -> DeployedScript {
    deploy_script(context, binary_name, ScriptHashType::Data2, args)
}

pub fn deploy_data_script(context: &mut Context, binary_name: &str, args: Bytes) -> DeployedScript {
    deploy_script(context, binary_name, ScriptHashType::Data, args)
}

pub fn deploy_script(
    context: &mut Context,
    binary_name: &str,
    hash_type: ScriptHashType,
    args: Bytes,
) -> DeployedScript {
    let out_point = context.deploy_cell(Loader::default().load_binary(binary_name));
    let script = context
        .build_script_with_hash_type(&out_point, hash_type, args)
        .expect("build deployed script");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

pub fn always_success_lock(context: &mut Context, args: Bytes) -> DeployedScript {
    let out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let script = context
        .build_script_with_hash_type(&out_point, ScriptHashType::Data2, args)
        .expect("build always-success lock");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

pub fn always_success_lock_empty(context: &mut Context) -> DeployedScript {
    always_success_lock(context, Bytes::new())
}

pub fn non_whitelisted_lock(context: &mut Context) -> DeployedScript {
    let out_point = context.deploy_cell(Bytes::from(vec![1u8]));
    let script = context
        .build_script_with_hash_type(&out_point, ScriptHashType::Data2, Bytes::new())
        .expect("build non-whitelisted lock");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

pub fn fake_data2_script(context: &mut Context, args_hash: [u8; 32]) -> DeployedScript {
    let out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let script = context
        .build_script_with_hash_type(
            &out_point,
            ScriptHashType::Data2,
            Bytes::from(args_hash.to_vec()),
        )
        .expect("build fake Data2 script");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

pub fn sudt_meta_script(context: &mut Context, args: Bytes) -> DeployedScript {
    deploy_data2_script(context, "sudt-meta", args)
}

pub fn xudt_meta_script(context: &mut Context) -> DeployedScript {
    deploy_data2_script(context, "xudt-meta", Bytes::from(vec![2u8; 32]))
}

pub fn sudt_script(context: &mut Context, meta_type_hash: [u8; 32]) -> DeployedScript {
    deploy_data2_script(context, "sudt", Bytes::from(meta_type_hash.to_vec()))
}

pub fn xudt_script(context: &mut Context, meta_type_hash: [u8; 32]) -> DeployedScript {
    deploy_data2_script(context, "xudt", Bytes::from(meta_type_hash.to_vec()))
}

pub fn access_list_script(context: &mut Context, meta_type_hash: [u8; 32]) -> DeployedScript {
    deploy_data2_script(context, "access-list", Bytes::from(meta_type_hash.to_vec()))
}

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
    custom_shard(lock_hash, lock_hash, entries)
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

pub fn calculate_type_id(input: &CellInput, output_index: u64) -> [u8; 32] {
    let mut type_id = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(input.as_slice());
    hasher.update(&output_index.to_le_bytes());
    hasher.finalize(&mut type_id);
    type_id
}

pub fn xudt_meta_data(
    config_flags: u8,
    current_supply: u128,
    mint_authority: Option<Authority>,
    extensions: Vec<Extension>,
) -> Bytes {
    xudt_meta_data_with_authorities(
        config_flags,
        current_supply,
        mint_authority,
        None,
        None,
        extensions,
    )
}

pub fn xudt_meta_data_with_authorities(
    config_flags: u8,
    current_supply: u128,
    mint_authority: Option<Authority>,
    metadata_authority: Option<Authority>,
    access_authority: Option<Authority>,
    extensions: Vec<Extension>,
) -> Bytes {
    build_xudt_meta_bytes(
        config_flags,
        current_supply,
        mint_authority,
        metadata_authority,
        access_authority,
        extensions,
    )
}
