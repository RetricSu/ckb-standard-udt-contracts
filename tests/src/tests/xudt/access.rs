use super::*;

#[test]
fn xudt_blacklist_rejects_listed_input_lock() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let access_list =
        fixture.live_access_list_input(full_domain_shard(vec![fixture.lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(access_list.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn xudt_blacklist_rejects_missing_non_membership_proof() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let non_covering =
        fixture.live_access_list_input(custom_shard([0u8; 32], [0u8; 32], Vec::new()));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(non_covering.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 60");
}

#[test]
fn xudt_blacklist_accepts_covering_non_membership_proof() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let proof = fixture.live_access_list_input(exact_shard(fixture.lock.script_hash, Vec::new()));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_whitelist_rejects_missing_input_lock() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let access_list =
        fixture.live_access_list_input(full_domain_shard(vec![fixture.other_lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(access_list.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn xudt_whitelist_accepts_covering_membership_proof() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let proof = fixture.live_access_list_input(exact_shard(
        fixture.lock.script_hash,
        vec![fixture.lock.script_hash],
    ));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_whitelist_ignores_non_data2_access_list_shards() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let fake_access_list = fixture.live_access_list_input_with_hash_type(
        ScriptHashType::Data,
        full_domain_shard(vec![fixture.lock.script_hash]),
    );

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(fake_access_list.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 61");
}
