use super::*;

#[test]
fn xudt_transfer_requires_meta() {
    let mut fixture = XudtFixture::new();
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default()
        .input(udt_input)
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
fn xudt_paused_rejects_transfer_and_mint() {
    let mut transfer = XudtFixture::new();
    let meta_dep = transfer.live_meta_dep(CONFIG_PAUSED, 0, true);
    let udt_input = transfer.live_udt_input(100);
    let tx = TransactionBuilder::default()
        .input(udt_input)
        .cell_dep(cell_dep(meta_dep.previous_output()))
        .output(typed_output(
            &transfer.lock.script,
            &transfer.xudt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(100).pack())
        .build();
    let tx = transfer.complete(tx);
    expect_tx_fail(&transfer.context, &tx);

    let mut mint = XudtFixture::new();
    let meta_input = mint.live_meta_input(CONFIG_SUPPLY_TRACKED | CONFIG_PAUSED, 0, true);
    let funding = create_funding_input(&mut mint.context, &mint.lock.script, 100_000_000_000);
    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(funding)
        .output(typed_output(
            &mint.lock.script,
            &mint.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &mint.lock.script,
            &mint.xudt.script,
            100_000_000_000,
        ))
        .output_data(
            xudt_meta_data(
                CONFIG_SUPPLY_TRACKED | CONFIG_PAUSED,
                50,
                Some(input_lock_authority(mint.lock.script_hash)),
                Vec::new(),
            )
            .pack(),
        )
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = mint.complete(tx);
    expect_tx_fail(&mint.context, &tx);
}

#[test]
fn xudt_paused_allows_user_destruction() {
    let mut fixture = XudtFixture::new();
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default().input(udt_input).build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}
