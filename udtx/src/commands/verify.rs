use crate::config::{UdtxConfig, ProfileConfig, TokenKind};
use crate::error::TokenCliError;
use crate::keys::KeyManager;
use crate::rpc::RpcClient;
use ckb_sdk::rpc::ckb_indexer::{ScriptType, SearchKey, SearchMode};
use ckb_types::prelude::*;

pub async fn verify(
    config: &UdtxConfig,
    profile: &ProfileConfig,
    key_manager: &mut KeyManager,
) -> Result<(), TokenCliError> {
    println!("Verify Configuration and On-Chain State");
    println!("=======================================");

    let owner_name = "owner";
    let owner_account = config.accounts.get(owner_name)
        .ok_or_else(|| TokenCliError::AuthMissing {
            role: format!("owner account '{}' not found in config", owner_name),
        })?;

    let account = key_manager.load_account(owner_name, owner_account, profile)?;

    let client = RpcClient::new(&profile.rpc_url)?;

    // Validate network
    match client.validate_network(profile.network_type.clone()).await {
        Ok(()) => println!("  [PASS] Network type matches RPC node"),
        Err(e) => {
            println!("  [FAIL] Network mismatch: {}", e);
            return Ok(());
        }
    }

    // Check CKB balance
    let balance = client.get_balance(&account.address).await?;
    println!("  [INFO] CKB Balance: {} shannons ({} CKB)", balance, balance / 100_000_000);

    if balance == 0 {
        println!("  [WARN] Account has no CKB balance. Transactions will fail.");
    }

    // Check token cells
    let kind = &config.token.kind;
    let contract = profile.contracts.get(match kind {
        TokenKind::Sudt => "sudt",
        TokenKind::Xudt => "xudt",
    });

    if let Some(contract) = contract {
        let contract_code_hash = ckb_types::packed::Byte32::from_slice(
            &hex::decode(contract.code_hash.trim_start_matches("0x"))
                .map_err(|e| TokenCliError::TxBuild { message: format!("invalid code hash: {}", e) })?
        ).map_err(|e| TokenCliError::TxBuild { message: format!("invalid code hash bytes: {}", e) })?;

        let lock_script_hash: [u8; 32] = account.lock_script.calc_script_hash().unpack();

        let udt_type_script = ckb_types::packed::Script::new_builder()
            .code_hash(contract_code_hash)
            .hash_type(match contract.hash_type.as_str() {
                "type" => ckb_types::core::ScriptHashType::Type,
                "data" | "data1" => ckb_types::core::ScriptHashType::Data,
                _ => ckb_types::core::ScriptHashType::Data,
            })
            .args(ckb_types::packed::Bytes::from(lock_script_hash.to_vec()))
            .build();

        let search_key = SearchKey {
            script: udt_type_script.clone().into(),
            script_type: ScriptType::Type,
            script_search_mode: Some(SearchMode::Exact),
            filter: None,
            with_data: Some(true),
            group_by_transaction: None,
        };

        let mut total_amount: u128 = 0;
        let mut cell_count = 0;
        let mut cursor = None;

        loop {
            let page = client.get_cells(search_key.clone(), 500, cursor.clone()).await?;
            for cell in &page.objects {
                if let Some(ref data) = cell.output_data {
                    let bytes = data.as_bytes();
                    if bytes.len() >= 16 {
                        let mut amount_bytes = [0u8; 16];
                        amount_bytes.copy_from_slice(&bytes[..16]);
                        let amount = u128::from_le_bytes(amount_bytes);
                        total_amount += amount;
                        cell_count += 1;
                    }
                }
            }
            if page.objects.len() < 500 {
                break;
            }
            cursor = Some(page.last_cursor);
        }

        println!("  [INFO] Token Type: {:?}", kind);
        println!("  [INFO] Token Cells Found: {}", cell_count);
        println!("  [INFO] Total Token Balance: {}", total_amount);

        if cell_count == 0 {
            println!("  [WARN] No token cells found on-chain. Token may not be issued yet.");
        }
    } else {
        println!("  [WARN] Contract reference for {:?} not found in profile", kind);
    }

    // Validate authorities
    let auth = &config.token.authorities;
    for (role, account_ref) in [
        ("mint", auth.mint.as_ref()),
        ("metadata", auth.metadata.as_ref()),
        ("access", auth.access.as_ref()),
    ] {
        if let Some(ref_name) = account_ref {
            if config.accounts.contains_key(ref_name) || ref_name == "owner_lock" {
                println!("  [PASS] Authority '{}' -> '{}' is valid", role, ref_name);
            } else {
                println!("  [FAIL] Authority '{}' references unknown account: {}", role, ref_name);
            }
        } else {
            println!("  [INFO] Authority '{}' is not set", role);
        }
    }

    println!("\nVerification complete.");
    Ok(())
}
