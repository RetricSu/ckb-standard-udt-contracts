use crate::{
    fixtures::{
        cell_dep, cell_dep_for_script, create_funding_input, create_typed_cell,
        deploy_script_with_args, expect_tx_fail_with_code, expect_tx_pass, typed_output,
    },
    metadata_builders::{input_lock_authority, udt_amount_bytes, DeployedScript},
    test_helpers::{
        always_success_lock, always_success_lock_empty,
        deploy_data_script as dynamic_library_script, xudt_meta_data,
        xudt_meta_script as meta_script, xudt_script,
    },
};
use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        core::{ScriptHashType, TransactionBuilder, TransactionView},
        packed::{CellInput, CellOutput},
        prelude::*,
    },
    context::Context,
};
use ckb_types::{packed::Script as MetadataScript, prelude::Entity as CkbEntity};
use standard_udt_types::metadata::{Extension, ExtensionType, CONFIG_SUPPLY_TRACKED};

use crate::Loader;

fn extension_attr(extension_type: ExtensionType, deployed: &DeployedScript) -> Extension {
    let script = MetadataScript::from_slice(deployed.script.as_slice()).expect("convert script");
    Extension {
        extension_type,
        script,
    }
}

fn extension_from_script(extension_type: ExtensionType, script: &MetadataScript) -> Extension {
    Extension {
        extension_type,
        script: script.clone(),
    }
}

fn metadata_script(script: &ckb_testtool::ckb_types::packed::Script) -> MetadataScript {
    MetadataScript::from_slice(script.as_slice()).expect("convert script")
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
        let lock = always_success_lock_empty(&mut context);
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

fn deploy_type_hash_dynamic_library(
    fixture: &mut PluginFixture,
    binary_name: &str,
    args: Bytes,
) -> DeployedScript {
    let out_point = fixture.context.create_cell(
        CellOutput::new_builder()
            .capacity(100_000_000_000u64.pack())
            .lock(fixture.lock.script.clone())
            .type_(Some(fixture.lock.script.clone()).pack())
            .build(),
        Loader::default().load_binary(binary_name),
    );
    let script = fixture
        .context
        .build_script_with_hash_type(&out_point, ScriptHashType::Type, args)
        .expect("build Type dynamic library script");
    DeployedScript {
        out_point,
        script_hash: script.calc_script_hash().unpack(),
        script,
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
fn xudt_dynamic_linking_extension_loads_type_hash_cell_dep() {
    let mut fixture = PluginFixture::new();
    let plugin = deploy_type_hash_dynamic_library(&mut fixture, "dl-shared-allow", Bytes::new());
    let extension = extension_attr(ExtensionType::DynamicLinking, &plugin);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension]);
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

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 70");
}

#[test]
fn xudt_input_lock_extension_requires_matching_input_lock() {
    let mut fixture = PluginFixture::new();
    let extension = extension_attr(ExtensionType::InputLock, &fixture.lock);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension]);
    let udt_input = fixture.live_udt_input(100);
    let tx = fixture.transfer_tx(meta_input, udt_input);

    expect_tx_pass(&fixture.context, &tx);

    let mut missing = PluginFixture::new();
    let other_lock = always_success_lock(&mut missing.context, Bytes::from_static(b"other-lock"));
    let extension = extension_attr(ExtensionType::InputLock, &other_lock);
    let meta_input = missing.live_meta_input(0, 0, vec![extension]);
    let udt_input = missing.live_udt_input(100);
    let tx = missing.transfer_tx(meta_input, udt_input);

    expect_tx_fail_with_code(&missing.context, &tx, "error code 70");
}

#[test]
fn xudt_input_type_extension_requires_matching_input_type() {
    let mut fixture = PluginFixture::new();
    let xudt_script = metadata_script(&fixture.xudt.script);
    let extension = extension_from_script(ExtensionType::InputType, &xudt_script);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension]);
    let udt_input = fixture.live_udt_input(100);
    let tx = fixture.transfer_tx(meta_input, udt_input);

    expect_tx_pass(&fixture.context, &tx);

    let mut missing = PluginFixture::new();
    let other_lock = always_success_lock(&mut missing.context, Bytes::from_static(b"other-type"));
    let extension = extension_attr(ExtensionType::InputType, &other_lock);
    let meta_input = missing.live_meta_input(0, 0, vec![extension]);
    let udt_input = missing.live_udt_input(100);
    let tx = missing.transfer_tx(meta_input, udt_input);

    expect_tx_fail_with_code(&missing.context, &tx, "error code 70");
}

#[test]
fn xudt_output_type_extension_requires_matching_output_type() {
    let mut fixture = PluginFixture::new();
    let xudt_script = metadata_script(&fixture.xudt.script);
    let extension = extension_from_script(ExtensionType::OutputType, &xudt_script);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension]);
    let udt_input = fixture.live_udt_input(100);
    let tx = fixture.transfer_tx(meta_input, udt_input);

    expect_tx_pass(&fixture.context, &tx);

    let mut missing = PluginFixture::new();
    let other_lock = always_success_lock(&mut missing.context, Bytes::from_static(b"other-type"));
    let extension = extension_attr(ExtensionType::OutputType, &other_lock);
    let meta_input = missing.live_meta_input(0, 0, vec![extension]);
    let udt_input = missing.live_udt_input(100);
    let tx = missing.transfer_tx(meta_input, udt_input);

    expect_tx_fail_with_code(&missing.context, &tx, "error code 70");
}

#[test]
fn xudt_presence_extensions_apply_to_mint() {
    let mut fixture = PluginFixture::new();
    let xudt_script = metadata_script(&fixture.xudt.script);
    let extensions = vec![
        extension_attr(ExtensionType::InputLock, &fixture.lock),
        extension_from_script(
            ExtensionType::InputType,
            &metadata_script(&fixture.meta.script),
        ),
        extension_from_script(ExtensionType::OutputType, &xudt_script),
    ];
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 0, extensions.clone());
    let tx = fixture.mint_tx(meta_input, extensions);

    expect_tx_pass(&fixture.context, &tx);

    let mut missing = PluginFixture::new();
    let other_lock = always_success_lock(&mut missing.context, Bytes::from_static(b"other-lock"));
    let extensions = vec![extension_attr(ExtensionType::InputLock, &other_lock)];
    let meta_input = missing.live_meta_input(CONFIG_SUPPLY_TRACKED, 0, extensions.clone());
    let tx = missing.mint_tx(meta_input, extensions);

    expect_tx_fail_with_code(&missing.context, &tx, "error code 70");
}

#[test]
fn xudt_presence_extensions_apply_to_protocol_burn() {
    let mut fixture = PluginFixture::new();
    let xudt_script = metadata_script(&fixture.xudt.script);
    let extensions = vec![
        extension_attr(ExtensionType::InputLock, &fixture.lock),
        extension_from_script(ExtensionType::InputType, &xudt_script),
        extension_from_script(ExtensionType::OutputType, &xudt_script),
    ];
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, extensions.clone());
    let tx = fixture.protocol_burn_tx(meta_input, extensions);

    expect_tx_pass(&fixture.context, &tx);

    let mut missing = PluginFixture::new();
    let other_lock = always_success_lock(&mut missing.context, Bytes::from_static(b"other-type"));
    let extensions = vec![extension_attr(ExtensionType::OutputType, &other_lock)];
    let meta_input = missing.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, extensions.clone());
    let tx = missing.protocol_burn_tx(meta_input, extensions);

    expect_tx_fail_with_code(&missing.context, &tx, "error code 70");
}

#[test]
fn xudt_user_destruction_skips_presence_extensions() {
    let mut fixture = PluginFixture::new();
    let other_lock = always_success_lock(&mut fixture.context, Bytes::from_static(b"other-lock"));
    let meta_dep = fixture.live_meta_input(
        0,
        0,
        vec![extension_attr(ExtensionType::InputLock, &other_lock)],
    );
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_executable_extension_index_counts_presence_extensions() {
    let mut fixture = PluginFixture::new();
    let plugin = dynamic_library_script(
        &mut fixture.context,
        "dl-shared-allow",
        Bytes::from_static(b"require_index_1"),
    );
    let extensions = vec![
        extension_attr(ExtensionType::InputLock, &fixture.lock),
        extension_attr(ExtensionType::DynamicLinking, &plugin),
    ];
    let meta_input = fixture.live_meta_input(0, 0, extensions);
    let udt_input = fixture.live_udt_input(100);

    let tx = fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_executable_dynamic_linking_fixture_fails_closed() {
    let mut fixture = PluginFixture::new();
    let plugin = deploy_script_with_args(&mut fixture.context, "dl-allow", Bytes::new());
    let extension = extension_attr(ExtensionType::DynamicLinking, &plugin);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension]);
    let udt_input = fixture.live_udt_input(100);

    let tx = fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 70");
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

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 70");
}

#[test]
fn xudt_dynamic_linking_extension_rejects_wrong_operation() {
    let mut fixture = PluginFixture::new();
    let plugin = dynamic_library_script(
        &mut fixture.context,
        "dl-shared-allow",
        Bytes::from_static(b"require_mint"),
    );
    let extension = extension_attr(ExtensionType::DynamicLinking, &plugin);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension.clone()]);
    let udt_input = fixture.live_udt_input(100);

    let tx = fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 70");
}

#[test]
fn xudt_dynamic_linking_extension_checks_extension_index() {
    let mut fixture = PluginFixture::new();
    let plugin = dynamic_library_script(
        &mut fixture.context,
        "dl-shared-allow",
        Bytes::from_static(b"require_index_1"),
    );
    let extension = extension_attr(ExtensionType::DynamicLinking, &plugin);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension.clone()]);
    let udt_input = fixture.live_udt_input(100);

    let tx = fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 70");
}

#[test]
fn xudt_dynamic_linking_extension_accepts_expected_operation_and_index() {
    let mut fixture = PluginFixture::new();
    let transfer_plugin = dynamic_library_script(
        &mut fixture.context,
        "dl-shared-allow",
        Bytes::from_static(b"require_transfer"),
    );
    let transfer_extension = extension_attr(ExtensionType::DynamicLinking, &transfer_plugin);
    let meta_input = fixture.live_meta_input(0, 0, vec![transfer_extension]);
    let udt_input = fixture.live_udt_input(100);

    let tx = fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&transfer_plugin))
        .build();

    expect_tx_pass(&fixture.context, &tx);

    let mut index_fixture = PluginFixture::new();
    let index_plugin = dynamic_library_script(
        &mut index_fixture.context,
        "dl-shared-allow",
        Bytes::from_static(b"require_index_0"),
    );
    let index_extension = extension_attr(ExtensionType::DynamicLinking, &index_plugin);
    let meta_input = index_fixture.live_meta_input(0, 0, vec![index_extension]);
    let udt_input = index_fixture.live_udt_input(100);
    let tx = index_fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&index_plugin))
        .build();

    expect_tx_pass(&index_fixture.context, &tx);
}

#[test]
fn xudt_spawn_extension_rejects_wrong_operation() {
    let mut fixture = PluginFixture::new();
    let plugin = deploy_script_with_args(
        &mut fixture.context,
        "spawn-allow",
        Bytes::from_static(b"require_mint"),
    );
    let extension = extension_attr(ExtensionType::Spawn, &plugin);
    let meta_input = fixture.live_meta_input(0, 0, vec![extension.clone()]);
    let udt_input = fixture.live_udt_input(100);

    let tx = fixture
        .transfer_tx(meta_input, udt_input)
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 70");
}

#[test]
fn xudt_spawn_extension_accepts_expected_operation() {
    let mut fixture = PluginFixture::new();
    let plugin = deploy_script_with_args(
        &mut fixture.context,
        "spawn-allow",
        Bytes::from_static(b"require_transfer"),
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
