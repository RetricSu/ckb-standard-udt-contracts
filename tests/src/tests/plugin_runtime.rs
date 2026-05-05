use crate::{
    fixtures::{
        cell_dep_for_script, create_funding_input, create_typed_cell, deploy_script_with_args,
        expect_tx_fail, expect_tx_pass, typed_output,
    },
    metadata_builders::{
        build_xudt_meta_bytes, input_lock_authority, script_hash, udt_amount_bytes, DeployedScript,
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
use ckb_types_120::{packed::Script as MetadataScript, prelude::Entity};
use standard_udt_types::metadata::{ScriptAttr, ScriptLocation, CONFIG_SUPPLY_TRACKED};

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

fn meta_script(context: &mut Context) -> DeployedScript {
    deploy_data2_script(context, "enhanced-xudt-meta", Bytes::from(vec![2u8; 32]))
}

fn xudt_script(context: &mut Context, meta_type_hash: [u8; 32]) -> DeployedScript {
    deploy_data2_script(
        context,
        "enhanced-xudt",
        Bytes::from(meta_type_hash.to_vec()),
    )
}

fn extension_attr(location: ScriptLocation, deployed: &DeployedScript) -> ScriptAttr {
    let script = MetadataScript::from_slice(deployed.script.as_slice()).expect("convert script");
    ScriptAttr {
        location,
        script_hash: deployed.script_hash,
        script: Some(script),
    }
}

fn xudt_meta_data(
    config_flags: u8,
    current_supply: u128,
    mint_authority: Option<ScriptAttr>,
    extensions: Vec<ScriptAttr>,
) -> Bytes {
    build_xudt_meta_bytes(
        config_flags,
        current_supply,
        mint_authority,
        None,
        None,
        extensions,
    )
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
        extensions: Vec<ScriptAttr>,
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

    fn mint_tx(&mut self, meta_input: CellInput, extensions: Vec<ScriptAttr>) -> TransactionView {
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
    let plugin = deploy_script_with_args(&mut fixture.context, "dl-allow", Bytes::new());
    let extension = extension_attr(ScriptLocation::DynamicLinking, &plugin);
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
    let plugin = deploy_script_with_args(&mut fixture.context, "dl-deny", Bytes::new());
    let extension = extension_attr(ScriptLocation::DynamicLinking, &plugin);
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
    let extension = extension_attr(ScriptLocation::Spawn, &plugin);
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
    let extension = extension_attr(ScriptLocation::Spawn, &plugin);
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
fn xudt_mint_extension_receives_mint_authority_checked() {
    let mut fixture = PluginFixture::new();
    let plugin = deploy_script_with_args(
        &mut fixture.context,
        "spawn-allow",
        Bytes::from_static(b"require_mint_checked"),
    );
    let extension = extension_attr(ScriptLocation::Spawn, &plugin);
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 0, vec![extension.clone()]);

    let tx = fixture
        .mint_tx(meta_input, vec![extension])
        .as_advanced_builder()
        .cell_dep(cell_dep_for_script(&plugin))
        .build();

    expect_tx_pass(&fixture.context, &tx);
}
