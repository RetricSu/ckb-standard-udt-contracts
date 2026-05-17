use super::*;

#[test]
fn sudt_user_destruction_without_meta_passes() {
    let mut fixture = SudtFixture::new();
    let udt_input = fixture.live_udt_input(100);

    let tx = TransactionBuilder::default().input(udt_input).build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn sudt_protocol_burn_requires_mint_authority() {
    let mut fixture = SudtFixture::new();
    let meta_input = fixture.live_meta_input(100, false);
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
            &fixture.udt.script,
            100_000_000_000,
        ))
        .output_data(tracked_meta_data(40, None).pack())
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_fail(&fixture.context, &tx);
}

#[test]
fn sudt_protocol_burn_updates_tracked_supply() {
    let mut fixture = SudtFixture::new();
    let meta_input = fixture.live_meta_input(100, true);
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
            &fixture.udt.script,
            100_000_000_000,
        ))
        .output_data(tracked_meta_data(40, Some(fixture.lock.script_hash)).pack())
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}

#[test]
fn sudt_untracked_burn_with_meta_input_does_not_require_mint_authority() {
    let mut fixture = SudtFixture::new();
    let metadata_authority = input_lock_authority(fixture.lock.script_hash);
    let meta_data = build_sudt_meta_bytes(0, 0, None, Some(metadata_authority));
    let meta_out_point = create_typed_cell(
        &mut fixture.context,
        &fixture.lock.script,
        &fixture.meta.script,
        100_000_000_000,
        meta_data.clone(),
    );
    let meta_input = CellInput::new_builder()
        .previous_output(meta_out_point)
        .build();
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
            &fixture.udt.script,
            100_000_000_000,
        ))
        .output_data(meta_data.pack())
        .output_data(udt_amount_bytes(40).pack())
        .build();
    let tx = fixture.complete(tx);

    expect_tx_pass(&fixture.context, &tx);
}
