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
fn xudt_blacklist_rejects_blacklisted_output_lock() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let proof =
        fixture.live_access_list_input(full_domain_shard(vec![fixture.other_lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()))
        .output(typed_output(
            &fixture.other_lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 61");
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
fn xudt_whitelist_rejects_non_whitelisted_output_lock() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let proof = fixture.live_access_list_input(full_domain_shard(vec![fixture.lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()))
        .output(typed_output(
            &fixture.other_lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 61");
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
fn xudt_blacklist_mint_rejects_blacklisted_output_lock() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED, 0, true);
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);
    let proof =
        fixture.live_access_list_input(full_domain_shard(vec![fixture.other_lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(funding)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()))
        .output(typed_output(
            &fixture.other_lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 61");
}

#[test]
fn xudt_whitelist_mint_rejects_non_whitelisted_output_lock() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, true);
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);
    let proof = fixture.live_access_list_input(full_domain_shard(vec![fixture.lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(funding)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()))
        .output(typed_output(
            &fixture.other_lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 61");
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

#[test]
fn xudt_whitelist_rejects_input_access_list_as_transfer_proof() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, true);
    let udt_input = fixture.live_udt_input(100);
    let access_input =
        fixture.live_access_list_input(full_domain_shard(vec![fixture.lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .input(access_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.access_list.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .output_data(full_domain_shard(vec![fixture.lock.script_hash]).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn xudt_rejects_unordered_access_list_cell_dep_proofs() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let input_proof = fixture.live_access_list_input(exact_shard(
        fixture.lock.script_hash,
        vec![fixture.lock.script_hash],
    ));
    let output_proof = fixture.live_access_list_input(exact_shard(
        fixture.other_lock.script_hash,
        vec![fixture.other_lock.script_hash],
    ));

    let (first, second) = {
        let input_start = fixture.lock.script_hash[0] & 0xf0;
        let output_start = fixture.other_lock.script_hash[0] & 0xf0;
        if input_start <= output_start {
            (
                output_proof.previous_output(),
                input_proof.previous_output(),
            )
        } else {
            (
                input_proof.previous_output(),
                output_proof.previous_output(),
            )
        }
    };

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(first))
        .cell_dep(cell_dep(second))
        .output(typed_output(
            &fixture.other_lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn xudt_rejects_overlapping_access_list_cell_dep_proofs() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let first = fixture.live_access_list_input(full_domain_shard(vec![fixture.lock.script_hash]));
    let second = fixture.live_access_list_input(full_domain_shard(Vec::new()));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(first.previous_output()))
        .cell_dep(cell_dep(second.previous_output()))
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
fn xudt_transfer_allows_same_meta_access_list_update_with_cell_dep_proof() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED, 0, true);
    let udt_input = fixture.live_udt_input(100);
    let access_input = fixture.live_access_list_input(full_domain_shard(Vec::new()));
    let proof = fixture.live_access_list_input(full_domain_shard(Vec::new()));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .input(access_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.access_list.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .output_data(full_domain_shard(Vec::new()).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_pure_user_destruction_allows_same_meta_access_list_update() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, true);
    let udt_input = fixture.live_udt_input(100);
    let access_input =
        fixture.live_access_list_input(full_domain_shard(vec![fixture.lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .input(access_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.access_list.script,
            100_000_000_000,
        ))
        .output_data(full_domain_shard(vec![fixture.lock.script_hash]).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_partial_user_destruction_checks_blacklisted_input_lock() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let proof = fixture.live_access_list_input(full_domain_shard(vec![fixture.lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()))
        .output(typed_output(
            &fixture.other_lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(99).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 61");
}

#[test]
fn xudt_partial_user_destruction_checks_whitelisted_output_lock() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, false);
    let udt_input = fixture.live_udt_input(100);
    let proof = fixture.live_access_list_input(full_domain_shard(vec![fixture.lock.script_hash]));

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()))
        .output(typed_output(
            &fixture.other_lock.script,
            &fixture.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(99).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail_with_code(&fixture.context, &tx, "error code 61");
}

#[test]
fn xudt_pure_user_destruction_skips_holder_access() {
    let mut fixture = XudtFixture::new();
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, false);
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_whitelist_accepts_many_output_locks_in_batches() {
    let mut fixture = XudtFixture::new();
    let output_count = 70u128;
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST, 0, false);
    let udt_input = fixture.live_udt_input(output_count);
    let mut output_locks = Vec::new();
    let mut entries = vec![fixture.lock.script_hash];
    for index in 0..output_count {
        let lock = always_success_lock(&mut fixture.context, Bytes::from(vec![index as u8 + 10]));
        entries.push(lock.script_hash);
        output_locks.push(lock);
    }
    entries.sort();
    let proof = fixture.live_access_list_input(full_domain_shard(entries));

    let mut builder = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()));
    for lock in &output_locks {
        builder = builder
            .output(typed_output(
                &lock.script,
                &fixture.xudt.script,
                100_000_000_000,
            ))
            .output_data(udt_amount_bytes(1).pack());
    }
    let tx = fixture.complete(builder.build());

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn xudt_blacklist_accepts_many_unlisted_output_locks_in_batches() {
    let mut fixture = XudtFixture::new();
    let output_count = 70u128;
    let meta_dep = fixture.live_meta_dep(CONFIG_ACCESS_ENABLED, 0, false);
    let udt_input = fixture.live_udt_input(output_count);
    let proof = fixture.live_access_list_input(full_domain_shard(Vec::new()));
    let mut output_locks = Vec::new();
    for index in 0..output_count {
        output_locks.push(always_success_lock(
            &mut fixture.context,
            Bytes::from(vec![index as u8 + 80]),
        ));
    }

    let mut builder = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .cell_dep(cell_dep(proof.previous_output()));
    for lock in &output_locks {
        builder = builder
            .output(typed_output(
                &lock.script,
                &fixture.xudt.script,
                100_000_000_000,
            ))
            .output_data(udt_amount_bytes(1).pack());
    }
    let tx = fixture.complete(builder.build());

    expect_tx_pass(&fixture.context, &tx);
}
