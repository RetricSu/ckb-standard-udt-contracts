use crate::config::{ProfileConfig, TokenKind, UdtxConfig};
use crate::error::TokenCliError;
use crate::keys::KeyManager;
use crate::rpc::RpcClient;
use ckb_sdk::rpc::ckb_indexer::{ScriptType, SearchKey, SearchMode};
use ckb_types::prelude::*;

pub async fn token_info(
    owner: Option<String>,
    token_type: Option<TokenKind>,
    config: &UdtxConfig,
    profile: &ProfileConfig,
) -> Result<(), TokenCliError> {
    let owner_name = owner.as_deref().unwrap_or("owner");
    let owner_account = config.accounts.get(owner_name)
        .ok_or_else(|| TokenCliError::AuthMissing {
            role: format!("owner account '{}' not found in config", owner_name),
        })?;

    let mut key_manager = KeyManager::new();
    let account = key_manager.load_account(owner_name, owner_account, profile)?;

    let kind = token_type.as_ref().unwrap_or(&config.token.kind);

    let contract = profile.contracts.get(match kind {
        TokenKind::Sudt => "sudt",
        TokenKind::Xudt => "xudt",
    }).ok_or_else(|| TokenCliError::Config(
        crate::config::ConfigError::Validation(
            format!("Contract reference for {:?} not found in profile", kind)
        )
    ))?;

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

    let client = RpcClient::new(&profile.rpc_url)?;

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

    println!("Token Info");
    println!("==========");
    println!("  Token Type: {:?}", kind);
    println!("  Symbol: {}", config.token.symbol);
    println!("  Decimals: {}", config.token.decimals);
    println!("  Owner: {} ({})", owner_name, account.address);
    println!("  Owner Lock Script Hash: 0x{}", hex::encode(&lock_script_hash));
    println!("  Type Script Code Hash: {}", contract.code_hash);
    println!("  Supply Tracked: {:?}", config.token.supply_policy.mode);
    println!("  Cells Found: {}", cell_count);
    println!("  Total Balance: {}", total_amount);

    Ok(())
}
