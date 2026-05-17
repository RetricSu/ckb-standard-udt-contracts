use super::*;

#[test]
fn xudt_meta_rejects_invalid_config_flags() {
    let case = update_meta_tx(|_, _, _| {
        let input = xudt_meta_data(0, 0, None, None, None, Vec::new());
        let output = with_config_flags(input.clone(), CONFIG_ACCESS_WHITELIST);
        (input, output, Vec::new())
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 30");
}

#[test]
fn xudt_meta_rejects_malformed_name_bytes_field() {
    let case = update_meta_tx(|_, _, _| {
        let data = malformed_name_meta_data();
        (data.clone(), data, Vec::new())
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 30");
}

#[test]
fn xudt_meta_rejects_oversized_name_field() {
    let case = update_meta_tx(|_, _, _| {
        let data = oversized_name_meta_data();
        (data.clone(), data, Vec::new())
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 30");
}

#[test]
fn xudt_meta_rejects_non_whitelisted_output_lock() {
    let case = update_meta_tx_with_output_lock(non_whitelisted_lock);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 20");
}

#[test]
fn xudt_meta_rejects_data_hash_type_output_lock() {
    let case = update_meta_tx_with_output_lock(|context| {
        always_success_lock_with_hash_type(context, ScriptHashType::Data, Bytes::new())
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 20");
}

#[test]
fn xudt_meta_update_rejects_duplicate_output_meta_cells() {
    let case = update_meta_tx_with_duplicate_outputs();

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 21");
}

#[test]
fn xudt_meta_rejects_noop_update_without_authority() {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta = meta_script(&mut context);
    let meta_data = xudt_meta_data(0, 0, None, None, None, Vec::new());
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
fn xudt_meta_extension_change_requires_mint_authority() {
    let no_mint_authority = update_meta_tx(|_, lock, _| {
        let metadata_authority = input_lock_authority(lock.script_hash);
        let input = xudt_meta_data(
            0,
            0,
            None,
            Some(metadata_authority.clone()),
            None,
            Vec::new(),
        );
        let output = xudt_meta_data(
            0,
            0,
            None,
            Some(metadata_authority),
            None,
            vec![presence_extension(ExtensionType::InputLock, &lock.script)],
        );
        (input, output, Vec::new())
    });

    expect_tx_fail_with_code(
        &no_mint_authority.context,
        &no_mint_authority.tx,
        "error code 50",
    );

    let with_mint_authority = update_meta_tx(|_, lock, _| {
        let mint_authority = input_lock_authority(lock.script_hash);
        let input = xudt_meta_data(0, 0, Some(mint_authority.clone()), None, None, Vec::new());
        let output = xudt_meta_data(
            0,
            0,
            Some(mint_authority),
            None,
            None,
            vec![presence_extension(ExtensionType::InputLock, &lock.script)],
        );
        (input, output, Vec::new())
    });

    expect_tx_pass(&with_mint_authority.context, &with_mint_authority.tx);
}

fn presence_extension(extension_type: ExtensionType, script: &Script) -> Extension {
    Extension {
        extension_type,
        script: MetadataScript::from_slice(script.as_slice()).expect("convert script"),
    }
}
