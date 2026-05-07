use super::*;

#[test]
fn access_list_rejects_unauthorized_update() {
    let mut listed = [0u8; 32];
    listed[31] = 0x10;
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        false,
        vec![full_domain_shard(Vec::new())],
        vec![full_domain_shard(vec![listed])],
    );

    expect_tx_fail(&case.context, &case.tx);
}

#[test]
fn access_list_update_accepts_mint_authority() {
    let case = access_list_update_tx_with_mint_authority(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(Vec::new())],
        vec![full_domain_shard(vec![entry(0x10)])],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_update_allows_visible_meta_with_non_whitelisted_lock() {
    let case = access_list_update_tx_with_non_whitelisted_meta_lock(
        CONFIG_ACCESS_ENABLED,
        vec![full_domain_shard(vec![entry(0x10)])],
        vec![full_domain_shard(vec![entry(0x10)])],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_rejects_non_whitelisted_output_lock() {
    let case =
        access_list_update_tx_with_non_whitelisted_output_lock(vec![full_domain_shard(Vec::new())]);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 20");
}

#[test]
fn access_list_update_with_dynamic_linking_authority_passes() {
    let case = access_list_update_tx_with_plugin_authority(
        "authority-dl-allow",
        false,
        vec![full_domain_shard(vec![entry(0x10)])],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_update_with_dynamic_linking_authority_denies() {
    let case = access_list_update_tx_with_plugin_authority(
        "authority-dl-deny",
        false,
        vec![full_domain_shard(vec![entry(0x10)])],
    );

    expect_tx_fail(&case.context, &case.tx);
}

#[test]
fn access_list_update_with_spawn_authority_passes() {
    let case = access_list_update_tx_with_plugin_authority(
        "authority-spawn-allow",
        true,
        vec![full_domain_shard(vec![entry(0x10)])],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_update_with_spawn_authority_denies() {
    let case = access_list_update_tx_with_plugin_authority(
        "authority-spawn-deny",
        true,
        vec![full_domain_shard(vec![entry(0x10)])],
    );

    expect_tx_fail(&case.context, &case.tx);
}
