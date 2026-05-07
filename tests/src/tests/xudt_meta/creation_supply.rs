use super::*;

#[test]
fn xudt_meta_create_rejects_short_same_token_udt_data() {
    let case = create_meta_tx_with_udt_output_data(0, vec![Bytes::from(vec![0u8; 15])]);

    expect_tx_fail(&case.context, &case.tx);
}

#[test]
fn xudt_meta_create_rejects_same_token_udt_sum_overflow() {
    let case = create_meta_tx_with_udt_output_data(
        0,
        vec![udt_amount_bytes(u128::MAX), udt_amount_bytes(1)],
    );

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 31");
}

#[test]
fn xudt_meta_create_blacklist_requires_full_domain_access_list_outputs() {
    let case = create_meta_tx_with_access_outputs(CONFIG_ACCESS_ENABLED, false);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 60");
}

#[test]
fn xudt_meta_create_whitelist_requires_full_domain_access_list_outputs() {
    let case =
        create_meta_tx_with_access_outputs(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, false);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 60");
}

#[test]
fn xudt_meta_create_blacklist_accepts_full_domain_access_list_outputs() {
    let case = create_meta_tx_with_access_outputs(CONFIG_ACCESS_ENABLED, true);

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_create_whitelist_accepts_full_domain_access_list_outputs() {
    let case =
        create_meta_tx_with_access_outputs(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, true);

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_rejects_supply_increase_without_udt_delta() {
    let case = update_meta_tx_with_udt_delta(100, 101, None, None);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 31");
}

#[test]
fn xudt_meta_rejects_supply_decrease_without_udt_delta() {
    let case = update_meta_tx_with_udt_delta(100, 99, None, None);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 31");
}

#[test]
fn xudt_meta_accepts_supply_increase_matching_udt_delta() {
    let case = update_meta_tx_with_udt_delta(100, 125, None, Some(25));

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_accepts_supply_decrease_matching_udt_delta() {
    let case = update_meta_tx_with_udt_delta(100, 75, Some(25), None);

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_rejects_supply_delta_mismatch() {
    let case = update_meta_tx_with_udt_delta(100, 125, Some(24), Some(24));

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 31");
}

#[test]
fn xudt_meta_ignores_fake_data2_udt_outputs() {
    let case = update_meta_tx_with_fake_udt_output(100, 125, 25);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 31");
}

#[test]
fn xudt_meta_mint_authority_can_update_metadata() {
    let case = update_meta_tx(|_, lock, _| {
        let authority = input_lock_authority(lock.script_hash);
        (
            xudt_meta_data(0, 0, Some(authority.clone()), None, None, Vec::new()),
            with_name(
                xudt_meta_data(0, 0, Some(authority), None, None, Vec::new()),
                b"new name",
            ),
            Vec::new(),
        )
    });

    expect_tx_pass(&case.context, &case.tx);
}
