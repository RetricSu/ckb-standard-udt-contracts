use super::*;

#[test]
fn sudt_meta_create_tracked_supply_matches_initial_outputs() {
    let (context, tx) = create_meta_tx(100, Some(100), None, true);

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_create_tracked_supply_mismatch_rejects() {
    let (context, tx) = create_meta_tx(101, Some(100), None, true);

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}

#[test]
fn sudt_meta_create_ignores_fake_data2_udt_outputs() {
    let (context, tx) = create_meta_tx(100, None, Some(100), true);

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}

#[test]
fn sudt_meta_create_rejects_short_same_token_udt_data() {
    let (context, tx) = create_meta_tx_with_udt_output_data(0, vec![Bytes::from(vec![0u8; 15])]);

    expect_tx_fail(&context, &tx);
}

#[test]
fn sudt_meta_create_rejects_same_token_udt_sum_overflow() {
    let (context, tx) = create_meta_tx_with_udt_output_data(
        0,
        vec![udt_amount_bytes(u128::MAX), udt_amount_bytes(1)],
    );

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}

#[test]
fn sudt_meta_create_rejects_type_id_mismatch() {
    let (context, tx) = create_meta_tx(100, Some(100), None, false);

    expect_tx_fail_with_code(&context, &tx, "error code 10");
}

#[test]
fn sudt_meta_rejects_supply_tracking_bit_change() {
    let (context, tx) = update_meta_tx(
        tracked_meta_data(0),
        build_sudt_meta_bytes(0, 0, None, None),
    );

    expect_tx_fail(&context, &tx);
}

#[test]
fn sudt_meta_rejects_untracked_nonzero_supply() {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta_out_point = context.deploy_cell(Loader::default().load_binary("sudt-meta"));
    let input = create_funding_input(&mut context, &lock.script, 1_000_000_000_000);
    let type_id = calculate_type_id(&input, 0);
    let meta_script = context
        .build_script_with_hash_type(
            &meta_out_point,
            ScriptHashType::Data2,
            Bytes::from(type_id.to_vec()),
        )
        .expect("build meta script");
    let meta_script_hash = script_hash(&meta_script);
    let meta = DeployedScript {
        out_point: meta_out_point,
        script: meta_script,
        script_hash: meta_script_hash,
    };
    let udt = udt_script(&mut context, meta.script_hash);
    let tx = TransactionBuilder::default()
        .input(input)
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(untracked_nonzero_meta_data(100).pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&udt))
        .build();
    let tx = context.complete_tx(tx);

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}
