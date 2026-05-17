use super::*;

#[test]
fn sudt_meta_update_metadata_change_requires_metadata_authority() {
    let (context, tx) = update_meta_tx(
        tracked_meta_data(0),
        sudt_meta_data(
            CONFIG_SUPPLY_TRACKED,
            0,
            None,
            None,
            b"new name".to_vec(),
            Vec::new(),
        ),
    );

    expect_tx_fail_with_code(&context, &tx, "error code 50");
}

#[test]
fn sudt_meta_update_rejects_duplicate_output_meta_cells() {
    let (context, tx) = update_meta_tx_with_duplicate_outputs();

    expect_tx_fail_with_code(&context, &tx, "error code 21");
}

#[test]
fn sudt_meta_rejects_noop_update_without_authority() {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta = meta_script(&mut context, Bytes::from(vec![2u8; 32]));
    let meta_data = tracked_meta_data(0);
    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        meta_data.clone(),
    );

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(meta_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .build();
    let tx = context.complete_tx(tx);

    expect_tx_fail_with_code(&context, &tx, "error code 50");
}

#[test]
fn sudt_meta_update_metadata_change_with_input_lock_authority_passes() {
    let (context, tx) = update_meta_tx_with_data(|lock_hash, _| {
        let authority = input_lock_authority(lock_hash);
        (
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(authority.clone()),
                Vec::new(),
                Vec::new(),
            ),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(authority),
                b"new name".to_vec(),
                Vec::new(),
            ),
        )
    });

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_mint_authority_can_update_metadata() {
    let (context, tx) = update_meta_tx_with_data(|lock_hash, _| {
        let authority = input_lock_authority(lock_hash);
        (
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                Some(authority.clone()),
                None,
                Vec::new(),
                Vec::new(),
            ),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                Some(authority),
                None,
                b"new name".to_vec(),
                Vec::new(),
            ),
        )
    });

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_mint_authority_fallback_survives_broken_metadata_authority() {
    let (context, tx) = update_meta_tx_with_locks(|context, lock_hash, _| {
        let output_lock = always_success_lock(context);
        let plugin =
            deploy_data_script(context, "authority-dl-allow", Bytes::from_static(b"allow"));
        let mint_authority = input_lock_authority(lock_hash);
        let metadata_authority = deployed_dynamic_linking_authority(&plugin);
        (
            output_lock,
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                Some(mint_authority.clone()),
                Some(metadata_authority.clone()),
                Vec::new(),
                Vec::new(),
            ),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                Some(mint_authority),
                Some(metadata_authority),
                b"new name".to_vec(),
                Vec::new(),
            ),
        )
    });

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_rejects_non_whitelisted_output_lock() {
    let (context, tx) = update_meta_tx_with_locks(|context, lock_hash, _| {
        let output_lock = non_whitelisted_lock(context);
        let authority = input_lock_authority(lock_hash);
        (
            output_lock,
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(authority.clone()),
                Vec::new(),
                Vec::new(),
            ),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(authority),
                b"new name".to_vec(),
                Vec::new(),
            ),
        )
    });

    expect_tx_fail_with_code(&context, &tx, "error code 20");
}

#[test]
fn sudt_meta_rejects_data_hash_type_output_lock() {
    let (context, tx) = update_meta_tx_with_locks(|context, lock_hash, _| {
        let output_lock =
            always_success_lock_with_hash_type(context, ScriptHashType::Data, Bytes::new());
        let authority = input_lock_authority(lock_hash);
        (
            output_lock,
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(authority.clone()),
                Vec::new(),
                Vec::new(),
            ),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(authority),
                b"new name".to_vec(),
                Vec::new(),
            ),
        )
    });

    expect_tx_fail_with_code(&context, &tx, "error code 20");
}

#[test]
fn sudt_meta_update_rejects_metadata_authority_recreation() {
    let (context, tx) = update_meta_tx_with_data(|lock_hash, _| {
        (
            tracked_meta_data(0),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(input_lock_authority(lock_hash)),
                Vec::new(),
                Vec::new(),
            ),
        )
    });

    expect_tx_fail_with_code(&context, &tx, "error code 50");
}
