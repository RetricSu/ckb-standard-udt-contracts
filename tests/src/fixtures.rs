use crate::{
    metadata_builders::{script_hash, DeployedScript},
    verify_and_dump_failed_tx, Loader,
};
use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        core::{DepType, TransactionView},
        packed::*,
        prelude::*,
    },
    context::Context,
};

pub const MAX_CYCLES: u64 = 10_000_000;

pub fn deploy_contract(context: &mut Context, name: &str) -> OutPoint {
    context.deploy_cell(Loader::default().load_binary(name))
}

pub fn deploy_script_with_args(
    context: &mut Context,
    binary_name: &str,
    args: Bytes,
) -> DeployedScript {
    let out_point = deploy_contract(context, binary_name);
    let script = context
        .build_script(&out_point, args)
        .expect("build deployed script");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

pub fn cell_dep_for_script(deployed: &DeployedScript) -> CellDep {
    cell_dep(deployed.out_point.clone())
}

pub fn typed_output(lock: &Script, type_script: &Script, capacity: u64) -> CellOutput {
    CellOutput::new_builder()
        .capacity(ckb_types::prelude::Pack::<Uint64>::pack(&capacity))
        .lock(lock.clone())
        .type_(Some(type_script.clone()).pack())
        .build()
}

pub fn normal_output(lock: &Script, capacity: u64) -> CellOutput {
    CellOutput::new_builder()
        .capacity(ckb_types::prelude::Pack::<Uint64>::pack(&capacity))
        .lock(lock.clone())
        .build()
}

pub fn create_typed_cell(
    context: &mut Context,
    lock: &Script,
    type_script: &Script,
    capacity: u64,
    data: Bytes,
) -> OutPoint {
    context.create_cell(typed_output(lock, type_script, capacity), data)
}

pub fn create_funding_input(context: &mut Context, lock: &Script, capacity: u64) -> CellInput {
    let out_point = context.create_cell(normal_output(lock, capacity), Bytes::new());
    CellInput::new_builder().previous_output(out_point).build()
}

pub fn cell_dep(out_point: OutPoint) -> CellDep {
    CellDep::new_builder()
        .out_point(out_point)
        .dep_type(DepType::Code)
        .build()
}

pub fn expect_tx_pass(context: &Context, tx: &TransactionView) {
    verify_and_dump_failed_tx(context, tx, MAX_CYCLES).expect("tx should pass");
}

pub fn expect_tx_fail_with_code(context: &Context, tx: &TransactionView, code_marker: &str) {
    match verify_and_dump_failed_tx(context, tx, MAX_CYCLES) {
        Ok(_) => panic!("tx should fail with code marker `{code_marker}`"),
        Err(err) => {
            let err_msg = err.to_string();
            assert!(
                err_msg.contains(code_marker),
                "tx failed but missing code marker `{code_marker}` in error: {err_msg}"
            );
        }
    }
}

pub fn expect_tx_fail_with_any_code(
    context: &Context,
    tx: &TransactionView,
    code_markers: &[&str],
) {
    match verify_and_dump_failed_tx(context, tx, MAX_CYCLES) {
        Ok(_) => panic!("tx should fail with one of code markers `{code_markers:?}`"),
        Err(err) => {
            let err_msg = err.to_string();
            assert!(
                code_markers.iter().any(|marker| err_msg.contains(marker)),
                "tx failed but missing any code marker `{code_markers:?}` in error: {err_msg}"
            );
        }
    }
}
