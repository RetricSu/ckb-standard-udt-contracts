use super::*;

#[test]
fn access_list_disabled_to_disabled_rejects_access_list_inputs_or_outputs() {
    let with_input =
        access_list_transition_tx(0, 0, true, vec![full_domain_shard(Vec::new())], Vec::new());
    expect_tx_fail_with_code(&with_input.context, &with_input.tx, "error code 61");

    let with_output =
        access_list_transition_tx(0, 0, true, Vec::new(), vec![full_domain_shard(Vec::new())]);
    expect_tx_fail_with_code(&with_output.context, &with_output.tx, "error code 61");
}

#[test]
fn access_list_whitelist_create_requires_full_domain_outputs() {
    let partial = access_list_transition_tx(
        0,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        Vec::new(),
        vec![custom_shard([0u8; 32], prefix_end(0x7f), Vec::new())],
    );
    expect_tx_fail_with_code(&partial.context, &partial.tx, "error code 61");

    let full = access_list_transition_tx(
        0,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        Vec::new(),
        vec![full_domain_shard(Vec::new())],
    );
    expect_tx_pass(&full.context, &full.tx);
}

#[test]
fn access_list_blacklist_create_requires_full_domain_outputs() {
    let partial = access_list_transition_tx(
        0,
        CONFIG_ACCESS_ENABLED,
        true,
        Vec::new(),
        vec![custom_shard([0u8; 32], prefix_end(0x7f), Vec::new())],
    );
    expect_tx_fail_with_code(&partial.context, &partial.tx, "error code 61");

    let full = access_list_transition_tx(
        0,
        CONFIG_ACCESS_ENABLED,
        true,
        Vec::new(),
        vec![full_domain_shard(Vec::new())],
    );
    expect_tx_pass(&full.context, &full.tx);
}

#[test]
fn access_list_whitelist_rejects_repeated_create_from_empty_inputs() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        Vec::new(),
        vec![full_domain_shard(Vec::new())],
    );

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 61");
}

#[test]
fn access_list_blacklist_rejects_repeated_create_from_empty_inputs() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED,
        true,
        Vec::new(),
        vec![full_domain_shard(Vec::new())],
    );

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 61");
}

#[test]
fn access_list_active_destroy_requires_full_domain_inputs_and_empty_outputs() {
    let partial = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        true,
        vec![custom_shard([0u8; 32], prefix_end(0x7f), Vec::new())],
        Vec::new(),
    );
    expect_tx_fail_with_code(&partial.context, &partial.tx, "error code 61");

    let with_output = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        true,
        vec![full_domain_shard(Vec::new())],
        vec![full_domain_shard(Vec::new())],
    );
    expect_tx_fail_with_code(&with_output.context, &with_output.tx, "error code 61");

    let full_destroy = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        0,
        true,
        vec![full_domain_shard(Vec::new())],
        Vec::new(),
    );
    expect_tx_pass(&full_destroy.context, &full_destroy.tx);

    let blacklist_destroy = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        0,
        true,
        vec![full_domain_shard(Vec::new())],
        Vec::new(),
    );
    expect_tx_pass(&blacklist_destroy.context, &blacklist_destroy.tx);
}

#[test]
fn access_list_mode_replace_requires_full_domain_inputs_and_outputs_but_allows_entry_reset() {
    let missing_input = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        Vec::new(),
        vec![full_domain_shard(vec![entry(0x20)])],
    );
    expect_tx_fail_with_code(&missing_input.context, &missing_input.tx, "error code 61");

    let full_replace = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![full_domain_shard(vec![entry(0x10)])],
        vec![full_domain_shard(vec![entry(0x20)])],
    );
    expect_tx_pass(&full_replace.context, &full_replace.tx);

    let reverse_replace = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(vec![entry(0x10)])],
        vec![full_domain_shard(vec![entry(0x20)])],
    );
    expect_tx_pass(&reverse_replace.context, &reverse_replace.tx);
}
