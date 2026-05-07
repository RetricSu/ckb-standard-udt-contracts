use crate::{
    fixtures::{
        cell_dep, cell_dep_for_script, create_funding_input, create_typed_cell, expect_tx_fail,
        expect_tx_pass, typed_output,
    },
    metadata_builders::{
        build_sudt_meta_bytes, dynamic_linking_authority, input_lock_authority, spawn_authority,
        udt_amount_bytes, DeployedScript,
    },
    test_helpers::{
        always_success_lock_empty as always_success_lock, deploy_data2_script, deploy_data_script,
        non_whitelisted_lock, sudt_meta_script as meta_script, sudt_script as udt_script,
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
use standard_udt_types::metadata::{Authority, CONFIG_SUPPLY_TRACKED};

mod fixture;
use self::fixture::*;

mod burn;
mod lifecycle;
mod mint;
