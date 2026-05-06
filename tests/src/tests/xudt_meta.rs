use crate::{
    fixtures::{
        cell_dep, cell_dep_for_script, create_typed_cell, expect_tx_fail, expect_tx_fail_with_code,
        expect_tx_pass, typed_output,
    },
    metadata_builders::{
        build_access_list_shard_bytes, dynamic_linking_authority, input_lock_authority,
        spawn_authority, udt_amount_bytes, DeployedScript,
    },
    test_helpers::{
        access_list_script, always_success_lock_empty as always_success_lock, custom_shard,
        deploy_data2_script, deploy_data_script, empty_full_domain_shard as full_domain_shard,
        non_whitelisted_lock, xudt_meta_data_with_authorities as xudt_meta_data,
        xudt_meta_script as meta_script, xudt_script,
    },
};
use ckb_testtool::{
    ckb_types::{
        bytes::Bytes,
        core::{TransactionBuilder, TransactionView},
        packed::{CellInput, Script},
        prelude::*,
    },
    context::Context,
};
use standard_udt_types::metadata::{CONFIG_ACCESS_ENABLED, CONFIG_ACCESS_WHITELIST, CONFIG_PAUSED};

fn with_config_flags(data: Bytes, config_flags: u8) -> Bytes {
    let mut data = data.to_vec();
    let config_offset = u32::from_le_bytes(data[4..8].try_into().expect("config offset")) as usize;
    data[config_offset] = config_flags;
    Bytes::from(data)
}

fn malformed_name_meta_data() -> Bytes {
    let name = 1u32.to_le_bytes().to_vec();
    replace_xudt_meta_table_field(xudt_meta_data(0, 0, None, None, None, Vec::new()), 3, &name)
}

fn oversized_name_meta_data() -> Bytes {
    let mut name = 1025u32.to_le_bytes().to_vec();
    name.extend_from_slice(&vec![0u8; 1025]);
    replace_xudt_meta_table_field(xudt_meta_data(0, 0, None, None, None, Vec::new()), 3, &name)
}

fn replace_xudt_meta_table_field(data: Bytes, field_index: usize, replacement: &[u8]) -> Bytes {
    let data = data.to_vec();
    let first_offset = read_u32(&data, 4) as usize;
    let field_count = first_offset / 4 - 1;
    let mut offsets = Vec::with_capacity(field_count + 1);
    for index in 0..field_count {
        offsets.push(read_u32(&data, 4 + index * 4) as usize);
    }
    offsets.push(data.len());

    let old_start = offsets[field_index];
    let old_end = offsets[field_index + 1];
    let delta = replacement.len() as isize - (old_end - old_start) as isize;
    let new_total = (data.len() as isize + delta) as usize;

    let mut result = Vec::with_capacity(new_total);
    result.extend_from_slice(&new_total.to_le_bytes()[..4]);
    for index in 0..field_count {
        let offset = if index <= field_index {
            offsets[index]
        } else {
            (offsets[index] as isize + delta) as usize
        };
        result.extend_from_slice(&(offset as u32).to_le_bytes());
    }
    result.extend_from_slice(&data[first_offset..old_start]);
    result.extend_from_slice(replacement);
    result.extend_from_slice(&data[old_end..]);

    Bytes::from(result)
}

fn read_u32(data: &[u8], start: usize) -> u32 {
    u32::from_le_bytes(data[start..start + 4].try_into().expect("u32 field"))
}

fn half_domain_shard() -> Bytes {
    custom_shard([0u8; 32], [0x7fu8; 32], Vec::new())
}

fn access_list_shard_with_extra_field() -> Bytes {
    let mut data = Vec::new();
    data.extend_from_slice(&84u32.to_le_bytes());
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&80u32.to_le_bytes());
    data.extend_from_slice(&84u32.to_le_bytes());
    data.extend_from_slice(&[0u8; 32]);
    data.extend_from_slice(&[0xffu8; 32]);
    data.extend_from_slice(&0u32.to_le_bytes());
    Bytes::from(data)
}

struct UpdateCase {
    context: Context,
    tx: TransactionView,
}

fn update_meta_tx<F>(build: F) -> UpdateCase
where
    F: FnOnce(&mut Context, &DeployedScript, &DeployedScript) -> (Bytes, Bytes, Vec<ExtraCell>),
{
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta = meta_script(&mut context);
    let (input_meta_data, output_meta_data, extra_cells) = build(&mut context, &lock, &meta);

    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        input_meta_data,
    );
    let mut builder = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(output_meta_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta));

    for extra in extra_cells {
        match extra {
            ExtraCell::Output {
                lock,
                type_script,
                data,
                cell_dep,
            } => {
                builder = builder
                    .output(typed_output(&lock, &type_script, 100_000_000_000))
                    .output_data(data.pack())
                    .cell_dep(cell_dep_for_script(&cell_dep));
            }
            ExtraCell::Input {
                previous_output,
                cell_dep,
            } => {
                builder = builder
                    .input(
                        CellInput::new_builder()
                            .previous_output(previous_output)
                            .build(),
                    )
                    .cell_dep(cell_dep_for_script(&cell_dep));
            }
            ExtraCell::Dep { cell_dep } => {
                builder = builder.cell_dep(cell_dep_for_script(&cell_dep));
            }
            ExtraCell::CellDep { previous_output } => {
                builder = builder.cell_dep(cell_dep(previous_output));
            }
        }
    }

    let tx = context.complete_tx(builder.build());
    UpdateCase { context, tx }
}

fn access_mode_transition_tx(
    input_flags: u8,
    output_flags: u8,
    include_full_input: bool,
    include_full_output: bool,
) -> UpdateCase {
    update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let mut extra_cells = Vec::new();

        if include_full_input {
            let access_list = access_list_script(context, meta.script_hash);
            extra_cells.push(ExtraCell::Input {
                previous_output: create_typed_cell(
                    context,
                    &lock.script,
                    &access_list.script,
                    100_000_000_000,
                    full_domain_shard(),
                ),
                cell_dep: access_list,
            });
        }

        if include_full_output {
            let access_list = access_list_script(context, meta.script_hash);
            extra_cells.push(ExtraCell::Output {
                lock: lock.script.clone(),
                type_script: access_list.script.clone(),
                data: full_domain_shard(),
                cell_dep: access_list,
            });
        }

        (
            xudt_meta_data(
                input_flags,
                0,
                None,
                None,
                Some(authority.clone()),
                Vec::new(),
            ),
            xudt_meta_data(output_flags, 0, None, None, Some(authority), Vec::new()),
            extra_cells,
        )
    })
}

fn update_meta_tx_with_output_lock<F>(build_lock: F) -> UpdateCase
where
    F: FnOnce(&mut Context) -> DeployedScript,
{
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let output_lock = build_lock(&mut context);
    let meta = meta_script(&mut context);
    let input = xudt_meta_data(
        0,
        0,
        None,
        Some(input_lock_authority(lock.script_hash)),
        None,
        Vec::new(),
    );
    let output = xudt_meta_data(
        0,
        0,
        None,
        Some(input_lock_authority(lock.script_hash)),
        None,
        Vec::new(),
    );

    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        input,
    );
    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .output(typed_output(
            &output_lock.script,
            &meta.script,
            100_000_000_000,
        ))
        .output_data(output.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&output_lock))
        .cell_dep(cell_dep_for_script(&meta))
        .build();
    let tx = context.complete_tx(tx);
    UpdateCase { context, tx }
}

enum ExtraCell {
    Output {
        lock: Script,
        type_script: Script,
        data: Bytes,
        cell_dep: DeployedScript,
    },
    Input {
        previous_output: ckb_testtool::ckb_types::packed::OutPoint,
        cell_dep: DeployedScript,
    },
    Dep {
        cell_dep: DeployedScript,
    },
    CellDep {
        previous_output: ckb_testtool::ckb_types::packed::OutPoint,
    },
}

#[test]
fn xudt_meta_rejects_invalid_config_flags() {
    let case = update_meta_tx(|_, _, _| {
        let input = xudt_meta_data(0, 0, None, None, None, Vec::new());
        let output = with_config_flags(input.clone(), CONFIG_ACCESS_WHITELIST);
        (input, output, Vec::new())
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 30");
}

#[test]
fn xudt_meta_rejects_malformed_name_bytes_field() {
    let case = update_meta_tx(|_, _, _| {
        let data = malformed_name_meta_data();
        (data.clone(), data, Vec::new())
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 30");
}

#[test]
fn xudt_meta_rejects_oversized_name_field() {
    let case = update_meta_tx(|_, _, _| {
        let data = oversized_name_meta_data();
        (data.clone(), data, Vec::new())
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 30");
}

#[test]
fn xudt_meta_rejects_non_whitelisted_output_lock() {
    let case = update_meta_tx_with_output_lock(non_whitelisted_lock);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 20");
}

#[test]
fn xudt_meta_blacklist_to_whitelist_requires_full_domain_inputs_and_outputs() {
    let missing_input = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        false,
        true,
    );
    expect_tx_fail(&missing_input.context, &missing_input.tx);

    let missing_output = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        false,
    );
    expect_tx_fail(&missing_output.context, &missing_output.tx);

    let full_replace = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        true,
    );
    expect_tx_pass(&full_replace.context, &full_replace.tx);
}

#[test]
fn xudt_meta_whitelist_to_blacklist_requires_full_domain_inputs_and_outputs() {
    let missing_input = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED,
        false,
        true,
    );
    expect_tx_fail(&missing_input.context, &missing_input.tx);

    let missing_output = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED,
        true,
        false,
    );
    expect_tx_fail(&missing_output.context, &missing_output.tx);

    let full_replace = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED,
        true,
        true,
    );
    expect_tx_pass(&full_replace.context, &full_replace.tx);
}

#[test]
fn xudt_meta_whitelist_to_disabled_requires_full_domain_inputs() {
    let missing_input = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        false,
        false,
    );
    expect_tx_fail_with_code(&missing_input.context, &missing_input.tx, "error code 60");

    let full_input = access_mode_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        true,
        false,
    );
    expect_tx_pass(&full_input.context, &full_input.tx);
}

#[test]
fn xudt_meta_active_transition_rejects_partial_access_list_domain() {
    let partial_input = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let input_access_list = access_list_script(context, meta.script_hash);
        let output_access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
                0,
                None,
                None,
                Some(authority.clone()),
                Vec::new(),
            ),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            vec![
                ExtraCell::Input {
                    previous_output: create_typed_cell(
                        context,
                        &lock.script,
                        &input_access_list.script,
                        100_000_000_000,
                        half_domain_shard(),
                    ),
                    cell_dep: input_access_list,
                },
                ExtraCell::Output {
                    lock: lock.script.clone(),
                    type_script: output_access_list.script.clone(),
                    data: full_domain_shard(),
                    cell_dep: output_access_list,
                },
            ],
        )
    });

    expect_tx_fail_with_code(&partial_input.context, &partial_input.tx, "error code 60");
}

#[test]
fn xudt_meta_disabled_to_blacklist_requires_full_domain_shards() {
    let missing_shard = update_meta_tx(|_, lock, _| {
        let authority = input_lock_authority(lock.script_hash);
        (
            xudt_meta_data(0, 0, None, None, Some(authority.clone()), Vec::new()),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            Vec::new(),
        )
    });
    expect_tx_fail_with_code(&missing_shard.context, &missing_shard.tx, "error code 60");

    let with_shard = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(0, 0, None, None, Some(authority.clone()), Vec::new()),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            vec![ExtraCell::Output {
                lock: lock.script.clone(),
                type_script: access_list.script.clone(),
                data: full_domain_shard(),
                cell_dep: access_list,
            }],
        )
    });
    expect_tx_pass(&with_shard.context, &with_shard.tx);
}

#[test]
fn xudt_meta_disabled_to_whitelist_requires_one_shard() {
    let missing_shard = update_meta_tx(|_, lock, _| {
        let authority = input_lock_authority(lock.script_hash);
        (
            xudt_meta_data(0, 0, None, None, Some(authority.clone()), Vec::new()),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            Vec::new(),
        )
    });
    expect_tx_fail_with_code(&missing_shard.context, &missing_shard.tx, "error code 60");

    let with_shard = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(0, 0, None, None, Some(authority.clone()), Vec::new()),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            vec![ExtraCell::Output {
                lock: lock.script.clone(),
                type_script: access_list.script.clone(),
                data: full_domain_shard(),
                cell_dep: access_list,
            }],
        )
    });
    expect_tx_pass(&with_shard.context, &with_shard.tx);
}

#[test]
fn xudt_meta_access_mode_switch_rejects_same_token_xudt_cells() {
    let case = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let access_list = access_list_script(context, meta.script_hash);
        let input_xudt = xudt_script(context, meta.script_hash);
        let output_xudt = xudt_script(context, meta.script_hash);
        let access_list_dep = create_typed_cell(
            context,
            &lock.script,
            &access_list.script,
            100_000_000_000,
            full_domain_shard(),
        );
        let previous_output = create_typed_cell(
            context,
            &lock.script,
            &input_xudt.script,
            100_000_000_000,
            udt_amount_bytes(1),
        );
        (
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED,
                0,
                None,
                None,
                Some(authority.clone()),
                Vec::new(),
            ),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            vec![
                ExtraCell::Output {
                    lock: lock.script.clone(),
                    type_script: access_list.script.clone(),
                    data: full_domain_shard(),
                    cell_dep: access_list,
                },
                ExtraCell::Input {
                    previous_output,
                    cell_dep: input_xudt,
                },
                ExtraCell::Output {
                    lock: lock.script.clone(),
                    type_script: output_xudt.script.clone(),
                    data: udt_amount_bytes(1),
                    cell_dep: output_xudt,
                },
                ExtraCell::CellDep {
                    previous_output: access_list_dep,
                },
            ],
        )
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 61");
}

#[test]
fn xudt_meta_access_authority_controls_pause_and_access_mode() {
    let without_authority = update_meta_tx(|_, _, _| {
        (
            xudt_meta_data(0, 0, None, None, None, Vec::new()),
            xudt_meta_data(CONFIG_PAUSED, 0, None, None, None, Vec::new()),
            Vec::new(),
        )
    });
    expect_tx_fail_with_code(
        &without_authority.context,
        &without_authority.tx,
        "error code 50",
    );

    let with_authority = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(0, 0, None, None, Some(authority.clone()), Vec::new()),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST | CONFIG_PAUSED,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            vec![ExtraCell::Output {
                lock: lock.script.clone(),
                type_script: access_list.script.clone(),
                data: full_domain_shard(),
                cell_dep: access_list,
            }],
        )
    });
    expect_tx_pass(&with_authority.context, &with_authority.tx);
}

fn xudt_meta_access_update_with_plugin_authority(plugin_name: &str, spawn: bool) -> UpdateCase {
    update_meta_tx(|context, lock, meta| {
        let plugin = if spawn {
            deploy_data2_script(context, plugin_name, Bytes::from_static(b"allow"))
        } else {
            deploy_data_script(context, plugin_name, Bytes::from_static(b"allow"))
        };
        let authority = if spawn {
            spawn_authority(&plugin)
        } else {
            dynamic_linking_authority(&plugin)
        };
        let access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(0, 0, None, None, Some(authority.clone()), Vec::new()),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST | CONFIG_PAUSED,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            vec![
                ExtraCell::Output {
                    lock: lock.script.clone(),
                    type_script: access_list.script.clone(),
                    data: full_domain_shard(),
                    cell_dep: access_list,
                },
                ExtraCell::Dep { cell_dep: plugin },
            ],
        )
    })
}

#[test]
fn xudt_meta_access_update_with_dynamic_linking_authority_passes() {
    let case = xudt_meta_access_update_with_plugin_authority("authority-dl-allow", false);

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_access_update_with_dynamic_linking_authority_denies() {
    let case = xudt_meta_access_update_with_plugin_authority("authority-dl-deny", false);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 51");
}

#[test]
fn xudt_meta_access_update_with_spawn_authority_passes() {
    let case = xudt_meta_access_update_with_plugin_authority("authority-spawn-allow", true);

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_access_update_with_spawn_authority_denies() {
    let case = xudt_meta_access_update_with_plugin_authority("authority-spawn-deny", true);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 51");
}

#[test]
fn xudt_meta_disabled_to_blacklist_rejects_overlapping_access_list_outputs() {
    let case = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let access_list = access_list_script(context, meta.script_hash);
        let overlapping_access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(0, 0, None, None, Some(authority.clone()), Vec::new()),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            vec![
                ExtraCell::Output {
                    lock: lock.script.clone(),
                    type_script: access_list.script.clone(),
                    data: full_domain_shard(),
                    cell_dep: access_list,
                },
                ExtraCell::Output {
                    lock: lock.script.clone(),
                    type_script: overlapping_access_list.script.clone(),
                    data: build_access_list_shard_bytes([0u8; 32], [0xffu8; 32], Vec::new()),
                    cell_dep: overlapping_access_list,
                },
            ],
        )
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 61");
}

#[test]
fn xudt_meta_disabled_to_whitelist_rejects_access_list_start_after_end() {
    let case = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let access_list = access_list_script(context, meta.script_hash);
        let mut start = [0u8; 32];
        start[31] = 0x10;
        (
            xudt_meta_data(0, 0, None, None, Some(authority.clone()), Vec::new()),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            vec![ExtraCell::Output {
                lock: lock.script.clone(),
                type_script: access_list.script.clone(),
                data: build_access_list_shard_bytes(start, [0u8; 32], Vec::new()),
                cell_dep: access_list,
            }],
        )
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 60");
}

#[test]
fn xudt_meta_disabled_to_whitelist_rejects_access_list_extra_table_field() {
    let case = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(0, 0, None, None, Some(authority.clone()), Vec::new()),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            vec![ExtraCell::Output {
                lock: lock.script.clone(),
                type_script: access_list.script.clone(),
                data: access_list_shard_with_extra_field(),
                cell_dep: access_list,
            }],
        )
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 60");
}

#[test]
fn xudt_meta_disabled_to_whitelist_rejects_duplicate_access_list_entries() {
    let case = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let access_list = access_list_script(context, meta.script_hash);
        let entry = [1u8; 32];
        (
            xudt_meta_data(0, 0, None, None, Some(authority.clone()), Vec::new()),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            vec![ExtraCell::Output {
                lock: lock.script.clone(),
                type_script: access_list.script.clone(),
                data: build_access_list_shard_bytes([0u8; 32], [0xffu8; 32], vec![entry, entry]),
                cell_dep: access_list,
            }],
        )
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 60");
}

#[test]
fn xudt_meta_blacklist_to_whitelist_requires_legal_output_shard() {
    let missing_shard = update_meta_tx(|_, lock, _| {
        let authority = input_lock_authority(lock.script_hash);
        (
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED,
                0,
                None,
                None,
                Some(authority.clone()),
                Vec::new(),
            ),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            Vec::new(),
        )
    });
    expect_tx_fail_with_code(&missing_shard.context, &missing_shard.tx, "error code 60");

    let with_shard = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let input_access_list = access_list_script(context, meta.script_hash);
        let output_access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED,
                0,
                None,
                None,
                Some(authority.clone()),
                Vec::new(),
            ),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            vec![
                ExtraCell::Input {
                    previous_output: create_typed_cell(
                        context,
                        &lock.script,
                        &input_access_list.script,
                        100_000_000_000,
                        full_domain_shard(),
                    ),
                    cell_dep: input_access_list,
                },
                ExtraCell::Output {
                    lock: lock.script.clone(),
                    type_script: output_access_list.script.clone(),
                    data: full_domain_shard(),
                    cell_dep: output_access_list,
                },
            ],
        )
    });
    expect_tx_pass(&with_shard.context, &with_shard.tx);
}

#[test]
fn xudt_meta_blacklist_to_whitelist_rejects_malformed_access_list_output() {
    let case = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let input_access_list = access_list_script(context, meta.script_hash);
        let output_access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED,
                0,
                None,
                None,
                Some(authority.clone()),
                Vec::new(),
            ),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            vec![
                ExtraCell::Input {
                    previous_output: create_typed_cell(
                        context,
                        &lock.script,
                        &input_access_list.script,
                        100_000_000_000,
                        full_domain_shard(),
                    ),
                    cell_dep: input_access_list,
                },
                ExtraCell::Output {
                    lock: lock.script.clone(),
                    type_script: output_access_list.script.clone(),
                    data: access_list_shard_with_extra_field(),
                    cell_dep: output_access_list,
                },
            ],
        )
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 60");
}

#[test]
fn xudt_meta_blacklist_to_disabled_requires_full_domain_access_list_inputs() {
    let missing_input = update_meta_tx(|_, lock, _| {
        let authority = input_lock_authority(lock.script_hash);
        (
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED,
                0,
                None,
                None,
                Some(authority.clone()),
                Vec::new(),
            ),
            xudt_meta_data(0, 0, None, None, Some(authority), Vec::new()),
            Vec::new(),
        )
    });
    expect_tx_fail_with_code(&missing_input.context, &missing_input.tx, "error code 60");

    let partial_input = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED,
                0,
                None,
                None,
                Some(authority.clone()),
                Vec::new(),
            ),
            xudt_meta_data(0, 0, None, None, Some(authority), Vec::new()),
            vec![ExtraCell::Input {
                previous_output: create_typed_cell(
                    context,
                    &lock.script,
                    &access_list.script,
                    100_000_000_000,
                    build_access_list_shard_bytes([0u8; 32], [0x7fu8; 32], Vec::new()),
                ),
                cell_dep: access_list,
            }],
        )
    });
    expect_tx_fail_with_code(&partial_input.context, &partial_input.tx, "error code 60");

    let full_input = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED,
                0,
                None,
                None,
                Some(authority.clone()),
                Vec::new(),
            ),
            xudt_meta_data(0, 0, None, None, Some(authority), Vec::new()),
            vec![ExtraCell::Input {
                previous_output: create_typed_cell(
                    context,
                    &lock.script,
                    &access_list.script,
                    100_000_000_000,
                    full_domain_shard(),
                ),
                cell_dep: access_list,
            }],
        )
    });
    expect_tx_pass(&full_input.context, &full_input.tx);
}
