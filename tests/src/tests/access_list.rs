use crate::{
    fixtures::{cell_dep_for_script, create_typed_cell, expect_tx_fail, typed_output},
    metadata_builders::{
        build_access_list_shard_bytes, build_xudt_meta_bytes, input_lock_authority, script_hash,
        DeployedScript,
    },
    Loader,
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
    deploy_data2_script(context, "enhanced-xudt-meta", Bytes::from(vec![2u8; 32]))
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

fn full_domain_shard(entries: Vec<[u8; 32]>) -> Bytes {
    build_access_list_shard_bytes([0u8; 32], [0xffu8; 32], entries)
}

struct AccessListCase {
    context: Context,
    tx: TransactionView,
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
