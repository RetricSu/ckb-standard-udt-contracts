use crate::{
    fixtures::{
        cell_dep, cell_dep_for_script, create_typed_cell, expect_tx_fail, expect_tx_fail_with_code,
        expect_tx_pass, typed_output,
    },
    metadata_builders::{
        dynamic_linking_authority, input_lock_authority, spawn_authority, DeployedScript,
    },
    test_helpers::{
        access_list_script, always_success_lock, bounded_suffix_shard as bounded_shard,
        custom_shard, deploy_data2_script, deploy_data_script, entry, full_domain_shard,
        non_whitelisted_lock, numbered_entries, prefix_end, prefix_entry, prefix_start,
        tail_suffix_shard as tail_shard, xudt_meta_data_with_authorities as build_xudt_meta_data,
        xudt_meta_script as meta_script,
    },
    verify_and_dump_failed_tx,
};
use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        core::{TransactionBuilder, TransactionView},
        packed::CellInput,
        prelude::*,
    },
    context::Context,
};
use standard_udt_types::metadata::{Authority, CONFIG_ACCESS_ENABLED, CONFIG_ACCESS_WHITELIST};

fn xudt_meta_data(config_flags: u8, authority: &DeployedScript) -> Bytes {
    xudt_meta_data_with_authority(
        config_flags,
        Some(input_lock_authority(authority.script_hash)),
    )
}

fn xudt_meta_data_with_authority(config_flags: u8, authority: Option<Authority>) -> Bytes {
    build_xudt_meta_data(config_flags, 0, None, None, authority, Vec::new())
}

fn shard(start_last: u8, end_last: u8, entries: Vec<[u8; 32]>) -> Bytes {
    crate::test_helpers::suffix_shard(start_last, end_last, entries)
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

fn access_list_transition_tx(
    input_config_flags: u8,
    output_config_flags: u8,
    include_authority_input: bool,
    input_shards: Vec<Bytes>,
    output_shards: Vec<Bytes>,
) -> AccessListCase {
    let mut context = Context::default();
    let authority = always_success_lock(&mut context, Bytes::from(vec![1u8]));
    let cell_lock = always_success_lock(&mut context, Bytes::from(vec![2u8]));
    let meta = always_success_lock(&mut context, Bytes::from(vec![3u8; 32]));
    let access_list = access_list_script(&mut context, meta.script_hash);

    let meta_out_point = create_typed_cell(
        &mut context,
        &cell_lock.script,
        &meta.script,
        100_000_000_000,
        xudt_meta_data(input_config_flags, &authority),
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
        .output_data(xudt_meta_data(output_config_flags, &authority).pack())
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

fn access_list_update_tx_with_non_whitelisted_meta_lock(
    config_flags: u8,
    input_shards: Vec<Bytes>,
    output_shards: Vec<Bytes>,
) -> AccessListCase {
    let mut context = Context::default();
    let authority = always_success_lock(&mut context, Bytes::from(vec![1u8]));
    let cell_lock = always_success_lock(&mut context, Bytes::from(vec![2u8]));
    let meta_lock = non_whitelisted_lock(&mut context);
    let meta = meta_script(&mut context);
    let access_list = access_list_script(&mut context, meta.script_hash);
    let meta_data = xudt_meta_data(config_flags, &authority);

    let meta_out_point = create_typed_cell(
        &mut context,
        &meta_lock.script,
        &meta.script,
        100_000_000_000,
        meta_data,
    );
    let mut builder = TransactionBuilder::default()
        .cell_dep(cell_dep(meta_out_point))
        .cell_dep(cell_dep_for_script(&cell_lock))
        .cell_dep(cell_dep_for_script(&authority))
        .cell_dep(cell_dep_for_script(&meta_lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&access_list));

    let auth_out_point = context.create_cell(
        ckb_testtool::ckb_types::packed::CellOutput::new_builder()
            .capacity(100_000_000_000u64.pack())
            .lock(authority.script.clone())
            .build(),
        Bytes::new(),
    );
    builder = builder.input(
        CellInput::new_builder()
            .previous_output(auth_out_point)
            .build(),
    );

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

fn access_list_update_tx_with_non_whitelisted_output_lock(
    output_shards: Vec<Bytes>,
) -> AccessListCase {
    let mut context = Context::default();
    let authority = always_success_lock(&mut context, Bytes::from(vec![1u8]));
    let cell_lock = always_success_lock(&mut context, Bytes::from(vec![2u8]));
    let output_lock = non_whitelisted_lock(&mut context);
    let meta = meta_script(&mut context);
    let access_list = access_list_script(&mut context, meta.script_hash);
    let meta_data = xudt_meta_data(CONFIG_ACCESS_ENABLED, &authority);
    let meta_out_point = create_typed_cell(
        &mut context,
        &cell_lock.script,
        &meta.script,
        100_000_000_000,
        meta_data,
    );
    let auth_out_point = context.create_cell(
        ckb_testtool::ckb_types::packed::CellOutput::new_builder()
            .capacity(100_000_000_000u64.pack())
            .lock(authority.script.clone())
            .build(),
        Bytes::new(),
    );
    let input_out_point = create_typed_cell(
        &mut context,
        &cell_lock.script,
        &access_list.script,
        100_000_000_000,
        full_domain_shard(Vec::new()),
    );

    let mut builder = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(meta_out_point)
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(auth_out_point)
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .output(typed_output(
            &cell_lock.script,
            &meta.script,
            100_000_000_000,
        ))
        .output_data(xudt_meta_data(CONFIG_ACCESS_ENABLED, &authority).pack())
        .cell_dep(cell_dep_for_script(&cell_lock))
        .cell_dep(cell_dep_for_script(&authority))
        .cell_dep(cell_dep_for_script(&output_lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&access_list));

    for data in output_shards {
        builder = builder
            .output(typed_output(
                &output_lock.script,
                &access_list.script,
                100_000_000_000,
            ))
            .output_data(data.pack());
    }

    let tx = context.complete_tx(builder.build());
    AccessListCase { context, tx }
}

fn access_list_update_tx_with_plugin_authority(
    plugin_name: &str,
    spawn: bool,
    output_shards: Vec<Bytes>,
) -> AccessListCase {
    let mut context = Context::default();
    let plugin = if spawn {
        deploy_data2_script(&mut context, plugin_name, Bytes::from_static(b"allow"))
    } else {
        deploy_data_script(&mut context, plugin_name, Bytes::from_static(b"allow"))
    };
    let authority = if spawn {
        spawn_authority(&plugin)
    } else {
        dynamic_linking_authority(&plugin)
    };
    let cell_lock = always_success_lock(&mut context, Bytes::from(vec![2u8]));
    let meta = meta_script(&mut context);
    let access_list = access_list_script(&mut context, meta.script_hash);
    let meta_data = xudt_meta_data_with_authority(CONFIG_ACCESS_ENABLED, Some(authority.clone()));

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
        .output_data(xudt_meta_data_with_authority(CONFIG_ACCESS_ENABLED, Some(authority)).pack())
        .cell_dep(cell_dep_for_script(&cell_lock))
        .cell_dep(cell_dep_for_script(&plugin))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&access_list));

    let input_out_point = create_typed_cell(
        &mut context,
        &cell_lock.script,
        &access_list.script,
        100_000_000_000,
        full_domain_shard(Vec::new()),
    );
    builder = builder.input(
        CellInput::new_builder()
            .previous_output(input_out_point)
            .build(),
    );

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
fn access_list_disabled_to_disabled_rejects_access_list_inputs_or_outputs() {
    let with_input =
        access_list_transition_tx(0, 0, true, vec![full_domain_shard(Vec::new())], Vec::new());
    expect_tx_fail_with_code(&with_input.context, &with_input.tx, "error code 61");

    let with_output =
        access_list_transition_tx(0, 0, true, Vec::new(), vec![full_domain_shard(Vec::new())]);
    expect_tx_fail_with_code(&with_output.context, &with_output.tx, "error code 61");
}

#[test]
fn access_list_whitelist_create_requires_full_domain_outputs() {
    let partial = access_list_transition_tx(
        0,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        Vec::new(),
        vec![custom_shard([0u8; 32], prefix_end(0x7f), Vec::new())],
    );
    expect_tx_fail_with_code(&partial.context, &partial.tx, "error code 61");

    let full = access_list_transition_tx(
        0,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        Vec::new(),
        vec![full_domain_shard(Vec::new())],
    );
    expect_tx_pass(&full.context, &full.tx);
}

#[test]
fn access_list_whitelist_rejects_repeated_create_from_empty_inputs() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        Vec::new(),
        vec![full_domain_shard(Vec::new())],
    );

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 61");
}

#[test]
fn access_list_blacklist_rejects_repeated_create_from_empty_inputs() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED,
        true,
        Vec::new(),
        vec![full_domain_shard(Vec::new())],
    );

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 61");
}

#[test]
fn access_list_whitelist_allows_same_range_insert_delete() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![custom_shard([0u8; 32], prefix_end(0x0f), vec![entry(0x10)])],
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x0f),
            vec![entry(0x10), entry(0x20)],
        )],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_whitelist_allows_split_preserving_entries() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x2f),
            vec![prefix_entry(0x08), prefix_entry(0x20)],
        )],
        vec![
            custom_shard([0u8; 32], prefix_end(0x0f), vec![prefix_entry(0x08)]),
            custom_shard(
                prefix_start(0x10),
                prefix_end(0x2f),
                vec![prefix_entry(0x20)],
            ),
        ],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_whitelist_rejects_split_that_changes_entries() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x2f),
            vec![prefix_entry(0x08)],
        )],
        vec![
            custom_shard([0u8; 32], prefix_end(0x0f), vec![prefix_entry(0x08)]),
            custom_shard(
                prefix_start(0x10),
                prefix_end(0x2f),
                vec![prefix_entry(0x20)],
            ),
        ],
    );

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 61");
}

#[test]
fn access_list_blacklist_allows_local_same_range_insert_delete() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED,
        true,
        vec![custom_shard([0u8; 32], prefix_end(0x0f), vec![entry(0x10)])],
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x0f),
            vec![entry(0x10), entry(0x20)],
        )],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_allows_local_split_preserving_entries() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED,
        true,
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x2f),
            vec![prefix_entry(0x08), prefix_entry(0x20)],
        )],
        vec![
            custom_shard([0u8; 32], prefix_end(0x0f), vec![prefix_entry(0x08)]),
            custom_shard(
                prefix_start(0x10),
                prefix_end(0x2f),
                vec![prefix_entry(0x20)],
            ),
        ],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_active_destroy_requires_full_domain_inputs_and_empty_outputs() {
    let partial = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        true,
        vec![custom_shard([0u8; 32], prefix_end(0x7f), Vec::new())],
        Vec::new(),
    );
    expect_tx_fail_with_code(&partial.context, &partial.tx, "error code 61");

    let with_output = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        true,
        vec![full_domain_shard(Vec::new())],
        vec![full_domain_shard(Vec::new())],
    );
    expect_tx_fail_with_code(&with_output.context, &with_output.tx, "error code 61");

    let full_destroy = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        true,
        vec![full_domain_shard(Vec::new())],
        Vec::new(),
    );
    expect_tx_pass(&full_destroy.context, &full_destroy.tx);
}

#[test]
fn access_list_mode_replace_requires_full_domain_inputs_and_outputs_but_allows_entry_reset() {
    let missing_input = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        Vec::new(),
        vec![full_domain_shard(vec![entry(0x20)])],
    );
    expect_tx_fail_with_code(&missing_input.context, &missing_input.tx, "error code 61");

    let full_replace = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![full_domain_shard(vec![entry(0x10)])],
        vec![full_domain_shard(vec![entry(0x20)])],
    );
    expect_tx_pass(&full_replace.context, &full_replace.tx);
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
fn access_list_blacklist_rejects_suffix_only_nibble_alignment() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(Vec::new())],
        vec![
            bounded_shard(0x00, 0x0f, Vec::new()),
            tail_shard(0x10, Vec::new()),
        ],
    );

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 60");
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
fn access_list_update_allows_visible_meta_with_non_whitelisted_lock() {
    let case = access_list_update_tx_with_non_whitelisted_meta_lock(
        CONFIG_ACCESS_ENABLED,
        vec![full_domain_shard(vec![entry(0x10)])],
        vec![full_domain_shard(vec![entry(0x10)])],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_rejects_non_whitelisted_output_lock() {
    let case =
        access_list_update_tx_with_non_whitelisted_output_lock(vec![full_domain_shard(Vec::new())]);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 20");
}

#[test]
fn access_list_update_with_dynamic_linking_authority_passes() {
    let case = access_list_update_tx_with_plugin_authority(
        "authority-dl-allow",
        false,
        vec![full_domain_shard(vec![entry(0x10)])],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_update_with_dynamic_linking_authority_denies() {
    let case = access_list_update_tx_with_plugin_authority(
        "authority-dl-deny",
        false,
        vec![full_domain_shard(vec![entry(0x10)])],
    );

    expect_tx_fail(&case.context, &case.tx);
}

#[test]
fn access_list_update_with_spawn_authority_passes() {
    let case = access_list_update_tx_with_plugin_authority(
        "authority-spawn-allow",
        true,
        vec![full_domain_shard(vec![entry(0x10)])],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_update_with_spawn_authority_denies() {
    let case = access_list_update_tx_with_plugin_authority(
        "authority-spawn-deny",
        true,
        vec![full_domain_shard(vec![entry(0x10)])],
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
        vec![full_domain_shard(vec![
            prefix_entry(0x08),
            prefix_entry(0x20),
        ])],
        vec![
            custom_shard([0u8; 32], prefix_end(0x0f), vec![prefix_entry(0x08)]),
            custom_shard(prefix_start(0x10), [0xffu8; 32], vec![prefix_entry(0x20)]),
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
