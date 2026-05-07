use super::*;

#[test]
fn sudt_meta_update_metadata_change_with_dynamic_linking_authority_passes() {
    let (context, tx) = update_meta_tx_with_plugin_authority("authority-dl-allow", false);

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_update_metadata_change_with_dynamic_linking_authority_denies() {
    let (context, tx) = update_meta_tx_with_plugin_authority("authority-dl-deny", false);

    expect_tx_fail_with_code(&context, &tx, "error code 51");
}

#[test]
fn sudt_meta_update_metadata_change_with_spawn_authority_passes() {
    let (context, tx) = update_meta_tx_with_plugin_authority("authority-spawn-allow", true);

    expect_tx_pass(&context, &tx);
}

#[test]
fn sudt_meta_update_metadata_change_with_spawn_authority_denies() {
    let (context, tx) = update_meta_tx_with_plugin_authority("authority-spawn-deny", true);

    expect_tx_fail_with_code(&context, &tx, "error code 51");
}
