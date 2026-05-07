use super::*;

pub(super) fn tracked_meta_data(current_supply: u128, lock_hash: Option<[u8; 32]>) -> Bytes {
    tracked_meta_data_with_authority(current_supply, lock_hash.map(input_lock_authority))
}

pub(super) fn tracked_meta_data_with_authority(
    current_supply: u128,
    mint_authority: Option<Authority>,
) -> Bytes {
    build_sudt_meta_bytes(CONFIG_SUPPLY_TRACKED, current_supply, mint_authority, None)
}

pub(super) fn untracked_meta_data(mint_authority: Option<Authority>) -> Bytes {
    build_sudt_meta_bytes(0, 0, mint_authority, None)
}

pub(super) struct SudtFixture {
    pub(super) context: Context,
    pub(super) lock: DeployedScript,
    pub(super) meta: DeployedScript,
    pub(super) udt: DeployedScript,
}

impl SudtFixture {
    pub(super) fn new() -> Self {
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

    pub(super) fn live_udt_input(&mut self, amount: u128) -> CellInput {
        let out_point = create_typed_cell(
            &mut self.context,
            &self.lock.script,
            &self.udt.script,
            100_000_000_000,
            udt_amount_bytes(amount),
        );
        CellInput::new_builder().previous_output(out_point).build()
    }

    pub(super) fn live_meta_input(&mut self, supply: u128, authorized: bool) -> CellInput {
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

    pub(super) fn live_meta_dep(&mut self, supply: u128, authorized: bool) -> CellInput {
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

    pub(super) fn live_meta_dep_with_authority(
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

    pub(super) fn live_meta_input_with_authority(
        &mut self,
        supply: u128,
        mint_authority: Option<Authority>,
    ) -> CellInput {
        self.live_meta_dep_with_authority(supply, mint_authority)
    }

    pub(super) fn complete(&mut self, tx: TransactionView) -> TransactionView {
        let tx = tx
            .as_advanced_builder()
            .cell_dep(cell_dep_for_script(&self.lock))
            .cell_dep(cell_dep_for_script(&self.meta))
            .cell_dep(cell_dep_for_script(&self.udt))
            .build();
        self.context.complete_tx(tx)
    }
}

pub(super) fn sudt_mint_with_plugin_authority(plugin_name: &str, spawn: bool) -> bool {
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
