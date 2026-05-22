use crate::config::{SupplyMode, TokenKind, UdtxConfig, ProfileConfig};
use crate::error::{tx_build_error, TokenCliError};
use crate::keys::KeyManager;
use standard_udt_types::metadata::{Authority, AuthorityType};
use standard_udt_types::metadata::CONFIG_SUPPLY_TRACKED;
use standard_udt_types::metadata::SudtMeta;
use ckb_types::prelude::*;

pub async fn create_token(
    token_type: TokenKind,
    name: Option<String>,
    symbol: Option<String>,
    decimals: Option<u8>,
    supply: Option<String>,
    owner: Option<String>,
    dry_run: bool,
    config: &UdtxConfig,
    profile: &ProfileConfig,
    key_manager: &mut KeyManager,
) -> Result<(), TokenCliError> {
    let owner_name = owner.as_deref().unwrap_or("owner");
    
    let owner_account = config.accounts.get(owner_name)
        .ok_or_else(|| TokenCliError::AuthMissing {
            role: format!("owner account '{}' not found in config", owner_name),
        })?;
    
    let account = key_manager.load_account(owner_name, owner_account, profile)?;
    
    let lock_script_hash: [u8; 32] = account.lock_script.calc_script_hash().unpack();
    
    let authority = Authority {
        authority_type: AuthorityType::InputLock,
        script_hash: lock_script_hash,
        script: None,
    };
    
    let config_flags = match config.token.supply_policy.mode {
        SupplyMode::Tracked => CONFIG_SUPPLY_TRACKED,
        SupplyMode::Untracked => 0,
    };
    
    let current_supply = supply
        .as_ref()
        .and_then(|s| s.parse::<u128>().ok())
        .unwrap_or(0);
    
    let token_name = name
        .as_ref()
        .map(|n| n.as_bytes().to_vec())
        .unwrap_or_else(|| config.token.symbol.clone().into_bytes());
    
    let token_symbol = symbol
        .as_ref()
        .map(|s| s.as_bytes().to_vec())
        .unwrap_or_else(|| config.token.symbol.clone().into_bytes());
    
    let token_decimals = decimals.unwrap_or(config.token.decimals);
    
    let meta = SudtMeta {
        config_flags,
        current_supply,
        decimals: token_decimals,
        name: token_name,
        symbol: token_symbol,
        uri: vec![],
        extra_data: vec![],
        mint_authority: Some(authority.clone()),
        metadata_authority: Some(authority.clone()),
    };
    
    meta.to_bytes().map_err(|e| {
        tx_build_error(format!("Failed to serialize metadata: {:?}", e))
    })?;
    
    println!("Token Creation Preview");
    println!("=====================");
    println!("  Type: {:?}", token_type);
    println!("  Name: {}", String::from_utf8_lossy(&meta.name));
    println!("  Symbol: {}", String::from_utf8_lossy(&meta.symbol));
    println!("  Decimals: {}", meta.decimals);
    println!("  Supply: {}", meta.current_supply);
    println!("  Supply Tracked: {}", config_flags & CONFIG_SUPPLY_TRACKED != 0);
    println!("  Owner: {} ({})", owner_name, account.address);
    println!("  Mint Authority: InputLock({})", hex::encode(&lock_script_hash));
    println!("  Metadata Authority: InputLock({})", hex::encode(&lock_script_hash));
    
    if dry_run {
        println!("\n[Dry Run] Token creation preview complete. No transaction sent.");
        return Ok(());
    }
    
    println!("\nToken creation is not yet implemented. Use --dry-run to preview.");
    
    Ok(())
}
