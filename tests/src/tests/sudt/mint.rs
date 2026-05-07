use super::*;

#[test]
fn sudt_mint_with_dynamic_linking_authority_passes() {
    assert!(sudt_mint_with_plugin_authority("authority-dl-allow", false));
}

#[test]
fn sudt_mint_with_dynamic_linking_authority_denies() {
    assert!(!sudt_mint_with_plugin_authority("authority-dl-deny", false));
}

#[test]
fn sudt_mint_with_spawn_authority_passes() {
    assert!(sudt_mint_with_plugin_authority(
        "authority-spawn-allow",
        true
    ));
}

#[test]
fn sudt_mint_with_spawn_authority_denies() {
    assert!(!sudt_mint_with_plugin_authority(
        "authority-spawn-deny",
        true
    ));
}

#[test]
fn sudt_mint_requires_mint_authority() {
    let mut fixture = SudtFixture::new();
    let meta_dep = fixture.live_meta_dep(0, false);
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);

    let tx = TransactionBuilder::default()
        .input(funding)
        .cell_dep(
            ckb_testtool::ckb_types::packed::CellDep::new_builder()
                .out_point(meta_dep.previous_output())
                .build(),
        )
        .output(typed_output(
            &fixture.lock.script,
            &fixture.udt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn sudt_mint_allows_visible_meta_with_non_whitelisted_lock() {
    let mut fixture = SudtFixture::new();
    let meta_lock = non_whitelisted_lock(&mut fixture.context);
    let meta_dep = create_typed_cell(
        &mut fixture.context,
        &meta_lock.script,
        &fixture.meta.script,
        100_000_000_000,
        untracked_meta_data(Some(input_lock_authority(fixture.lock.script_hash))),
    );
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);

    let tx = TransactionBuilder::default()
        .input(funding)
        .cell_dep(cell_dep(meta_dep))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.udt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn sudt_tracked_mint_updates_supply() {
    let mut fixture = SudtFixture::new();
    let meta_input = fixture.live_meta_input(0, true);
    let funding = create_funding_input(&mut fixture.context, &fixture.lock.script, 100_000_000_000);

    let tx = TransactionBuilder::default()
        .input(meta_input)
        .input(funding)
        .output(typed_output(
            &fixture.lock.script,
            &fixture.meta.script,
            100_000_000_000,
        ))
        .output(typed_output(
            &fixture.lock.script,
            &fixture.udt.script,
            100_000_000_000,
        ))
        .output_data(tracked_meta_data(50, Some(fixture.lock.script_hash)).pack())
        .output_data(udt_amount_bytes(50).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}
