use crate::{
    fixtures::{
        cell_dep_for_script, create_funding_input, create_typed_cell, expect_tx_fail,
        expect_tx_pass, typed_output,
    },
    metadata_builders::{
        build_sudt_meta_bytes, dynamic_linking_authority, input_lock_authority, script_hash,
        spawn_authority, udt_amount_bytes, DeployedScript,
    },
    Loader,
};
use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_types::{
        bytes::Bytes,
        core::{ScriptHashType, TransactionBuilder, TransactionView},
        packed::CellInput,
        prelude::*,
    },
    context::Context,
};
use standard_udt_types::metadata::{Authority, CONFIG_SUPPLY_TRACKED};

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

fn deploy_data_script(context: &mut Context, binary_name: &str, args: Bytes) -> DeployedScript {
    let out_point = context.deploy_cell(Loader::default().load_binary(binary_name));
    let script = context
        .build_script_with_hash_type(&out_point, ScriptHashType::Data, args)
        .expect("build deployed Data script");
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

fn non_whitelisted_lock(context: &mut Context) -> DeployedScript {
    let out_point = context.deploy_cell(Bytes::from(vec![1u8]));
    let script = context
        .build_script_with_hash_type(&out_point, ScriptHashType::Data2, Bytes::new())
        .expect("build non-whitelisted lock");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

fn tracked_meta_data(current_supply: u128, lock_hash: Option<[u8; 32]>) -> Bytes {
    tracked_meta_data_with_authority(current_supply, lock_hash.map(input_lock_authority))
}

fn tracked_meta_data_with_authority(
    current_supply: u128,
    mint_authority: Option<Authority>,
) -> Bytes {
    build_sudt_meta_bytes(CONFIG_SUPPLY_TRACKED, current_supply, mint_authority, None)
}

struct SudtFixture {
    context: Context,
    lock: DeployedScript,
    meta: DeployedScript,
    udt: DeployedScript,
}

impl SudtFixture {
    fn new() -> Self {
        let mut context = Context::default();
        let lock = always_success_lock(&mut context);
        let meta = meta_script(&mut context, Bytes::from(vec![42u8; 32]));
        let udt = udt_script(&mut context, meta.script_hash);

        Self {
            context,
            lock,
            meta,
            udt,
        }
    }

    fn live_udt_input(&mut self, amount: u128) -> CellInput {
        let out_point = create_typed_cell(
            &mut self.context,
            &self.lock.script,
            &self.udt.script,
            100_000_000_000,
            udt_amount_bytes(amount),
        );
        CellInput::new_builder().previous_output(out_point).build()
    }

    fn live_meta_input(&mut self, supply: u128, authorized: bool) -> CellInput {
        let lock_hash = authorized.then_some(self.lock.script_hash);
        let out_point = create_typed_cell(
            &mut self.context,
            &self.lock.script,
            &self.meta.script,
            100_000_000_000,
            tracked_meta_data(supply, lock_hash),
        );
        CellInput::new_builder().previous_output(out_point).build()
    }

    fn live_meta_dep(&mut self, supply: u128, authorized: bool) -> CellInput {
        let lock_hash = authorized.then_some(self.lock.script_hash);
        let out_point = create_typed_cell(
            &mut self.context,
            &self.lock.script,
            &self.meta.script,
            100_000_000_000,
            tracked_meta_data(supply, lock_hash),
        );
        CellInput::new_builder().previous_output(out_point).build()
    }

    fn live_meta_dep_with_authority(
        &mut self,
        supply: u128,
        mint_authority: Option<Authority>,
    ) -> CellInput {
        let out_point = create_typed_cell(
            &mut self.context,
            &self.lock.script,
            &self.meta.script,
            100_000_000_000,
            tracked_meta_data_with_authority(supply, mint_authority),
        );
        CellInput::new_builder().previous_output(out_point).build()
    }

    fn live_meta_input_with_authority(
        &mut self,
        supply: u128,
        mint_authority: Option<Authority>,
    ) -> CellInput {
        self.live_meta_dep_with_authority(supply, mint_authority)
    }

    fn complete(&mut self, tx: TransactionView) -> TransactionView {
        let tx = tx
            .as_advanced_builder()
            .cell_dep(cell_dep_for_script(&self.lock))
            .cell_dep(cell_dep_for_script(&self.meta))
            .cell_dep(cell_dep_for_script(&self.udt))
            .build();
        self.context.complete_tx(tx)
    }
}

fn sudt_mint_with_plugin_authority(plugin_name: &str, spawn: bool) -> bool {
    let mut fixture = SudtFixture::new();
    let plugin = if spawn {
        deploy_data2_script(
            &mut fixture.context,
            plugin_name,
            Bytes::from_static(b"allow"),
        )
    } else {
        deploy_data_script(
            &mut fixture.context,
            plugin_name,
            Bytes::from_static(b"allow"),
        )
    };
    let authority = if spawn {
        spawn_authority(&plugin)
    } else {
        dynamic_linking_authority(&plugin)
    };
    let meta_input = fixture.live_meta_input_with_authority(0, Some(authority.clone()));
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
            &fixture.udt.script,
            100_000_000_000,
        ))
        .output_data(tracked_meta_data_with_authority(50, Some(authority)).pack())
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
fn sudt_mint_with_dynamic_linking_authority_passes() {
    assert!(sudt_mint_with_plugin_authority("authority-dl-allow", false));
}

#[test]
fn sudt_mint_with_dynamic_linking_authority_denies() {
    assert!(!sudt_mint_with_plugin_authority("authority-dl-deny", false));
}

#[test]
fn sudt_mint_with_spawn_authority_passes() {
    assert!(sudt_mint_with_plugin_authority(
        "authority-spawn-allow",
        true
    ));
}

#[test]
fn sudt_mint_with_spawn_authority_denies() {
    assert!(!sudt_mint_with_plugin_authority(
        "authority-spawn-deny",
        true
    ));
}

#[test]
fn sudt_transfer_does_not_require_meta() {
    let mut fixture = SudtFixture::new();
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .output(typed_output(
            &fixture.lock.script,
            &fixture.udt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn sudt_mint_requires_mint_authority() {
    let mut fixture = SudtFixture::new();
    let meta_dep = fixture.live_meta_dep(0, false);
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);

    let tx = TransactionBuilder::default()
        .input(funding)
        .cell_dep(
            ckb_testtool::ckb_types::packed::CellDep::new_builder()
                .out_point(meta_dep.previous_output())
                .build(),
        )
        .output(typed_output(
            &fixture.lock.script,
            &fixture.udt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn sudt_mint_allows_visible_meta_with_non_whitelisted_lock() {
    let mut fixture = SudtFixture::new();
    let meta_lock = non_whitelisted_lock(&mut fixture.context);
    let meta_input = fixture.live_meta_input(0, true);
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(funding)
        .output(typed_output(
            &meta_lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.udt.script,
            100_000_000_000,
        ))
        .output_data(tracked_meta_data(50, Some(fixture.lock.script_hash)).pack())
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn sudt_tracked_mint_updates_supply() {
    let mut fixture = SudtFixture::new();
    let meta_input = fixture.live_meta_input(0, true);
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
            &fixture.udt.script,
            100_000_000_000,
        ))
        .output_data(tracked_meta_data(50, Some(fixture.lock.script_hash)).pack())
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn sudt_user_destruction_without_meta_passes() {
    let mut fixture = SudtFixture::new();
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default().input(udt_input).build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn sudt_protocol_burn_requires_mint_authority() {
    let mut fixture = SudtFixture::new();
    let meta_input = fixture.live_meta_input(100, false);
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(udt_input)
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.udt.script,
            100_000_000_000,
        ))
        .output_data(tracked_meta_data(40, None).pack())
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn sudt_protocol_burn_updates_tracked_supply() {
    let mut fixture = SudtFixture::new();
    let meta_input = fixture.live_meta_input(100, true);
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(udt_input)
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.udt.script,
            100_000_000_000,
        ))
        .output_data(tracked_meta_data(40, Some(fixture.lock.script_hash)).pack())
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}
