use crate::{
    fixtures::{
        cell_dep_for_script, create_funding_input, create_typed_cell, expect_tx_fail,
        expect_tx_fail_with_code, expect_tx_pass, typed_output,
    },
    metadata_builders::{build_sudt_meta_bytes, script_hash, udt_amount_bytes, DeployedScript},
    Loader,
};
use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        core::{ScriptHashType, TransactionBuilder, TransactionView},
        packed::CellInput,
        prelude::*,
    },
    context::Context,
};
use standard_udt_types::metadata::CONFIG_SUPPLY_TRACKED;

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
    deploy_data2_script(context, "enhanced-sudt-meta", args)
}

fn udt_script(context: &mut Context, meta_type_hash: [u8; 32]) -> DeployedScript {
    deploy_data2_script(
        context,
        "enhanced-sudt",
        Bytes::from(meta_type_hash.to_vec()),
    )
}

fn always_success_lock(context: &mut Context) -> DeployedScript {
    deploy_data2_script(context, "enhanced-sudt", Bytes::new())
}

fn tracked_meta_data(current_supply: u128) -> Bytes {
    build_sudt_meta_bytes(CONFIG_SUPPLY_TRACKED, current_supply, None, None)
}

fn untracked_nonzero_meta_data(current_supply: u128) -> Bytes {
    let mut data = tracked_meta_data(current_supply).to_vec();
    let config_offset = u32::from_le_bytes(data[4..8].try_into().expect("config offset")) as usize;
    data[config_offset] = 0;
    Bytes::from(data)
}

fn create_meta_tx(meta_data: Bytes, udt_amount: Option<u128>) -> (Context, TransactionView) {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta = meta_script(&mut context, Bytes::from(vec![1u8; 32]));
    let udt = udt_script(&mut context, meta.script_hash);
    let input = create_funding_input(&mut context, &lock.script, 1_000_000_000_000);

    let mut outputs = vec![typed_output(&lock.script, &meta.script, 100_000_000_000)];
    let mut outputs_data = vec![meta_data];
    if let Some(amount) = udt_amount {
        outputs.push(typed_output(&lock.script, &udt.script, 100_000_000_000));
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
    let tx = context.complete_tx(tx);
    (context, tx)
}

fn update_meta_tx(input_meta_data: Bytes, output_meta_data: Bytes) -> (Context, TransactionView) {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
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
    let (context, tx) = create_meta_tx(tracked_meta_data(100), Some(100));

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_create_tracked_supply_mismatch_rejects() {
    let (context, tx) = create_meta_tx(tracked_meta_data(101), Some(100));

    expect_tx_fail_with_code(&context, &tx, "error code 4");
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
    let (context, tx) = create_meta_tx(untracked_nonzero_meta_data(100), None);

    expect_tx_fail_with_code(&context, &tx, "error code 4");
}
