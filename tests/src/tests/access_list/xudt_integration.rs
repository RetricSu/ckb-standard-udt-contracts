use super::*;

#[test]
fn access_list_whitelist_missing_coverage_is_fail_closed_for_xudt() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![shard(0x00, 0x0f, Vec::new())],
        Vec::new(),
    );

    expect_tx_fail(&case.context, &case.tx);
}
