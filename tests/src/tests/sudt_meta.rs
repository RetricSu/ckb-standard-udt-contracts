use crate::{
    fixtures::{
        cell_dep_for_script, create_funding_input, create_typed_cell, expect_tx_fail,
        expect_tx_fail_with_code, expect_tx_pass, typed_output,
    },
    metadata_builders::{
        build_sudt_meta_bytes, input_lock_authority, script_hash, udt_amount_bytes, DeployedScript,
    },
    Loader,
};
use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_hash::new_blake2b,
    ckb_types::{
        bytes::Bytes,
        core::{ScriptHashType, TransactionBuilder, TransactionView},
        packed::{CellInput, Script},
        prelude::*,
    },
    context::Context,
};
use ckb_types_120::{packed::Script as MetadataScript, prelude::Entity};
use standard_udt_types::metadata::{ScriptAttr, ScriptLocation, SudtMeta, CONFIG_SUPPLY_TRACKED};

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

fn meta_script(context: &mut Context, args: Bytes) -> DeployedScript {
    deploy_data2_script(context, "sudt-meta", args)
}

fn udt_script(context: &mut Context, meta_type_hash: [u8; 32]) -> DeployedScript {
    deploy_data2_script(context, "sudt", Bytes::from(meta_type_hash.to_vec()))
}

fn always_success_lock(context: &mut Context) -> DeployedScript {
    let out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let script = context
        .build_script_with_hash_type(&out_point, ScriptHashType::Data2, Bytes::new())
        .expect("build always-success lock");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

fn fake_data2_script(context: &mut Context, meta_type_hash: [u8; 32]) -> DeployedScript {
    let out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let script = context
        .build_script_with_hash_type(
            &out_point,
            ScriptHashType::Data2,
            Bytes::from(meta_type_hash.to_vec()),
        )
        .expect("build fake Data2 script");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

fn tracked_meta_data(current_supply: u128) -> Bytes {
    build_sudt_meta_bytes(CONFIG_SUPPLY_TRACKED, current_supply, None, None)
}

fn sudt_meta_data(
    config_flags: u8,
    current_supply: u128,
    mint_authority: Option<ScriptAttr>,
    metadata_authority: Option<ScriptAttr>,
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

fn dynamic_linking_authority(script: Script) -> ScriptAttr {
    let metadata_script =
        MetadataScript::from_slice(script.as_slice()).expect("convert script bytes");
    ScriptAttr {
        location: ScriptLocation::DynamicLinking,
        script_hash: script_hash(&script),
        script: Some(metadata_script),
    }
}

fn untracked_nonzero_meta_data(current_supply: u128) -> Bytes {
    let mut data = tracked_meta_data(current_supply).to_vec();
    let config_offset = u32::from_le_bytes(data[4..8].try_into().expect("config offset")) as usize;
    data[config_offset] = 0;
    Bytes::from(data)
}

fn calculate_type_id(input: &CellInput, output_index: u64) -> [u8; 32] {
    let mut type_id = [0u8; 32];
    let mut hasher = new_blake2b();
    hasher.update(input.as_slice());
    hasher.update(&output_index.to_le_bytes());
    hasher.finalize(&mut type_id);
    type_id
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

#[test]
fn sudt_meta_create_tracked_supply_matches_initial_outputs() {
    let (context, tx) = create_meta_tx(100, Some(100), None, true);

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_create_tracked_supply_mismatch_rejects() {
    let (context, tx) = create_meta_tx(101, Some(100), None, true);

    expect_tx_fail_with_code(&context, &tx, "error code 15");
}

#[test]
fn sudt_meta_create_ignores_fake_data2_udt_outputs() {
    let (context, tx) = create_meta_tx(100, None, Some(100), true);

    expect_tx_fail_with_code(&context, &tx, "error code 15");
}

#[test]
fn sudt_meta_create_rejects_type_id_mismatch() {
    let (context, tx) = create_meta_tx(100, Some(100), None, false);

    expect_tx_fail_with_code(&context, &tx, "error code 13");
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

    expect_tx_fail_with_code(&context, &tx, "error code 15");
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

    expect_tx_fail_with_code(&context, &tx, "error code 17");
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

    expect_tx_fail_with_code(&context, &tx, "error code 17");
}

#[test]
fn sudt_meta_update_supply_change_with_input_lock_mint_authority_passes() {
    let (context, tx) = update_meta_tx_with_data(|lock_hash, _| {
        let authority = input_lock_authority(lock_hash);
        (
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                100,
                Some(authority.clone()),
                None,
                Vec::new(),
                Vec::new(),
            ),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                101,
                Some(authority),
                None,
                Vec::new(),
                Vec::new(),
            ),
        )
    });

    expect_tx_pass(&context, &tx);
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

    expect_tx_fail_with_code(&context, &tx, "error code 17");
}

#[test]
fn sudt_meta_update_rejects_dynamic_linking_authority_for_now() {
    let (context, tx) = update_meta_tx_with_data(|_, lock_script| {
        let authority = dynamic_linking_authority(lock_script);
        let input_meta = sudt_meta_data(
            CONFIG_SUPPLY_TRACKED,
            0,
            None,
            Some(authority),
            Vec::new(),
            Vec::new(),
        );
        (
            input_meta,
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                None,
                b"new name".to_vec(),
                Vec::new(),
            ),
        )
    });

    expect_tx_fail_with_code(&context, &tx, "error code 18");
}
