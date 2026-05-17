use crate::{
    fixtures::{
        cell_dep, cell_dep_for_script, create_funding_input, create_typed_cell,
        expect_tx_fail_with_any_code, expect_tx_fail_with_code, expect_tx_pass, typed_output,
    },
    metadata_builders::{
        build_access_list_shard_bytes, dynamic_linking_authority, input_lock_authority,
        script_hash, spawn_authority, udt_amount_bytes, DeployedScript,
    },
    test_helpers::{
        access_list_script, always_success_lock_empty as always_success_lock,
        always_success_lock_with_hash_type, calculate_type_id, custom_shard, deploy_data2_script,
        deploy_data_script, empty_full_domain_shard as full_domain_shard, fake_data2_script,
        non_whitelisted_lock, xudt_meta_data_with_authorities as xudt_meta_data,
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
use ckb_types::{packed::Script as MetadataScript, prelude::Entity as CkbEntity};
use standard_udt_types::metadata::{
    Authority, Extension, ExtensionType, CONFIG_ACCESS_ENABLED, CONFIG_ACCESS_WHITELIST,
    CONFIG_PAUSED, CONFIG_SUPPLY_TRACKED,
};

mod fixture;
use self::fixture::*;

mod access_list_validation;
mod access_transitions;
mod creation_supply;
mod plugin_authority;
mod validation;
