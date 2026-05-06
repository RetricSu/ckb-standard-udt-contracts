use crate::{
    fixtures::{
        cell_dep_for_script, create_funding_input, create_typed_cell, deploy_script_with_args,
        expect_tx_fail, expect_tx_pass, typed_output,
    },
    metadata_builders::{input_lock_authority, udt_amount_bytes, DeployedScript},
    test_helpers::{
        always_success_lock_empty as always_success_lock,
        deploy_data_script as dynamic_library_script, xudt_meta_data,
        xudt_meta_script as meta_script, xudt_script,
    },
};
use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        core::{TransactionBuilder, TransactionView},
        packed::CellInput,
        prelude::*,
    },
    context::Context,
};
use ckb_types_120::{packed::Script as MetadataScript, prelude::Entity};
use standard_udt_types::metadata::{Extension, ExtensionType, CONFIG_SUPPLY_TRACKED};

fn extension_attr(extension_type: ExtensionType, deployed: &DeployedScript) -> Extension {
    let script = MetadataScript::from_slice(deployed.script.as_slice()).expect("convert script");
    Extension {
        extension_type,
        script,
    }
}

struct PluginFixture {
    context: Context,
    lock: DeployedScript,
    meta: DeployedScript,
    xudt: DeployedScript,
}

impl PluginFixture {
    fn new() -> Self {
        let mut context = Context::default();
        let lock = always_success_lock(&mut context);
        let meta = meta_script(&mut context);
        let xudt = xudt_script(&mut context, meta.script_hash);

        Self {
            context,
            lock,
            meta,
            xudt,
        }
    }

    fn live_udt_input(&mut self, amount: u128) -> CellInput {
        let out_point = create_typed_cell(
            &mut self.context,
            &self.lock.script,
            &self.xudt.script,
            100_000_000_000,
            udt_amount_bytes(amount),
        );
        CellInput::new_builder().previous_output(out_point).build()
    }

    fn live_meta_input(
        &mut self,
        config_flags: u8,
        supply: u128,
        extensions: Vec<Extension>,
    ) -> CellInput {
        let out_point = create_typed_cell(
            &mut self.context,
            &self.lock.script,
            &self.meta.script,
            100_000_000_000,
            xudt_meta_data(
                config_flags,
                supply,
                Some(input_lock_authority(self.lock.script_hash)),
                extensions,
            ),
        );
        CellInput::new_builder().previous_output(out_point).build()
    }

    fn transfer_tx(&mut self, meta_input: CellInput, udt_input: CellInput) -> TransactionView {
        let tx = TransactionBuilder::default()
            .input(meta_input)
            .input(udt_input)
            .output(typed_output(
                &self.lock.script,
                &self.meta.script,
                100_000_000_000,
            ))
            .output(typed_output(
                &self.lock.script,
                &self.xudt.script,
                100_000_000_000,
            ))
            .output_data(xudt_meta_data(0, 0, None, Vec::new()).pack())
            .output_data(udt_amount_bytes(100).pack())
            .build();
        self.complete(tx)
    }

    fn mint_tx(&mut self, meta_input: CellInput, extensions: Vec<Extension>) -> TransactionView {
        let funding = create_funding_input(&mut self.context, &self.lock.script, 100_000_000_000);
        let tx = TransactionBuilder::default()
            .input(meta_input)
            .input(funding)
            .output(typed_output(
                &self.lock.script,
                &self.meta.script,
                100_000_000_000,
            ))
            .output(typed_output(
                &self.lock.script,
                &self.xudt.script,
                100_000_000_000,
            ))
            .output_data(
                xudt_meta_data(
                    CONFIG_SUPPLY_TRACKED,
                    50,
                    Some(input_lock_authority(self.lock.script_hash)),
                    extensions,
                )
                .pack(),
            )
            .output_data(udt_amount_bytes(50).pack())
            .build();
        self.complete(tx)
    }

    fn protocol_burn_tx(
        &mut self,
        meta_input: CellInput,
        extensions: Vec<Extension>,
    ) -> TransactionView {
        let udt_input = self.live_udt_input(100);
        let tx = TransactionBuilder::default()
            .input(meta_input)
            .input(udt_input)
            .output(typed_output(
                &self.lock.script,
                &self.meta.script,
                100_000_000_000,
            ))
            .output(typed_output(
                &self.lock.script,
                &self.xudt.script,
                100_000_000_000,
            ))
            .output_data(
                xudt_meta_data(
                    CONFIG_SUPPLY_TRACKED,
                    50,
                    Some(input_lock_authority(self.lock.script_hash)),
                    extensions,
                )
                .pack(),
            )
            .output_data(udt_amount_bytes(50).pack())
            .build();
        self.complete(tx)
    }

    fn complete(&mut self, tx: TransactionView) -> TransactionView {
        let tx = tx
            .as_advanced_builder()
            .cell_dep(cell_dep_for_script(&self.lock))
            .cell_dep(cell_dep_for_script(&self.meta))
            .cell_dep(cell_dep_for_script(&self.xudt))
            .build();
        self.context.complete_tx(tx)
    }
}

#[test]
fn xudt_extension_allow_plugin_passes() {
    let mut fixture = PluginFixture::new();
    let plugin = dynamic_library_script(&mut fixture.context, "dl-shared-allow", Bytes::new());
    let extension = extension_attr(ExtensionType::DynamicLinking, &plugin);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension.clone()]);
    let udt_input = fixture.live_udt_input(100);

    let tx = fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_extension_deny_plugin_rejects() {
    let mut fixture = PluginFixture::new();
    let plugin = dynamic_library_script(&mut fixture.context, "dl-shared-deny", Bytes::new());
    let extension = extension_attr(ExtensionType::DynamicLinking, &plugin);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension]);
    let udt_input = fixture.live_udt_input(100);

    let tx = fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn xudt_executable_dynamic_linking_fixture_fails_closed() {
    let mut fixture = PluginFixture::new();
    let plugin = deploy_script_with_args(&mut fixture.context, "dl-deny", Bytes::new());
    let extension = extension_attr(ExtensionType::DynamicLinking, &plugin);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension]);
    let udt_input = fixture.live_udt_input(100);

    let tx = fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn xudt_spawn_extension_allow_plugin_passes() {
    let mut fixture = PluginFixture::new();
    let plugin = deploy_script_with_args(&mut fixture.context, "spawn-allow", Bytes::new());
    let extension = extension_attr(ExtensionType::Spawn, &plugin);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension.clone()]);
    let udt_input = fixture.live_udt_input(100);

    let tx = fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_spawn_extension_deny_plugin_rejects() {
    let mut fixture = PluginFixture::new();
    let plugin = deploy_script_with_args(&mut fixture.context, "spawn-deny", Bytes::new());
    let extension = extension_attr(ExtensionType::Spawn, &plugin);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension]);
    let udt_input = fixture.live_udt_input(100);

    let tx = fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn xudt_transfer_extension_receives_mint_authority_none() {
    let mut fixture = PluginFixture::new();
    let plugin = dynamic_library_script(
        &mut fixture.context,
        "dl-shared-allow",
        Bytes::from_static(b"require_mint_none"),
    );
    let extension = extension_attr(ExtensionType::DynamicLinking, &plugin);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension.clone()]);
    let udt_input = fixture.live_udt_input(100);

    let tx = fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_spawn_transfer_extension_receives_mint_authority_none() {
    let mut fixture = PluginFixture::new();
    let plugin = deploy_script_with_args(
        &mut fixture.context,
        "spawn-allow",
        Bytes::from_static(b"require_mint_none"),
    );
    let extension = extension_attr(ExtensionType::Spawn, &plugin);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension.clone()]);
    let udt_input = fixture.live_udt_input(100);

    let tx = fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_mint_extension_receives_mint_authority_checked() {
    let mut fixture = PluginFixture::new();
    let plugin = dynamic_library_script(
        &mut fixture.context,
        "dl-shared-allow",
        Bytes::from_static(b"require_mint_checked"),
    );
    let extension = extension_attr(ExtensionType::DynamicLinking, &plugin);
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 0, vec![extension.clone()]);

    let tx = fixture
        .mint_tx(meta_input, vec![extension])
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_protocol_burn_extension_receives_mint_authority_none() {
    let mut fixture = PluginFixture::new();
    let plugin = dynamic_library_script(
        &mut fixture.context,
        "dl-shared-allow",
        Bytes::from_static(b"require_mint_none"),
    );
    let extension = extension_attr(ExtensionType::DynamicLinking, &plugin);
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, vec![extension.clone()]);

    let tx = fixture
        .protocol_burn_tx(meta_input, vec![extension])
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_pass(&fixture.context, &tx);
}
