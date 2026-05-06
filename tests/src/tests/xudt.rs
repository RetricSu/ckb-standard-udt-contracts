use crate::{
    fixtures::{
        cell_dep, cell_dep_for_script, create_funding_input, create_typed_cell, expect_tx_fail,
        expect_tx_fail_with_code, expect_tx_pass, typed_output,
    },
    metadata_builders::{
        dynamic_linking_authority, input_lock_authority, script_hash, spawn_authority,
        udt_amount_bytes, DeployedScript,
    },
    test_helpers::{
        access_list_script, always_success_lock, calculate_type_id, custom_shard,
        deploy_data2_script, deploy_data_script, exact_shard, full_domain_shard,
        non_whitelisted_lock, xudt_meta_data, xudt_meta_script as meta_script, xudt_script,
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
use standard_udt_types::metadata::{
    Authority, CONFIG_ACCESS_ENABLED, CONFIG_ACCESS_WHITELIST, CONFIG_PAUSED, CONFIG_SUPPLY_TRACKED,
};

struct XudtFixture {
    context: Context,
    lock: DeployedScript,
    other_lock: DeployedScript,
    meta: DeployedScript,
    xudt: DeployedScript,
    access_list: DeployedScript,
}

impl XudtFixture {
    fn new() -> Self {
        let mut context = Context::default();
        let lock = always_success_lock(&mut context, Bytes::from(vec![1u8]));
        let other_lock = always_success_lock(&mut context, Bytes::from(vec![2u8]));
        let meta = meta_script(&mut context);
        let xudt = xudt_script(&mut context, meta.script_hash);
        let access_list = access_list_script(&mut context, meta.script_hash);

        Self {
            context,
            lock,
            other_lock,
            meta,
            xudt,
            access_list,
        }
    }

    fn new_with_always_success_meta() -> Self {
        let mut context = Context::default();
        let lock = always_success_lock(&mut context, Bytes::from(vec![1u8]));
        let other_lock = always_success_lock(&mut context, Bytes::from(vec![2u8]));
        let meta = always_success_lock(&mut context, Bytes::from(vec![3u8]));
        let xudt = xudt_script(&mut context, meta.script_hash);
        let access_list = access_list_script(&mut context, meta.script_hash);

        Self {
            context,
            lock,
            other_lock,
            meta,
            xudt,
            access_list,
        }
    }

    fn live_udt_input_with_lock(&mut self, lock: &Script, amount: u128) -> CellInput {
        let out_point = create_typed_cell(
            &mut self.context,
            lock,
            &self.xudt.script,
            100_000_000_000,
            udt_amount_bytes(amount),
        );
        CellInput::new_builder().previous_output(out_point).build()
    }

    fn live_udt_input(&mut self, amount: u128) -> CellInput {
        let lock = self.lock.script.clone();
        self.live_udt_input_with_lock(&lock, amount)
    }

    fn live_meta_input(&mut self, config_flags: u8, supply: u128, authorized: bool) -> CellInput {
        let mint_authority = authorized.then_some(input_lock_authority(self.lock.script_hash));
        self.live_meta_input_with_authority(config_flags, supply, mint_authority)
    }

    fn live_meta_input_with_authority(
        &mut self,
        config_flags: u8,
        supply: u128,
        mint_authority: Option<Authority>,
    ) -> CellInput {
        let out_point = create_typed_cell(
            &mut self.context,
            &self.lock.script,
            &self.meta.script,
            100_000_000_000,
            xudt_meta_data(config_flags, supply, mint_authority, Vec::new()),
        );
        CellInput::new_builder().previous_output(out_point).build()
    }

    fn live_meta_dep(&mut self, config_flags: u8, supply: u128, authorized: bool) -> CellInput {
        self.live_meta_input(config_flags, supply, authorized)
    }

    fn output_meta_data(
        &self,
        config_flags: u8,
        supply: u128,
        mint_authority: Option<Authority>,
    ) -> Bytes {
        xudt_meta_data(config_flags, supply, mint_authority, Vec::new())
    }

    fn live_access_list_input(&mut self, data: Bytes) -> CellInput {
        let out_point = create_typed_cell(
            &mut self.context,
            &self.lock.script,
            &self.access_list.script,
            100_000_000_000,
            data,
        );
        CellInput::new_builder().previous_output(out_point).build()
    }

    fn live_access_list_input_with_hash_type(
        &mut self,
        hash_type: ScriptHashType,
        data: Bytes,
    ) -> CellInput {
        let type_script = self
            .context
            .build_script_with_hash_type(
                &self.access_list.out_point,
                hash_type,
                Bytes::from(self.meta.script_hash.to_vec()),
            )
            .expect("build access-list script with hash type");
        let out_point = create_typed_cell(
            &mut self.context,
            &self.lock.script,
            &type_script,
            100_000_000_000,
            data,
        );
        CellInput::new_builder().previous_output(out_point).build()
    }

    fn complete(&mut self, tx: TransactionView) -> TransactionView {
        let tx = tx
            .as_advanced_builder()
            .cell_dep(cell_dep_for_script(&self.lock))
            .cell_dep(cell_dep_for_script(&self.other_lock))
            .cell_dep(cell_dep_for_script(&self.meta))
            .cell_dep(cell_dep_for_script(&self.xudt))
            .cell_dep(cell_dep_for_script(&self.access_list))
            .build();
        self.context.complete_tx(tx)
    }
}

#[test]
fn xudt_transfer_requires_meta() {
    let mut fixture = XudtFixture::new();
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn xudt_paused_rejects_transfer_and_mint() {
    let mut transfer = XudtFixture::new();
    let meta_dep = transfer.live_meta_dep(CONFIG_PAUSED, 0, true);
    let udt_input = transfer.live_udt_input(100);
    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .output(typed_output(
            &transfer.lock.script,
            &transfer.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = transfer.complete(tx);
    expect_tx_fail(&transfer.context, &tx);

    let mut mint = XudtFixture::new();
    let meta_input = mint.live_meta_input(CONFIG_SUPPLY_TRACKED | CONFIG_PAUSED, 0, true);
    let funding = create_funding_input(&mut mint.context, &mint.lock.script, 100_000_000_000);
    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(funding)
        .output(typed_output(
            &mint.lock.script,
            &mint.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &mint.lock.script,
            &mint.xudt.script,
            100_000_000_000,
        ))
        .output_data(
            xudt_meta_data(
                CONFIG_SUPPLY_TRACKED | CONFIG_PAUSED,
                50,
                Some(input_lock_authority(mint.lock.script_hash)),
                Vec::new(),
            )
            .pack(),
        )
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = mint.complete(tx);
    expect_tx_fail(&mint.context, &tx);
}

#[test]
fn xudt_paused_allows_user_destruction() {
    let mut fixture = XudtFixture::new();
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default().input(udt_input).build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

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

fn xudt_mint_with_plugin_authority(plugin_name: &str, spawn: bool) -> bool {
    let mut fixture = XudtFixture::new();
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

#[test]
fn xudt_protocol_burn_requires_mint_authority_and_updates_supply() {
    let mut unauthorized = XudtFixture::new();
    let meta_input = unauthorized.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, false);
    let udt_input = unauthorized.live_udt_input(100);
    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(udt_input)
        .output(typed_output(
            &unauthorized.lock.script,
            &unauthorized.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &unauthorized.lock.script,
            &unauthorized.xudt.script,
            100_000_000_000,
        ))
        .output_data(xudt_meta_data(CONFIG_SUPPLY_TRACKED, 40, None, Vec::new()).pack())
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = unauthorized.complete(tx);
    expect_tx_fail(&unauthorized.context, &tx);

    let mut authorized = XudtFixture::new();
    let meta_input = authorized.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, true);
    let udt_input = authorized.live_udt_input(100);
    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(udt_input)
        .output(typed_output(
            &authorized.lock.script,
            &authorized.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &authorized.lock.script,
            &authorized.xudt.script,
            100_000_000_000,
        ))
        .output_data(
            xudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                40,
                Some(input_lock_authority(authorized.lock.script_hash)),
                Vec::new(),
            )
            .pack(),
        )
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = authorized.complete(tx);
    expect_tx_pass(&authorized.context, &tx);
}

#[test]
fn xudt_protocol_burn_with_meta_dep_still_uses_input_meta() {
    let mut fixture = XudtFixture::new();
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, true);
    let duplicate_meta_dep = fixture.live_meta_dep(CONFIG_SUPPLY_TRACKED, 100, true);
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(udt_input)
        .cell_dep(cell_dep(duplicate_meta_dep.previous_output()))
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
                40,
                Some(input_lock_authority(fixture.lock.script_hash)),
                Vec::new(),
            )
            .pack(),
        )
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_user_destruction_skips_access_and_extensions() {
    let mut fixture = XudtFixture::new();
    let udt_input = fixture.live_udt_input(100);
    let listed_lock = fixture.lock.script_hash;
    let access_list = fixture.live_access_list_input(full_domain_shard(vec![listed_lock]));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(access_list.previous_output()))
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_protocol_burn_with_meta_dep_requires_mint_authority() {
    let mut fixture = XudtFixture::new_with_always_success_meta();
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, false);
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(meta_input.clone())
        .input(udt_input)
        .cell_dep(cell_dep(meta_input.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output_data(xudt_meta_data(CONFIG_SUPPLY_TRACKED, 100, None, Vec::new()).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 50");
}

#[test]
fn xudt_negative_delta_with_input_meta_requires_protocol_burn_authority() {
    let mut fixture = XudtFixture::new();
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, false);
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(meta_input.clone())
        .input(udt_input)
        .cell_dep(cell_dep(meta_input.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output_data(xudt_meta_data(CONFIG_SUPPLY_TRACKED, 100, None, Vec::new()).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 50");
}

#[test]
fn xudt_blacklist_rejects_listed_input_lock() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let access_list =
        fixture.live_access_list_input(full_domain_shard(vec![fixture.lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(access_list.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn xudt_blacklist_rejects_missing_non_membership_proof() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let non_covering =
        fixture.live_access_list_input(custom_shard([0u8; 32], [0u8; 32], Vec::new()));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(non_covering.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 60");
}

#[test]
fn xudt_blacklist_accepts_covering_non_membership_proof() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let proof = fixture.live_access_list_input(exact_shard(fixture.lock.script_hash, Vec::new()));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_whitelist_rejects_missing_input_lock() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let access_list =
        fixture.live_access_list_input(full_domain_shard(vec![fixture.other_lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(access_list.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn xudt_whitelist_accepts_covering_membership_proof() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let proof = fixture.live_access_list_input(exact_shard(
        fixture.lock.script_hash,
        vec![fixture.lock.script_hash],
    ));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_whitelist_ignores_non_data2_access_list_shards() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let fake_access_list = fixture.live_access_list_input_with_hash_type(
        ScriptHashType::Data,
        full_domain_shard(vec![fixture.lock.script_hash]),
    );

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(fake_access_list.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 61");
}

#[test]
fn xudt_protocol_burn_access_mode_switch_still_requires_mint_authority() {
    let mut fixture = XudtFixture::new_with_always_success_meta();
    let meta_input = fixture.live_meta_input_with_authority(CONFIG_ACCESS_ENABLED, 0, None);
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
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(
            fixture
                .output_meta_data(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, None)
                .pack(),
        )
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 50");
}

#[test]
fn xudt_protocol_burn_access_mode_switch_does_not_skip_access_checks() {
    let mut fixture = XudtFixture::new_with_always_success_meta();
    let authority = input_lock_authority(fixture.lock.script_hash);
    let meta_input = fixture.live_meta_input_with_authority(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        Some(authority.clone()),
    );
    let udt_input = fixture.live_udt_input(100);
    let access_list =
        fixture.live_access_list_input(full_domain_shard(vec![fixture.other_lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(udt_input)
        .cell_dep(cell_dep(access_list.previous_output()))
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
            fixture
                .output_meta_data(CONFIG_ACCESS_ENABLED, 0, Some(authority))
                .pack(),
        )
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 61");
}
