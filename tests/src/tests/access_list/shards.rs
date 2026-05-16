use super::*;

#[test]
fn access_list_whitelist_allows_same_range_insert_delete() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![custom_shard([0u8; 32], prefix_end(0x0f), vec![entry(0x10)])],
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x0f),
            vec![entry(0x10), entry(0x20)],
        )],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_whitelist_allows_split() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x2f),
            vec![prefix_entry(0x08), prefix_entry(0x20)],
        )],
        vec![
            custom_shard([0u8; 32], prefix_end(0x0f), vec![prefix_entry(0x08)]),
            custom_shard(
                prefix_start(0x10),
                prefix_end(0x2f),
                vec![prefix_entry(0x20)],
            ),
        ],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_whitelist_allows_split_that_changes_entries() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
        true,
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x2f),
            vec![prefix_entry(0x08)],
        )],
        vec![
            custom_shard([0u8; 32], prefix_end(0x0f), vec![prefix_entry(0x08)]),
            custom_shard(
                prefix_start(0x10),
                prefix_end(0x2f),
                vec![prefix_entry(0x20)],
            ),
        ],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_allows_local_same_range_insert_delete() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED,
        true,
        vec![custom_shard([0u8; 32], prefix_end(0x0f), vec![entry(0x10)])],
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x0f),
            vec![entry(0x10), entry(0x20)],
        )],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_allows_local_split() {
    let case = access_list_transition_tx(
        CONFIG_ACCESS_ENABLED,
        CONFIG_ACCESS_ENABLED,
        true,
        vec![custom_shard(
            [0u8; 32],
            prefix_end(0x2f),
            vec![prefix_entry(0x08), prefix_entry(0x20)],
        )],
        vec![
            custom_shard([0u8; 32], prefix_end(0x0f), vec![prefix_entry(0x08)]),
            custom_shard(
                prefix_start(0x10),
                prefix_end(0x2f),
                vec![prefix_entry(0x20)],
            ),
        ],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_requires_full_domain_coverage() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(Vec::new())],
        vec![shard(0x00, 0x7f, Vec::new())],
    );

    expect_tx_fail(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_rejects_suffix_only_nibble_alignment() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(Vec::new())],
        vec![
            bounded_shard(0x00, 0x0f, Vec::new()),
            tail_shard(0x10, Vec::new()),
        ],
    );

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 60");
}

#[test]
fn access_list_rejects_overlapping_shards() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(Vec::new())],
        vec![shard(0x00, 0x8f, Vec::new()), shard(0x80, 0xff, Vec::new())],
    );

    expect_tx_fail(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_allows_same_range_insert_delete() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(vec![entry(0x10)])],
        vec![full_domain_shard(vec![entry(0x10), entry(0x20)])],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_allows_split() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(vec![
            prefix_entry(0x08),
            prefix_entry(0x20),
        ])],
        vec![
            custom_shard([0u8; 32], prefix_end(0x0f), vec![prefix_entry(0x08)]),
            custom_shard(prefix_start(0x10), [0xffu8; 32], vec![prefix_entry(0x20)]),
        ],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_allows_split_that_changes_entries() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(vec![prefix_entry(0x08)])],
        vec![
            custom_shard([0u8; 32], prefix_end(0x0f), vec![prefix_entry(0x08)]),
            custom_shard(prefix_start(0x10), [0xffu8; 32], vec![prefix_entry(0x20)]),
        ],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_allows_boundary_rewrite_with_entry_changes() {
    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![
            custom_shard([0u8; 32], prefix_end(0x0f), vec![prefix_entry(0x08)]),
            custom_shard(prefix_start(0x10), [0xffu8; 32], vec![prefix_entry(0x20)]),
        ],
        vec![
            custom_shard(
                [0u8; 32],
                prefix_end(0x1f),
                vec![prefix_entry(0x08), prefix_entry(0x18)],
            ),
            custom_shard(prefix_start(0x20), [0xffu8; 32], vec![prefix_entry(0x20)]),
        ],
    );

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn access_list_blacklist_allows_large_split() {
    let entries = numbered_entries(4096);
    let mut first_half_end = [0xffu8; 32];
    first_half_end[0] = 0x7f;
    let mut second_half_start = [0u8; 32];
    second_half_start[0] = 0x80;

    let case = access_list_update_tx(
        CONFIG_ACCESS_ENABLED,
        true,
        vec![full_domain_shard(entries.clone())],
        vec![
            custom_shard([0u8; 32], first_half_end, entries),
            custom_shard(second_half_start, [0xffu8; 32], Vec::new()),
        ],
    );

    expect_tx_pass_with_cycles(&case.context, &case.tx, 100_000_000);
}
