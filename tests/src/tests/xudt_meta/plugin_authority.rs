use super::*;

fn xudt_meta_access_update_with_plugin_authority(plugin_name: &str, spawn: bool) -> UpdateCase {
    update_meta_tx(|context, lock, meta| {
        let plugin = if spawn {
            deploy_data2_script(context, plugin_name, Bytes::from_static(b"allow"))
        } else {
            deploy_data_script(context, plugin_name, Bytes::from_static(b"allow"))
        };
        let authority = if spawn {
            spawn_authority(&plugin)
        } else {
            dynamic_linking_authority(&plugin)
        };
        let access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(0, 0, None, None, Some(authority.clone()), Vec::new()),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST | CONFIG_PAUSED,
                0,
                None,
                None,
                Some(authority),
                Vec::new(),
            ),
            vec![
                ExtraCell::Output {
                    lock: lock.script.clone(),
                    type_script: access_list.script.clone(),
                    data: full_domain_shard(),
                    cell_dep: access_list,
                },
                ExtraCell::Dep { cell_dep: plugin },
            ],
        )
    })
}

#[test]
fn xudt_meta_access_update_with_dynamic_linking_authority_passes() {
    let case = xudt_meta_access_update_with_plugin_authority("authority-dl-allow", false);

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_access_update_with_dynamic_linking_authority_denies() {
    let case = xudt_meta_access_update_with_plugin_authority("authority-dl-deny", false);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 51");
}

#[test]
fn xudt_meta_mint_authority_fallback_survives_broken_access_authority() {
    let case = update_meta_tx(|context, lock, meta| {
        let plugin =
            deploy_data_script(context, "authority-dl-allow", Bytes::from_static(b"allow"));
        let mint_authority = input_lock_authority(lock.script_hash);
        let access_authority = dynamic_linking_authority(&plugin);
        let access_list = access_list_script(context, meta.script_hash);
        (
            xudt_meta_data(
                0,
                0,
                Some(mint_authority.clone()),
                None,
                Some(access_authority.clone()),
                Vec::new(),
            ),
            xudt_meta_data(
                CONFIG_ACCESS_ENABLED | CONFIG_ACCESS_WHITELIST,
                0,
                Some(mint_authority),
                None,
                Some(access_authority),
                Vec::new(),
            ),
            vec![ExtraCell::Output {
                lock: lock.script.clone(),
                type_script: access_list.script.clone(),
                data: full_domain_shard(),
                cell_dep: access_list,
            }],
        )
    });

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_access_update_with_spawn_authority_passes() {
    let case = xudt_meta_access_update_with_plugin_authority("authority-spawn-allow", true);

    expect_tx_pass(&case.context, &case.tx);
}

#[test]
fn xudt_meta_access_update_with_spawn_authority_denies() {
    let case = xudt_meta_access_update_with_plugin_authority("authority-spawn-deny", true);

    expect_tx_fail_with_code(&case.context, &case.tx, "error code 51");
}
