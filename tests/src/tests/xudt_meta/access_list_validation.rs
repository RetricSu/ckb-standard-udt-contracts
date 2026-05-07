use super::*;

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
