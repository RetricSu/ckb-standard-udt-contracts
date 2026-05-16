use super::*;

#[test]
fn sudt_meta_update_supply_change_with_input_lock_mint_authority_passes() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 101, None, Some(1));

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_rejects_supply_increase_without_udt_delta() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 101, None, None);

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}

#[test]
fn sudt_meta_rejects_supply_decrease_without_udt_delta() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 99, None, None);

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}

#[test]
fn sudt_meta_accepts_supply_increase_matching_udt_delta() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 125, None, Some(25));

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_accepts_supply_decrease_matching_udt_delta() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 75, Some(25), None);

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_rejects_supply_delta_mismatch() {
    let (context, tx) = update_meta_tx_with_udt_delta(100, 125, Some(24), Some(24));

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}

#[test]
fn sudt_meta_destroy_accepts_tracked_zero_supply() {
    let (context, tx) = destroy_meta_tx_with_data(|lock_hash| {
        sudt_meta_data(
            CONFIG_SUPPLY_TRACKED,
            0,
            Some(input_lock_authority(lock_hash)),
            None,
            Vec::new(),
            Vec::new(),
        )
    });

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_destroy_rejects_metadata_authority_without_mint_authority() {
    let (context, tx) = destroy_meta_tx_with_data(|lock_hash| {
        sudt_meta_data(
            CONFIG_SUPPLY_TRACKED,
            0,
            None,
            Some(input_lock_authority(lock_hash)),
            Vec::new(),
            Vec::new(),
        )
    });

    expect_tx_fail_with_code(&context, &tx, "error code 50");
}

#[test]
fn sudt_meta_destroy_rejects_tracked_nonzero_supply() {
    let (context, tx) = destroy_meta_tx_with_data(|lock_hash| {
        sudt_meta_data(
            CONFIG_SUPPLY_TRACKED,
            1,
            Some(input_lock_authority(lock_hash)),
            None,
            Vec::new(),
            Vec::new(),
        )
    });

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}

#[test]
fn sudt_meta_destroy_rejects_untracked_zero_supply() {
    let (context, tx) = destroy_meta_tx_with_data(|lock_hash| {
        sudt_meta_data(
            0,
            0,
            Some(input_lock_authority(lock_hash)),
            None,
            Vec::new(),
            Vec::new(),
        )
    });

    expect_tx_fail_with_code(&context, &tx, "error code 31");
}

#[test]
fn sudt_meta_update_rejects_mint_authority_recreation() {
    let (context, tx) = update_meta_tx_with_data(|lock_hash, _| {
        (
            tracked_meta_data(0),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                Some(input_lock_authority(lock_hash)),
                None,
                Vec::new(),
                Vec::new(),
            ),
        )
    });

    expect_tx_fail_with_code(&context, &tx, "error code 50");
}
