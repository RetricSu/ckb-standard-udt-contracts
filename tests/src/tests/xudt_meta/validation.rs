use super::*;

#[test]
fn xudt_meta_rejects_invalid_config_flags() {
    let case = update_meta_tx(|_, _, _| {
        let input = xudt_meta_data(0, 0, None, None, None, Vec::new());
        let output = with_config_flags(input.clone(), CONFIG_ACCESS_WHITELIST);
        (input, output, Vec::new())
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 30");
}

#[test]
fn xudt_meta_rejects_malformed_name_bytes_field() {
    let case = update_meta_tx(|_, _, _| {
        let data = malformed_name_meta_data();
        (data.clone(), data, Vec::new())
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 30");
}

#[test]
fn xudt_meta_rejects_oversized_name_field() {
    let case = update_meta_tx(|_, _, _| {
        let data = oversized_name_meta_data();
        (data.clone(), data, Vec::new())
    });

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 30");
}

#[test]
fn xudt_meta_rejects_non_whitelisted_output_lock() {
    let case = update_meta_tx_with_output_lock(non_whitelisted_lock);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 20");
}

#[test]
fn xudt_meta_update_rejects_duplicate_output_meta_cells() {
    let case = update_meta_tx_with_duplicate_outputs();

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 21");
}
