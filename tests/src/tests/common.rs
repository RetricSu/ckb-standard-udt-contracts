use crate::{verify_and_dump_failed_tx, Loader};
use ckb_testtool::{
    ckb_hash::new_blake2b,
    ckb_types::{
        bytes::Bytes,
        core::{DepType, ScriptHashType, TransactionView},
        packed::*,
        prelude::*,
    },
    context::Context,
};

pub const MAX_CYCLES: u64 = 10_000_000;
pub const FLAG_ACCESS_BLACKLIST: u8 = 0b001;
pub const FLAG_ACCESS_WHITELIST: u8 = 0b011;
pub const FLAG_PAUSED: u8 = 0b100;
pub const ALWAYS_SUCCESS_LOCK_DATA_HASH: [u8; 32] = [
    0xe6, 0x83, 0xb0, 0x41, 0x39, 0x34, 0x47, 0x68, 0x34, 0x84, 0x99, 0xc2, 0x3e, 0xb1, 0x32, 0x6d,
    0x5a, 0x52, 0xd6, 0xdb, 0x00, 0x6c, 0x0d, 0x2f, 0xec, 0xe0, 0x0a, 0x83, 0x1f, 0x36, 0x60, 0xd7,
];

pub fn deploy_contract(context: &mut Context, loader: &Loader, name: &str) -> OutPoint {
    context.deploy_cell(loader.load_binary(name))
}

pub fn deploy_always_success_lock(context: &mut Context) -> Script {
    let as_bin: Bytes = ckb_testtool::builtin::ALWAYS_SUCCESS.clone();
    let out_point = context.deploy_cell(as_bin);
    context
        .build_script_with_hash_type(&out_point, ScriptHashType::Data, Bytes::new())
        .expect("always-success lock script")
}

fn build_script_from_code_hash(
    code_hash: [u8; 32],
    hash_type: ScriptHashType,
    args: Bytes,
) -> Script {
    Script::new_builder()
        .code_hash(Byte32::from_slice(&code_hash).expect("byte32"))
        .hash_type(hash_type.into())
        .args(args.pack())
        .build()
}

pub fn build_zero_lock_script() -> Script {
    build_script_from_code_hash(
        ALWAYS_SUCCESS_LOCK_DATA_HASH,
        ScriptHashType::Data,
        Bytes::new(),
    )
}

pub fn build_non_whitelist_lock_script() -> Script {
    build_script_from_code_hash([0u8; 32], ScriptHashType::Data, Bytes::new())
}

pub fn build_meta_type_script(tag: u8) -> Script {
    build_script_from_code_hash([tag; 32], ScriptHashType::Data, Bytes::from(vec![tag; 32]))
}

pub fn typed_output(lock: &Script, type_script: &Script, capacity: u64) -> CellOutput {
    CellOutput::new_builder()
        .capacity(capacity.pack())
        .lock(lock.clone())
        .type_(Some(type_script.clone()).pack())
        .build()
}

pub fn normal_output(lock: &Script, capacity: u64) -> CellOutput {
    CellOutput::new_builder()
        .capacity(capacity.pack())
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
        .dep_type(DepType::Code.into())
        .build()
}

pub fn expect_tx_pass(context: &Context, tx: &TransactionView) {
    verify_and_dump_failed_tx(context, tx, MAX_CYCLES).expect("tx should pass");
}

pub fn expect_tx_fail(context: &Context, tx: &TransactionView) {
    assert!(
        verify_and_dump_failed_tx(context, tx, MAX_CYCLES).is_err(),
        "tx should fail"
    );
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

pub fn expect_tx_fail_with_unstable_marker(context: &Context, tx: &TransactionView, marker: &str) {
    match verify_and_dump_failed_tx(context, tx, MAX_CYCLES) {
        Ok(_) => panic!("tx should fail with non-stable marker `{marker}`"),
        Err(err) => {
            let err_msg = err.to_string();
            assert!(
                err_msg.contains(marker),
                "tx failed but missing non-stable marker `{marker}` in error: {err_msg}"
            );
            eprintln!(
                "[FIX-012] non-stable failure observed for tx 0x{:x}: {err_msg}",
                tx.hash()
            );
        }
    }
}

pub fn compute_type_id_args(first_input: &CellInput, output_index: u64) -> Bytes {
    let mut blake2b = new_blake2b();
    blake2b.update(first_input.as_slice());
    blake2b.update(&output_index.to_le_bytes());
    let mut hash = [0u8; 32];
    blake2b.finalize(&mut hash);
    Bytes::from(hash.to_vec())
}

pub fn range_start(first_byte: u8) -> [u8; 32] {
    let mut value = [0u8; 32];
    value[0] = first_byte;
    value
}

pub fn range_end(first_byte: u8) -> [u8; 32] {
    let mut value = [0xffu8; 32];
    value[0] = first_byte;
    value
}
