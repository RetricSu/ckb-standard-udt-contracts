use crate::{
    fixtures::{
        cell_dep_for_script, create_funding_input, create_typed_cell, expect_tx_fail,
        expect_tx_fail_with_code, expect_tx_pass, typed_output,
    },
    metadata_builders::{
        build_sudt_meta_bytes, dynamic_linking_authority as deployed_dynamic_linking_authority,
        input_lock_authority, script_hash, spawn_authority, udt_amount_bytes, DeployedScript,
    },
    test_helpers::{
        always_success_lock_empty as always_success_lock, calculate_type_id, deploy_data2_script,
        deploy_data_script, fake_data2_script, non_whitelisted_lock,
        sudt_meta_script as meta_script, sudt_script as udt_script,
    },
    Loader,
};
use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        core::{ScriptHashType, TransactionBuilder, TransactionView},
        packed::{CellInput, Script},
        prelude::*,
    },
    context::Context,
};
use standard_udt_types::metadata::{Authority, SudtMeta, CONFIG_SUPPLY_TRACKED};

fn tracked_meta_data(current_supply: u128) -> Bytes {
    build_sudt_meta_bytes(CONFIG_SUPPLY_TRACKED, current_supply, None, None)
}

fn sudt_meta_data(
    config_flags: u8,
    current_supply: u128,
    mint_authority: Option<Authority>,
    metadata_authority: Option<Authority>,
    name: Vec<u8>,
    extra_data: Vec<u8>,
) -> Bytes {
    Bytes::from(
        SudtMeta {
            config_flags,
            current_supply,
            decimals: 0,
            name,
            symbol: Vec::new(),
            uri: Vec::new(),
            extra_data,
            mint_authority,
            metadata_authority,
        }
        .to_bytes()
        .expect("build SudtMeta bytes"),
    )
}

fn untracked_nonzero_meta_data(current_supply: u128) -> Bytes {
    let mut data = tracked_meta_data(current_supply).to_vec();
    let config_offset = u32::from_le_bytes(data[4..8].try_into().expect("config offset")) as usize;
    data[config_offset] = 0;
    Bytes::from(data)
}

fn create_meta_tx(
    current_supply: u128,
    udt_amount: Option<u128>,
    fake_udt_amount: Option<u128>,
    valid_type_id: bool,
) -> (Context, TransactionView) {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta_out_point = context.deploy_cell(Loader::default().load_binary("sudt-meta"));
    let input = create_funding_input(&mut context, &lock.script, 1_000_000_000_000);
    let type_id = if valid_type_id {
        calculate_type_id(&input, 0)
    } else {
        [1u8; 32]
    };
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
    let udt = udt_script(&mut context, meta.script_hash);
    let meta_data = tracked_meta_data(current_supply);

    let mut outputs = vec![typed_output(&lock.script, &meta.script, 100_000_000_000)];
    let mut outputs_data = vec![meta_data];
    if let Some(amount) = udt_amount {
        outputs.push(typed_output(&lock.script, &udt.script, 100_000_000_000));
        outputs_data.push(udt_amount_bytes(amount));
    }
    let fake_udt = if fake_udt_amount.is_some() {
        Some(fake_data2_script(&mut context, meta.script_hash))
    } else {
        None
    };
    if let (Some(fake), Some(amount)) = (fake_udt.as_ref(), fake_udt_amount) {
        outputs.push(typed_output(&lock.script, &fake.script, 100_000_000_000));
        outputs_data.push(udt_amount_bytes(amount));
    }

    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&udt))
        .build();
    let tx = if let Some(fake) = fake_udt.as_ref() {
        tx.as_advanced_builder()
            .cell_dep(cell_dep_for_script(fake))
            .build()
    } else {
        tx
    };
    let tx = context.complete_tx(tx);
    (context, tx)
}

fn create_meta_tx_with_udt_output_data(
    current_supply: u128,
    udt_outputs_data: Vec<Bytes>,
) -> (Context, TransactionView) {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta_out_point = context.deploy_cell(Loader::default().load_binary("sudt-meta"));
    let input = create_funding_input(&mut context, &lock.script, 1_000_000_000_000);
    let type_id = calculate_type_id(&input, 0);
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
    let udt = udt_script(&mut context, meta.script_hash);

    let mut outputs = vec![typed_output(&lock.script, &meta.script, 100_000_000_000)];
    let mut outputs_data = vec![tracked_meta_data(current_supply)];
    for data in udt_outputs_data {
        outputs.push(typed_output(&lock.script, &udt.script, 100_000_000_000));
        outputs_data.push(data);
    }

    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&udt))
        .build();
    let tx = context.complete_tx(tx);
    (context, tx)
}

fn update_meta_tx(input_meta_data: Bytes, output_meta_data: Bytes) -> (Context, TransactionView) {
    update_meta_tx_with_data(|_, _| (input_meta_data, output_meta_data))
}

fn update_meta_tx_with_data<F>(build_data: F) -> (Context, TransactionView)
where
    F: FnOnce([u8; 32], Script) -> (Bytes, Bytes),
{
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let (input_meta_data, output_meta_data) = build_data(lock.script_hash, lock.script.clone());
    let meta = meta_script(&mut context, Bytes::from(vec![2u8; 32]));
    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        input_meta_data,
    );
    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();

    let tx = TransactionBuilder::default()
        .input(input)
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(output_meta_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .build();
    let tx = context.complete_tx(tx);
    (context, tx)
}

fn update_meta_tx_with_udt_delta(
    input_supply: u128,
    output_supply: u128,
    input_udt_amount: Option<u128>,
    output_udt_amount: Option<u128>,
) -> (Context, TransactionView) {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let authority = input_lock_authority(lock.script_hash);
    let input_meta_data = sudt_meta_data(
        CONFIG_SUPPLY_TRACKED,
        input_supply,
        Some(authority.clone()),
        None,
        Vec::new(),
        Vec::new(),
    );
    let output_meta_data = sudt_meta_data(
        CONFIG_SUPPLY_TRACKED,
        output_supply,
        Some(authority),
        None,
        Vec::new(),
        Vec::new(),
    );
    let meta = meta_script(&mut context, Bytes::from(vec![2u8; 32]));
    let udt = udt_script(&mut context, meta.script_hash);
    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        input_meta_data,
    );

    let mut builder = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(output_meta_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&udt));

    if let Some(amount) = input_udt_amount {
        let out_point = create_typed_cell(
            &mut context,
            &lock.script,
            &udt.script,
            100_000_000_000,
            udt_amount_bytes(amount),
        );
        builder = builder.input(CellInput::new_builder().previous_output(out_point).build());
    }

    if let Some(amount) = output_udt_amount {
        builder = builder
            .output(typed_output(&lock.script, &udt.script, 100_000_000_000))
            .output_data(udt_amount_bytes(amount).pack());
    }

    let tx = context.complete_tx(builder.build());
    (context, tx)
}

fn update_meta_tx_with_locks<F>(build_data: F) -> (Context, TransactionView)
where
    F: FnOnce(&mut Context, [u8; 32], Script) -> (DeployedScript, Bytes, Bytes),
{
    let mut context = Context::default();
    let input_lock = always_success_lock(&mut context);
    let (output_lock, input_meta_data, output_meta_data) = build_data(
        &mut context,
        input_lock.script_hash,
        input_lock.script.clone(),
    );
    let meta = meta_script(&mut context, Bytes::from(vec![2u8; 32]));
    let input_out_point = create_typed_cell(
        &mut context,
        &input_lock.script,
        &meta.script,
        100_000_000_000,
        input_meta_data,
    );
    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .output(typed_output(
            &output_lock.script,
            &meta.script,
            100_000_000_000,
        ))
        .output_data(output_meta_data.pack())
        .cell_dep(cell_dep_for_script(&input_lock))
        .cell_dep(cell_dep_for_script(&output_lock))
        .cell_dep(cell_dep_for_script(&meta))
        .build();
    let tx = context.complete_tx(tx);
    (context, tx)
}

fn update_meta_tx_with_plugin_authority(
    plugin_name: &str,
    spawn: bool,
) -> (Context, TransactionView) {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let plugin = if spawn {
        deploy_data2_script(&mut context, plugin_name, Bytes::from_static(b"allow"))
    } else {
        deploy_data_script(&mut context, plugin_name, Bytes::from_static(b"allow"))
    };
    let authority = if spawn {
        spawn_authority(&plugin)
    } else {
        deployed_dynamic_linking_authority(&plugin)
    };
    let input_meta_data = sudt_meta_data(
        CONFIG_SUPPLY_TRACKED,
        0,
        None,
        Some(authority),
        Vec::new(),
        Vec::new(),
    );
    let output_meta_data = sudt_meta_data(
        CONFIG_SUPPLY_TRACKED,
        0,
        None,
        None,
        b"new name".to_vec(),
        Vec::new(),
    );
    let meta = meta_script(&mut context, Bytes::from(vec![2u8; 32]));
    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        input_meta_data,
    );
    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();

    let tx = TransactionBuilder::default()
        .input(input)
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(output_meta_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&plugin))
        .build();
    let tx = context.complete_tx(tx);
    (context, tx)
}

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

    expect_tx_fail_with_code(&context, &tx, "error code 21");
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

    expect_tx_fail(&context, &tx);
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

#[test]
fn sudt_meta_update_supply_change_with_input_lock_mint_authority_passes() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 101, None, Some(1));

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_rejects_supply_increase_without_udt_delta() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 101, None, None);

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}

#[test]
fn sudt_meta_rejects_supply_decrease_without_udt_delta() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 99, None, None);

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}

#[test]
fn sudt_meta_accepts_supply_increase_matching_udt_delta() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 125, None, Some(25));

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_accepts_supply_decrease_matching_udt_delta() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 75, Some(25), None);

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_rejects_supply_delta_mismatch() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 125, None, Some(24));

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}

#[test]
fn sudt_meta_update_rejects_mint_authority_recreation() {
    let (context, tx) = update_meta_tx_with_data(|lock_hash, _| {
        (
            tracked_meta_data(0),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                Some(input_lock_authority(lock_hash)),
                None,
                Vec::new(),
                Vec::new(),
            ),
        )
    });

    expect_tx_fail_with_code(&context, &tx, "error code 50");
}

#[test]
fn sudt_meta_update_metadata_change_with_dynamic_linking_authority_passes() {
    let (context, tx) = update_meta_tx_with_plugin_authority("authority-dl-allow", false);

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_update_metadata_change_with_dynamic_linking_authority_denies() {
    let (context, tx) = update_meta_tx_with_plugin_authority("authority-dl-deny", false);

    expect_tx_fail_with_code(&context, &tx, "error code 51");
}

#[test]
fn sudt_meta_update_metadata_change_with_spawn_authority_passes() {
    let (context, tx) = update_meta_tx_with_plugin_authority("authority-spawn-allow", true);

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_update_metadata_change_with_spawn_authority_denies() {
    let (context, tx) = update_meta_tx_with_plugin_authority("authority-spawn-deny", true);

    expect_tx_fail_with_code(&context, &tx, "error code 51");
}
