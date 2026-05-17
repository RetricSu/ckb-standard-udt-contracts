use crate::{
    fixtures::{
        cell_dep, cell_dep_for_script, create_funding_input, create_typed_cell,
        expect_tx_fail_with_code, expect_tx_pass, normal_output, typed_output,
    },
    metadata_builders::{
        dynamic_linking_authority, input_lock_authority, script_hash, spawn_authority,
        udt_amount_bytes, DeployedScript,
    },
    test_helpers::{
        access_list_script, always_success_lock, calculate_type_id, custom_shard,
        deploy_data2_script, deploy_data_script, exact_shard, full_domain_shard,
        non_whitelisted_lock, xudt_meta_data, xudt_meta_data_with_authorities,
        xudt_meta_script as meta_script, xudt_script,
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

mod access;
mod burn;
mod lifecycle;
mod mint;

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
