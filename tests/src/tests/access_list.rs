use crate::{
    fixtures::{
        cell_dep_for_script, create_typed_cell, expect_tx_fail, expect_tx_pass, typed_output,
    },
    metadata_builders::{
        build_access_list_shard_bytes, build_xudt_meta_bytes, input_lock_authority, script_hash,
        DeployedScript,
    },
    verify_and_dump_failed_tx, Loader,
};
use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_types::{
        bytes::Bytes,
        core::{ScriptHashType, TransactionBuilder, TransactionView},
        packed::CellInput,
        prelude::*,
    },
    context::Context,
};
use standard_udt_types::metadata::{CONFIG_ACCESS_ENABLED, CONFIG_ACCESS_WHITELIST};

fn deploy_data2_script(context: &mut Context, binary_name: &str, args: Bytes) -> DeployedScript {
    let out_point = context.deploy_cell(Loader::default().load_binary(binary_name));
    let script = context
        .build_script_with_hash_type(&out_point, ScriptHashType::Data2, args)
        .expect("build deployed Data2 script");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

fn always_success_lock(context: &mut Context, args: Bytes) -> DeployedScript {
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

fn meta_script(context: &mut Context) -> DeployedScript {
    deploy_data2_script(context, "xudt-meta", Bytes::from(vec![2u8; 32]))
}

fn access_list_script(context: &mut Context, meta_type_hash: [u8; 32]) -> DeployedScript {
    deploy_data2_script(context, "access-list", Bytes::from(meta_type_hash.to_vec()))
}

fn xudt_meta_data(config_flags: u8, authority: &DeployedScript) -> Bytes {
    build_xudt_meta_bytes(
        config_flags,
        0,
        None,
        None,
        Some(input_lock_authority(authority.script_hash)),
        Vec::new(),
    )
}

fn shard(start_last: u8, end_last: u8, entries: Vec<[u8; 32]>) -> Bytes {
    let mut start = [0u8; 32];
    let mut end = [0xffu8; 32];
    start[31] = start_last;
    end[31] = end_last;
    build_access_list_shard_bytes(start, end, entries)
}

fn bounded_shard(start_last: u8, end_last: u8, entries: Vec<[u8; 32]>) -> Bytes {
    let mut start = [0u8; 32];
    let mut end = [0u8; 32];
    start[31] = start_last;
    end[31] = end_last;
    build_access_list_shard_bytes(start, end, entries)
}

fn tail_shard(start_last: u8, entries: Vec<[u8; 32]>) -> Bytes {
    let mut start = [0u8; 32];
    start[31] = start_last;
    build_access_list_shard_bytes(start, [0xffu8; 32], entries)
}

fn custom_shard(start: [u8; 32], end: [u8; 32], entries: Vec<[u8; 32]>) -> Bytes {
    build_access_list_shard_bytes(start, end, entries)
}

fn full_domain_shard(entries: Vec<[u8; 32]>) -> Bytes {
    build_access_list_shard_bytes([0u8; 32], [0xffu8; 32], entries)
}

fn entry(last: u8) -> [u8; 32] {
    let mut value = [0u8; 32];
    value[31] = last;
    value
}

fn numbered_entries(count: u16) -> Vec<[u8; 32]> {
    (0..count)
        .map(|number| {
            let mut value = [0u8; 32];
            value[..2].copy_from_slice(&number.to_be_bytes());
            value
        })
        .collect()
}

struct AccessListCase {
    context: Context,
    tx: TransactionView,
}

fn expect_tx_pass_with_cycles(context: &Context, tx: &TransactionView, max_cycles: u64) {
    verify_and_dump_failed_tx(context, tx, max_cycles).expect("tx should pass");
}

fn access_list_update_tx(
    config_flags: u8,
    include_authority_input: bool,
    input_shards: Vec<Bytes>,
    output_shards: Vec<Bytes>,
) -> AccessListCase {
    let mut context = Context::default();
    let authority = always_success_lock(&mut context, Bytes::from(vec![1u8]));
    let cell_lock = always_success_lock(&mut context, Bytes::from(vec![2u8]));
    let meta = meta_script(&mut context);
    let access_list = access_list_script(&mut context, meta.script_hash);
    let meta_data = xudt_meta_data(config_flags, &authority);

    let meta_out_point = create_typed_cell(
        &mut context,
        &cell_lock.script,
        &meta.script,
        100_000_000_000,
        meta_data,
    );
    let mut builder = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(meta_out_point)
                .build(),
        )
        .output(typed_output(
            &cell_lock.script,
            &meta.script,
            100_000_000_000,
        ))
        .output_data(xudt_meta_data(config_flags, &authority).pack())
        .cell_dep(cell_dep_for_script(&cell_lock))
        .cell_dep(cell_dep_for_script(&authority))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&access_list));

    if include_authority_input {
        let out_point = context.create_cell(
            ckb_testtool::ckb_types::packed::CellOutput::new_builder()
                .capacity(100_000_000_000u64.pack())
                .lock(authority.script.clone())
                .build(),
            Bytes::new(),
        );
        builder = builder.input(CellInput::new_builder().previous_output(out_point).build());
    }

    for data in input_shards {
        let out_point = create_typed_cell(
            &mut context,
            &cell_lock.script,
            &access_list.script,
            100_000_000_000,
            data,
        );
        builder = builder.input(CellInput::new_builder().previous_output(out_point).build());
    }

    for data in output_shards {
        builder = builder
            .output(typed_output(
                &cell_lock.script,
                &access_list.script,
                100_000_000_000,
            ))
            .output_data(data.pack());
    }

    let tx = context.complete_tx(builder.build());
    AccessListCase { context, tx }
}

#[test]
fn access_list_blacklist_requires_full_domain_coverage() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(Vec::new())],
        vec![shard(0x00, 0x7f, Vec::new())],
    );

    expect_tx_fail(&case.context, &case.tx);
}

#[test]
fn access_list_rejects_overlapping_shards() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(Vec::new())],
        vec![shard(0x00, 0x8f, Vec::new()), shard(0x80, 0xff, Vec::new())],
    );

    expect_tx_fail(&case.context, &case.tx);
}

#[test]
fn access_list_rejects_unauthorized_update() {
    let mut listed = [0u8; 32];
    listed[31] = 0x10;
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        false,
        vec![full_domain_shard(Vec::new())],
        vec![full_domain_shard(vec![listed])],
    );

    expect_tx_fail(&case.context, &case.tx);
}

#[test]
fn access_list_whitelist_missing_coverage_is_fail_closed_for_xudt() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![shard(0x00, 0x0f, Vec::new())],
        Vec::new(),
    );

    expect_tx_fail(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_allows_same_range_insert_delete() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(vec![entry(0x10)])],
        vec![full_domain_shard(vec![entry(0x10), entry(0x20)])],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_allows_split_preserving_entries() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(vec![entry(0x08), entry(0x20)])],
        vec![
            bounded_shard(0x00, 0x0f, vec![entry(0x08)]),
            tail_shard(0x10, vec![entry(0x20)]),
        ],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_rejects_split_that_changes_entries() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(vec![entry(0x08)])],
        vec![
            bounded_shard(0x00, 0x0f, vec![entry(0x08)]),
            tail_shard(0x10, vec![entry(0x20)]),
        ],
    );

    expect_tx_fail(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_rejects_boundary_rewrite_with_entry_changes() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![
            bounded_shard(0x00, 0x0f, vec![entry(0x08)]),
            tail_shard(0x10, vec![entry(0x20)]),
        ],
        vec![
            bounded_shard(0x00, 0x1f, vec![entry(0x08), entry(0x18)]),
            tail_shard(0x20, vec![entry(0x20)]),
        ],
    );

    expect_tx_fail(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_allows_large_split_preserving_entries() {
    let entries = numbered_entries(4096);
    let mut first_half_end = [0xffu8; 32];
    first_half_end[0] = 0x7f;
    let mut second_half_start = [0u8; 32];
    second_half_start[0] = 0x80;

    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(entries.clone())],
        vec![
            custom_shard([0u8; 32], first_half_end, entries),
            custom_shard(second_half_start, [0xffu8; 32], Vec::new()),
        ],
    );

    expect_tx_pass_with_cycles(&case.context, &case.tx, 100_000_000);
}
