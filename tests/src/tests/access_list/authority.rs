use super::*;

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

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 51");
}

#[test]
fn access_list_update_accepts_mint_authority() {
    let case = access_list_update_tx_with_mint_authority(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(Vec::new())],
        vec![full_domain_shard(vec![entry(0x10)])],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_mint_authority_fallback_survives_broken_access_authority() {
    let mut context = Context::default();
    let authority = always_success_lock(&mut context, Bytes::from(vec![1u8]));
    let cell_lock = always_success_lock(&mut context, Bytes::from(vec![2u8]));
    let plugin = deploy_data_script(
        &mut context,
        "authority-dl-allow",
        Bytes::from_static(b"allow"),
    );
    let meta = meta_script(&mut context);
    let access_list = access_list_script(&mut context, meta.script_hash);
    let meta_data = build_xudt_meta_data(
        CONFIG_ACCESS_ENABLED,
        0,
        Some(input_lock_authority(authority.script_hash)),
        None,
        Some(dynamic_linking_authority(&plugin)),
        Vec::new(),
    );
    let meta_out_point = create_typed_cell(
        &mut context,
        &cell_lock.script,
        &meta.script,
        100_000_000_000,
        meta_data,
    );
    let authority_out_point = context.create_cell(
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

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(authority_out_point)
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .output(typed_output(
            &cell_lock.script,
            &access_list.script,
            100_000_000_000,
        ))
        .output_data(full_domain_shard(vec![entry(0x10)]).pack())
        .cell_dep(cell_dep(meta_out_point))
        .cell_dep(cell_dep_for_script(&cell_lock))
        .cell_dep(cell_dep_for_script(&authority))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&access_list))
        .build();
    let tx = context.complete_tx(tx);

    expect_tx_pass(&context, &tx);
}

#[test]
fn access_list_rejects_noop_update_without_authority() {
    let mut context = Context::default();
    let authority = always_success_lock(&mut context, Bytes::from(vec![1u8]));
    let cell_lock = always_success_lock(&mut context, Bytes::from(vec![2u8]));
    let meta = meta_script(&mut context);
    let access_list = access_list_script(&mut context, meta.script_hash);
    let meta_data = build_xudt_meta_data(
        CONFIG_ACCESS_ENABLED,
        0,
        None,
        None,
        Some(input_lock_authority(authority.script_hash)),
        Vec::new(),
    );
    let meta_out_point = create_typed_cell(
        &mut context,
        &cell_lock.script,
        &meta.script,
        100_000_000_000,
        meta_data,
    );
    let shard_data = full_domain_shard(Vec::new());
    let input_out_point = create_typed_cell(
        &mut context,
        &cell_lock.script,
        &access_list.script,
        100_000_000_000,
        shard_data.clone(),
    );

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .output(typed_output(
            &cell_lock.script,
            &access_list.script,
            100_000_000_000,
        ))
        .output_data(shard_data.pack())
        .cell_dep(cell_dep(meta_out_point))
        .cell_dep(cell_dep_for_script(&cell_lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&access_list))
        .build();
    let tx = context.complete_tx(tx);

    expect_tx_fail_with_code(&context, &tx, "error code 51");
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
fn access_list_rejects_data_hash_type_output_lock() {
    let case = access_list_update_tx_with_output_lock(
        |context| always_success_lock_with_hash_type(context, ScriptHashType::Data, Bytes::new()),
        vec![full_domain_shard(Vec::new())],
    );

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

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 51");
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

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 51");
}
