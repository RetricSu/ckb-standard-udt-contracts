use crate::{
    fixtures::{
        cell_dep_for_script, create_funding_input, create_typed_cell, expect_tx_fail,
        expect_tx_fail_with_code, expect_tx_pass, typed_output,
    },
    metadata_builders::{
        build_sudt_meta_bytes, dynamic_linking_authority as deployed_dynamic_linking_authority,
        input_lock_authority, script_hash, spawn_authority, udt_amount_bytes, DeployedScript,
    },
    test_helpers::{
        always_success_lock_empty as always_success_lock, always_success_lock_with_hash_type,
        calculate_type_id, deploy_data2_script, deploy_data_script, fake_data2_script,
        non_whitelisted_lock, sudt_meta_script as meta_script, sudt_script as udt_script,
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
use standard_udt_types::metadata::{Authority, SudtMeta, CONFIG_SUPPLY_TRACKED};

mod fixture;
use self::fixture::*;

mod create;
mod metadata_authority;
mod plugin_authority;
mod supply;
