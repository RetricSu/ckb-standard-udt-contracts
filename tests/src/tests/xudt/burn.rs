use super::*;

#[test]
fn xudt_protocol_burn_requires_mint_authority_and_updates_supply() {
    let mut unauthorized = XudtFixture::new();
    let meta_input = unauthorized.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, false);
    let udt_input = unauthorized.live_udt_input(100);
    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(udt_input)
        .output(typed_output(
            &unauthorized.lock.script,
            &unauthorized.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &unauthorized.lock.script,
            &unauthorized.xudt.script,
            100_000_000_000,
        ))
        .output_data(xudt_meta_data(CONFIG_SUPPLY_TRACKED, 40, None, Vec::new()).pack())
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = unauthorized.complete(tx);
    expect_tx_fail(&unauthorized.context, &tx);

    let mut authorized = XudtFixture::new();
    let meta_input = authorized.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, true);
    let udt_input = authorized.live_udt_input(100);
    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(udt_input)
        .output(typed_output(
            &authorized.lock.script,
            &authorized.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &authorized.lock.script,
            &authorized.xudt.script,
            100_000_000_000,
        ))
        .output_data(
            xudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                40,
                Some(input_lock_authority(authorized.lock.script_hash)),
                Vec::new(),
            )
            .pack(),
        )
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = authorized.complete(tx);
    expect_tx_pass(&authorized.context, &tx);
}

#[test]
fn xudt_protocol_burn_with_meta_dep_still_uses_input_meta() {
    let mut fixture = XudtFixture::new();
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, true);
    let duplicate_meta_dep = fixture.live_meta_dep(CONFIG_SUPPLY_TRACKED, 100, true);
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(udt_input)
        .cell_dep(cell_dep(duplicate_meta_dep.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(
            xudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                40,
                Some(input_lock_authority(fixture.lock.script_hash)),
                Vec::new(),
            )
            .pack(),
        )
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_user_destruction_skips_access_and_extensions() {
    let mut fixture = XudtFixture::new();
    let udt_input = fixture.live_udt_input(100);
    let listed_lock = fixture.lock.script_hash;
    let access_list = fixture.live_access_list_input(full_domain_shard(vec![listed_lock]));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(access_list.previous_output()))
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_protocol_burn_with_meta_dep_requires_mint_authority() {
    let mut fixture = XudtFixture::new_with_always_success_meta();
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, false);
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(meta_input.clone())
        .input(udt_input)
        .cell_dep(cell_dep(meta_input.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output_data(xudt_meta_data(CONFIG_SUPPLY_TRACKED, 100, None, Vec::new()).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 50");
}

#[test]
fn xudt_negative_delta_with_input_meta_requires_protocol_burn_authority() {
    let mut fixture = XudtFixture::new_with_always_success_meta();
    let meta_input = fixture.live_meta_input(CONFIG_SUPPLY_TRACKED, 100, false);
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(meta_input.clone())
        .input(udt_input)
        .cell_dep(cell_dep(meta_input.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output_data(xudt_meta_data(CONFIG_SUPPLY_TRACKED, 100, None, Vec::new()).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 50");
}

#[test]
fn xudt_protocol_burn_access_mode_switch_still_requires_mint_authority() {
    let mut fixture = XudtFixture::new_with_always_success_meta();
    let meta_input = fixture.live_meta_input_with_authority(CONFIG_ACCESS_ENABLED, 0, None);
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(udt_input)
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(
            fixture
                .output_meta_data(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, None)
                .pack(),
        )
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 50");
}

#[test]
fn xudt_protocol_burn_access_mode_switch_does_not_skip_access_checks() {
    let mut fixture = XudtFixture::new_with_always_success_meta();
    let authority = input_lock_authority(fixture.lock.script_hash);
    let meta_input = fixture.live_meta_input_with_authority(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        Some(authority.clone()),
    );
    let udt_input = fixture.live_udt_input(100);
    let access_list =
        fixture.live_access_list_input(full_domain_shard(vec![fixture.other_lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(udt_input)
        .cell_dep(cell_dep(access_list.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(
            fixture
                .output_meta_data(CONFIG_ACCESS_ENABLED, 0, Some(authority))
                .pack(),
        )
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 61");
}
