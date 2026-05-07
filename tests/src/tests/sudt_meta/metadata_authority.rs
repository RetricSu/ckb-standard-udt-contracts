use super::*;

#[test]
fn sudt_meta_update_metadata_change_requires_metadata_authority() {
    let (context, tx) = update_meta_tx(
        tracked_meta_data(0),
        sudt_meta_data(
            CONFIG_SUPPLY_TRACKED,
            0,
            None,
            None,
            b"new name".to_vec(),
            Vec::new(),
        ),
    );

    expect_tx_fail_with_code(&context, &tx, "error code 50");
}

#[test]
fn sudt_meta_update_rejects_duplicate_output_meta_cells() {
    let (context, tx) = update_meta_tx_with_duplicate_outputs();

    expect_tx_fail_with_code(&context, &tx, "error code 21");
}

#[test]
fn sudt_meta_update_metadata_change_with_input_lock_authority_passes() {
    let (context, tx) = update_meta_tx_with_data(|lock_hash, _| {
        let authority = input_lock_authority(lock_hash);
        (
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(authority.clone()),
                Vec::new(),
                Vec::new(),
            ),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(authority),
                b"new name".to_vec(),
                Vec::new(),
            ),
        )
    });

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_mint_authority_can_update_metadata() {
    let (context, tx) = update_meta_tx_with_data(|lock_hash, _| {
        let authority = input_lock_authority(lock_hash);
        (
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                Some(authority.clone()),
                None,
                Vec::new(),
                Vec::new(),
            ),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                Some(authority),
                None,
                b"new name".to_vec(),
                Vec::new(),
            ),
        )
    });

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_rejects_non_whitelisted_output_lock() {
    let (context, tx) = update_meta_tx_with_locks(|context, lock_hash, _| {
        let output_lock = non_whitelisted_lock(context);
        let authority = input_lock_authority(lock_hash);
        (
            output_lock,
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(authority.clone()),
                Vec::new(),
                Vec::new(),
            ),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(authority),
                b"new name".to_vec(),
                Vec::new(),
            ),
        )
    });

    expect_tx_fail(&context, &tx);
}

#[test]
fn sudt_meta_update_rejects_metadata_authority_recreation() {
    let (context, tx) = update_meta_tx_with_data(|lock_hash, _| {
        (
            tracked_meta_data(0),
            sudt_meta_data(
                CONFIG_SUPPLY_TRACKED,
                0,
                None,
                Some(input_lock_authority(lock_hash)),
                Vec::new(),
                Vec::new(),
            ),
        )
    });

    expect_tx_fail_with_code(&context, &tx, "error code 50");
}
