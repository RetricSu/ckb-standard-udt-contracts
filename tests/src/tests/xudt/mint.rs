use super::*;
use ckb_testtool::ckb_types::packed::CellOutput;

#[test]
fn xudt_tracked_mint_updates_supply() {
    let mut fixture = XudtFixture::new();
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 0, true);
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(funding)
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(
            xudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                50,
                Some(input_lock_authority(fixture.lock.script_hash)),
                Vec::new(),
            )
            .pack(),
        )
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_tracked_mint_adds_to_existing_supply() {
    let mut fixture = XudtFixture::new();
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, true);
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(funding)
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(
            xudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                125,
                Some(input_lock_authority(fixture.lock.script_hash)),
                Vec::new(),
            )
            .pack(),
        )
        .output_data(udt_amount_bytes(25).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_tracked_mint_updates_supply_with_many_outputs() {
    let mut fixture = XudtFixture::new();
    let output_count = 20u128;
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 0, true);
    let funding = create_funding_input(
        &mut fixture.context,
        &fixture.lock.script,
        3_000_000_000_000,
    );

    let mut builder = TransactionBuilder::default()
        .input(meta_input)
        .input(funding)
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output_data(
            xudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                output_count,
                Some(input_lock_authority(fixture.lock.script_hash)),
                Vec::new(),
            )
            .pack(),
        );
    for _ in 0..output_count {
        builder = builder
            .output(typed_output(
                &fixture.lock.script,
                &fixture.xudt.script,
                100_000_000_000,
            ))
            .output_data(udt_amount_bytes(1).pack());
    }
    let tx = fixture.complete(builder.build());

    expect_tx_pass(&fixture.context, &tx);
}

fn xudt_mint_with_plugin_authority(plugin_name: &str, spawn: bool) -> bool {
    xudt_mint_with_plugin_authority_args(plugin_name, spawn, Bytes::from_static(b"allow"))
}

fn xudt_mint_with_plugin_authority_args(plugin_name: &str, spawn: bool, args: Bytes) -> bool {
    let mut fixture = XudtFixture::new();
    let plugin = if spawn {
        deploy_data2_script(&mut fixture.context, plugin_name, args)
    } else {
        deploy_data_script(&mut fixture.context, plugin_name, args)
    };
    let authority = if spawn {
        spawn_authority(&plugin)
    } else {
        dynamic_linking_authority(&plugin)
    };
    let meta_input =
        fixture.live_meta_input_with_authority(CONFIG_SUPPLY_TRACKED, 0, Some(authority.clone()));
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(funding)
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(xudt_meta_data(CONFIG_SUPPLY_TRACKED, 50, Some(authority), Vec::new()).pack())
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = fixture
        .complete(tx)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    fixture
        .context
        .verify_tx(&tx, crate::fixtures::MAX_CYCLES)
        .is_ok()
}

fn xudt_mint_with_type_hash_dynamic_linking_authority(args: Bytes) -> bool {
    let mut fixture = XudtFixture::new();
    let out_point = fixture.context.create_cell(
        CellOutput::new_builder()
            .capacity(100_000_000_000u64.pack())
            .lock(fixture.lock.script.clone())
            .type_(Some(fixture.lock.script.clone()).pack())
            .build(),
        Loader::default().load_binary("authority-dl-allow"),
    );
    let script = fixture
        .context
        .build_script_with_hash_type(&out_point, ScriptHashType::Type, args)
        .expect("build Type dynamic-linking authority script");
    let plugin = DeployedScript {
        out_point,
        script_hash: script_hash(&script),
        script,
    };
    let authority = dynamic_linking_authority(&plugin);
    let meta_input =
        fixture.live_meta_input_with_authority(CONFIG_SUPPLY_TRACKED, 0, Some(authority.clone()));
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(funding)
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(xudt_meta_data(CONFIG_SUPPLY_TRACKED, 50, Some(authority), Vec::new()).pack())
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = fixture
        .complete(tx)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    fixture
        .context
        .verify_tx(&tx, crate::fixtures::MAX_CYCLES)
        .is_ok()
}

#[test]
fn xudt_mint_with_dynamic_linking_authority_passes() {
    assert!(xudt_mint_with_plugin_authority("authority-dl-allow", false));
}

#[test]
fn xudt_mint_with_type_hash_dynamic_linking_authority_passes() {
    assert!(xudt_mint_with_type_hash_dynamic_linking_authority(
        Bytes::from_static(b"allow"),
    ));
}

#[test]
fn xudt_mint_with_dynamic_linking_authority_validates_hash_context() {
    assert!(xudt_mint_with_plugin_authority_args(
        "authority-dl-allow",
        false,
        Bytes::from_static(b"require_hash"),
    ));
}

#[test]
fn xudt_mint_with_dynamic_linking_authority_denies() {
    assert!(!xudt_mint_with_plugin_authority("authority-dl-deny", false));
}

#[test]
fn xudt_mint_with_spawn_authority_passes() {
    assert!(xudt_mint_with_plugin_authority(
        "authority-spawn-allow",
        true
    ));
}

#[test]
fn xudt_mint_with_spawn_authority_denies() {
    assert!(!xudt_mint_with_plugin_authority(
        "authority-spawn-deny",
        true
    ));
}

#[test]
fn xudt_mint_allows_visible_meta_with_non_whitelisted_lock() {
    let mut fixture = XudtFixture::new();
    let meta_lock = non_whitelisted_lock(&mut fixture.context);
    let authority = input_lock_authority(fixture.lock.script_hash);
    let meta_dep = create_typed_cell(
        &mut fixture.context,
        &meta_lock.script,
        &fixture.meta.script,
        100_000_000_000,
        xudt_meta_data(0, 0, Some(authority.clone()), Vec::new()),
    );
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);

    let tx = TransactionBuilder::default()
        .input(funding)
        .cell_dep(cell_dep(meta_dep))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(50).pack())
        .cell_dep(cell_dep_for_script(&meta_lock))
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_untracked_mint_with_meta_dep_does_not_require_meta_update() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(0, 0, true);
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);

    let tx = TransactionBuilder::default()
        .input(funding)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_initial_create_mint_uses_output_meta() {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context, Bytes::from(vec![1u8]));
    let meta_out_point = context.deploy_cell(Loader::default().load_binary("xudt-meta"));
    let funding = create_funding_input(&mut context, &lock.script, 1_000_000_000_000);
    let type_id = calculate_type_id(&funding, 0);
    let meta = {
        let script = context
            .build_script_with_hash_type(
                &meta_out_point,
                ScriptHashType::Data2,
                Bytes::from(type_id.to_vec()),
            )
            .expect("build deployed Data2 meta script");
        let script_hash = script_hash(&script);
        DeployedScript {
            out_point: meta_out_point,
            script,
            script_hash,
        }
    };
    let xudt = xudt_script(&mut context, meta.script_hash);

    let tx = TransactionBuilder::default()
        .input(funding)
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output(typed_output(&lock.script, &xudt.script, 100_000_000_000))
        .output_data(
            xudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                50,
                Some(input_lock_authority(lock.script_hash)),
                Vec::new(),
            )
            .pack(),
        )
        .output_data(udt_amount_bytes(50).pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&xudt))
        .build();
    let tx = context.complete_tx(tx);

    expect_tx_pass(&context, &tx);
}
