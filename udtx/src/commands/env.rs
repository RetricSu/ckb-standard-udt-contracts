use crate::config::{UdtxConfig, ProfileConfig};
use crate::error::TokenCliError;
use crate::rpc::RpcClient;

pub async fn env_check(config: &UdtxConfig, profile: &ProfileConfig) -> Result<(), TokenCliError> {
    println!("UDTX Environment Check");
    println!("======================\n");

    println!("Configuration: {}", std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("udtx.yaml")
        .display());
    println!("  Profile: {}", config.network.profile);
    println!("  RPC URL: {}", profile.rpc_url);
    println!("  Network: {:?}", profile.network_type);
    println!();

    print!("Checking RPC connectivity... ");
    let client = match RpcClient::new(&profile.rpc_url) {
        Ok(c) => c,
        Err(e) => {
            println!("FAILED");
            println!("  Error: {}", e);
            println!();
            println!("Suggestions:");
            println!("  - Ensure a CKB node is running and accessible");
            println!("  - Check the RPC URL in profiles/{}.yaml", config.network.profile);
            return Ok(());
        }
    };
    println!("OK");

    print!("Fetching chain info... ");
    let info = match client.get_chain_info().await {
        Ok(i) => i,
        Err(e) => {
            println!("FAILED");
            println!("  Error: {}", e);
            return Ok(());
        }
    };
    println!("OK");
    println!();

    println!("Chain Details:");
    println!("  Chain:          {}", info.chain);
    println!("  Block Height:   {:?}", client.get_tip_block_number().await.unwrap_or(0));
    println!("  Epoch:          {}", info.epoch);
    println!("  Median Time:    {}", info.median_time);
    println!();

    let detected = match info.chain.as_str() {
        "ckb" => "mainnet",
        "ckb_testnet" | "pudge" => "testnet",
        "ckb_dev" => "devnet",
        other => other,
    };

    let expected = match profile.network_type {
        crate::config::NetworkType::Mainnet => "mainnet",
        crate::config::NetworkType::Testnet => "testnet",
        crate::config::NetworkType::Devnet => "devnet",
    };

    if detected != expected {
        println!("[WARN] Network mismatch: profile expects '{}', but RPC reports '{}'", expected, detected);
    } else {
        println!("[OK] Network type matches: {}", expected);
    }

    println!();
    println!("Environment check complete.");
    Ok(())
}
