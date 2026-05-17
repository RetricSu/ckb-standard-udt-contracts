use crate::{
    fixtures::{
        cell_dep, cell_dep_for_script, create_typed_cell, expect_tx_fail_with_code, expect_tx_pass,
        typed_output,
    },
    metadata_builders::{
        dynamic_linking_authority, input_lock_authority, spawn_authority, DeployedScript,
    },
    test_helpers::{
        access_list_script, always_success_lock, always_success_lock_with_hash_type,
        bounded_suffix_shard as bounded_shard, custom_shard, deploy_data2_script,
        deploy_data_script, entry, full_domain_shard, non_whitelisted_lock, numbered_entries,
        prefix_end, prefix_entry, prefix_start, tail_suffix_shard as tail_shard,
        xudt_meta_data_with_authorities as build_xudt_meta_data, xudt_meta_script as meta_script,
    },
    verify_and_dump_failed_tx,
};
use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        core::{ScriptHashType, TransactionBuilder, TransactionView},
        packed::CellInput,
        prelude::*,
    },
    context::Context,
};
use standard_udt_types::metadata::{Authority, CONFIG_ACCESS_ENABLED, CONFIG_ACCESS_WHITELIST};

mod fixture;
use fixture::*;

mod authority;
mod lifecycle;
mod shards;
mod xudt_integration;
