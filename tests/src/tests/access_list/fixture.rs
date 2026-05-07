use super::*;

fn xudt_meta_data(config_flags: u8, authority: &DeployedScript) -> Bytes {
    xudt_meta_data_with_authority(
        config_flags,
        Some(input_lock_authority(authority.script_hash)),
    )
}

fn xudt_meta_data_with_authority(config_flags: u8, authority: Option<Authority>) -> Bytes {
    build_xudt_meta_data(config_flags, 0, None, None, authority, Vec::new())
}

pub(super) fn shard(start_last: u8, end_last: u8, entries: Vec<[u8; 32]>) -> Bytes {
    crate::test_helpers::suffix_shard(start_last, end_last, entries)
}

pub(super) struct AccessListCase {
    pub(super) context: Context,
    pub(super) tx: TransactionView,
}

pub(super) fn expect_tx_pass_with_cycles(context: &Context, tx: &TransactionView, max_cycles: u64) {
    verify_and_dump_failed_tx(context, tx, max_cycles).expect("tx should pass");
}

pub(super) fn access_list_update_tx(
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

pub(super) fn access_list_update_tx_with_mint_authority(
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
    let mint_authority = input_lock_authority(authority.script_hash);
    let meta_data = build_xudt_meta_data(
        config_flags,
        0,
        Some(mint_authority.clone()),
        None,
        None,
        Vec::new(),
    );

    let meta_out_point = create_typed_cell(
        &mut context,
        &cell_lock.script,
        &meta.script,
        100_000_000_000,
        meta_data.clone(),
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
        .output_data(meta_data.pack())
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

pub(super) fn access_list_transition_tx(
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

pub(super) fn access_list_update_tx_with_non_whitelisted_meta_lock(
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

pub(super) fn access_list_update_tx_with_non_whitelisted_output_lock(
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

pub(super) fn access_list_update_tx_with_plugin_authority(
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
