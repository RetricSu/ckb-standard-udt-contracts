use super::*;

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

#[test]
fn xudt_meta_mint_authority_can_update_access_state() {
    let case = update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(0, 0, Some(authority.clone()), None, None, Vec::new()),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST | CONFIG_PAUSED,
                0,
                Some(authority),
                None,
                None,
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

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_destroy_accepts_tracked_zero_supply_when_access_disabled() {
    let case = destroy_meta_tx(CONFIG_SUPPLY_TRACKED, 0, false);

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_destroy_rejects_metadata_authority_without_mint_authority() {
    let case = destroy_meta_tx_with_authorities(CONFIG_SUPPLY_TRACKED, 0, false, |lock_hash| {
        (None, Some(input_lock_authority(lock_hash)), None)
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 50");
}

#[test]
fn xudt_meta_destroy_rejects_access_authority_without_mint_authority() {
    let case = destroy_meta_tx_with_authorities(CONFIG_SUPPLY_TRACKED, 0, false, |lock_hash| {
        (None, None, Some(input_lock_authority(lock_hash)))
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 50");
}

#[test]
fn xudt_meta_destroy_rejects_tracked_nonzero_supply() {
    let case = destroy_meta_tx(CONFIG_SUPPLY_TRACKED, 1, false);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 31");
}

#[test]
fn xudt_meta_destroy_rejects_active_access_without_full_domain_inputs() {
    let case = destroy_meta_tx(CONFIG_SUPPLY_TRACKED | CONFIG_ACCESS_ENABLED, 0, false);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 60");
}

#[test]
fn xudt_meta_destroy_accepts_active_access_with_full_domain_inputs() {
    let case = destroy_meta_tx(CONFIG_SUPPLY_TRACKED | CONFIG_ACCESS_ENABLED, 0, true);

    expect_tx_pass(&case.context, &case.tx);
}
