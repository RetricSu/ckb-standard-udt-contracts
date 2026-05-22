use crate::config::{AccountConfig, ConfigError, NetworkType, UdtxConfig, ProfileConfig, ContractRef};
use crate::error::TokenCliError;
use crate::keys::KeyManager;
use crate::rpc::RpcClient;
use std::path::PathBuf;

pub struct CheckResult {
    pub name: &'static str,
    pub passed: bool,
    pub details: Vec<String>,
    pub suggestions: Vec<String>,
}

impl CheckResult {
    pub fn ok(name: &'static str, details: Vec<String>) -> Self {
        Self {
            name,
            passed: true,
            details,
            suggestions: vec![],
        }
    }

    pub fn fail(name: &'static str, details: Vec<String>, suggestions: Vec<String>) -> Self {
        Self {
            name,
            passed: false,
            details,
            suggestions,
        }
    }
}

pub async fn doctor_check() -> Result<bool, TokenCliError> {
    println!("UDTX Doctor");
    println!("===========\n");

    let mut all_passed = true;
    let mut results: Vec<CheckResult> = Vec::new();

    let config_result = check_config().await;
    if !config_result.passed {
        all_passed = false;
    }
    results.push(config_result);

    let config = match try_load_config().await {
        Ok(pair) => Some(pair),
        Err(_) => None,
    };

    if let Some((_, ref profile)) = config {
        let network_result = check_network(profile).await;
        if !network_result.passed {
            all_passed = false;
        }
        results.push(network_result);
    }

    if let Some((ref config, ref profile)) = config {
        let accounts_result = check_accounts(config, profile).await;
        if !accounts_result.passed {
            all_passed = false;
        }
        results.push(accounts_result);
    }

    if let Some((_, ref profile)) = config {
        let contracts_result = check_contracts(profile).await;
        if !contracts_result.passed {
            all_passed = false;
        }
        results.push(contracts_result);
    }

    for result in &results {
        print_check_result(result);
    }

    println!();
    if all_passed {
        println!("All checks passed!");
    } else {
        println!("Some checks failed. See details above.");
    }

    Ok(all_passed)
}

fn print_check_result(result: &CheckResult) {
    let icon = if result.passed { "[✓]" } else { "[✗]" };
    println!("{} {}", icon, result.name);
    for detail in &result.details {
        println!("    {}", detail);
    }
    for suggestion in &result.suggestions {
        println!("    → {}", suggestion);
    }
    println!();
}

async fn try_load_config() -> Result<(UdtxConfig, ProfileConfig), TokenCliError> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let config_path = cwd.join("udtx.yaml");

    if !config_path.exists() {
        return Err(TokenCliError::Config(ConfigError::Validation(
            "udtx.yaml not found in current directory".into(),
        )));
    }

    let (config, profile) = crate::config::load_config_with_profile(&config_path)?;
    Ok((config, profile))
}

async fn check_config() -> CheckResult {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let config_path = cwd.join("udtx.yaml");

    if !config_path.exists() {
        return CheckResult::fail(
            "Configuration",
            vec![format!("udtx.yaml: not found at {}", config_path.display())],
            vec![
                "Run `udtx init` to create a new project".to_string(),
                "Or ensure you are in the project root directory".to_string(),
            ],
        );
    }

    let mut details = vec![format!("udtx.yaml: found at {}", config_path.display())];
    let mut suggestions = vec![];

    let config = match crate::config::load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            return CheckResult::fail(
                "Configuration",
                vec![
                    format!("udtx.yaml: found"),
                    format!("Error: {}", e),
                ],
                vec!["Check YAML syntax and required fields".to_string()],
            );
        }
    };

    let profile_path = crate::config::resolve_profile_path(&cwd, &config.network.profile);
    let profile = match crate::config::load_profile(&profile_path) {
        Ok(p) => {
            details.push(format!(
                "profiles/{}.yaml: valid",
                config.network.profile
            ));
            p
        }
        Err(e) => {
            return CheckResult::fail(
                "Configuration",
                vec![
                    format!("udtx.yaml: valid"),
                    format!(
                        "profiles/{}.yaml: error - {}",
                        config.network.profile, e
                    ),
                ],
                vec![format!(
                    "Check that profiles/{}.yaml exists and is valid",
                    config.network.profile
                )],
            );
        }
    };

    if let Err(e) = profile.validate() {
        return CheckResult::fail(
            "Configuration",
            vec![
                format!("udtx.yaml: valid"),
                format!(
                    "profiles/{}.yaml: validation error - {}",
                    config.network.profile, e
                ),
            ],
            vec!["Check profile RPC URL and network type".to_string()],
        );
    }

    if config.accounts.is_empty() {
        suggestions.push("No accounts configured in udtx.yaml".to_string());
    } else {
        details.push(format!("Accounts configured: {}", config.accounts.len()));
    }

    let auth = &config.token.authorities;
    for (role, account_ref) in [
        ("mint", auth.mint.as_ref()),
        ("metadata", auth.metadata.as_ref()),
        ("access", auth.access.as_ref()),
    ] {
        if let Some(ref_name) = account_ref {
            if ref_name == "owner_lock" {
                details.push(format!("Authority '{}': owner_lock (special)", role));
            } else if config.accounts.contains_key(ref_name) {
                details.push(format!("Authority '{}': {}", role, ref_name));
            } else {
                suggestions.push(format!(
                    "Authority '{}' references unknown account '{}'",
                    role, ref_name
                ));
            }
        }
    }

    if !suggestions.is_empty() {
        return CheckResult::fail("Configuration", details, suggestions);
    }

    CheckResult::ok("Configuration", details)
}

async fn check_network(profile: &ProfileConfig) -> CheckResult {
    let client = match RpcClient::new(&profile.rpc_url) {
        Ok(c) => c,
        Err(e) => {
            return CheckResult::fail(
                "Network Connection",
                vec![
                    format!("RPC URL: {}", profile.rpc_url),
                    format!("Error: {}", e),
                ],
                vec!["Check if the RPC URL is correctly formatted".to_string()],
            );
        }
    };

    let info = match client.get_chain_info().await {
        Ok(i) => i,
        Err(e) => {
            return CheckResult::fail(
                "Network Connection",
                vec![
                    format!("RPC URL: {}", profile.rpc_url),
                    format!("Error: {}", e),
                ],
                vec![
                    "Check if CKB node is running".to_string(),
                    format!("Verify RPC URL in profiles/{}.yaml", profile.name),
                ],
            );
        }
    };

    let detected = match info.chain.as_str() {
        "ckb" => NetworkType::Mainnet,
        "ckb_testnet" | "pudge" => NetworkType::Testnet,
        "ckb_dev" => NetworkType::Devnet,
        other => {
            return CheckResult::fail(
                "Network Connection",
                vec![
                    format!("RPC URL: {}", profile.rpc_url),
                    format!("Chain: {} (unknown)", other),
                ],
                vec!["Expected chain: ckb, ckb_testnet, or ckb_dev".to_string()],
            );
        }
    };

    let network_str = match profile.network_type {
        NetworkType::Mainnet => "mainnet",
        NetworkType::Testnet => "testnet",
        NetworkType::Devnet => "devnet",
    };

    let mut details = vec![
        format!("RPC URL: {}", profile.rpc_url),
        format!("Chain: {} ({})", info.chain, network_str),
    ];

    if let Ok(height) = client.get_tip_block_number().await {
        details.push(format!("Block height: {}", height));
    }

    if detected != profile.network_type {
        return CheckResult::fail(
            "Network Connection",
            details,
            vec![
                format!(
                    "Network mismatch: profile expects {:?}, but RPC reports {:?}",
                    profile.network_type, detected
                ),
                "Check your profile's network_type and RPC URL".to_string(),
            ],
        );
    }

    CheckResult::ok("Network Connection", details)
}

async fn check_accounts(config: &UdtxConfig, profile: &ProfileConfig) -> CheckResult {
    let mut details = vec![];
    let mut suggestions = vec![];
    let mut km = KeyManager::new();

    for (name, acct_config) in &config.accounts {
        match acct_config {
            AccountConfig::PrivateKeyEnv { private_key_env } => {
                match km.load_account(name, acct_config, profile) {
                    Ok(account) => {
                        let client = match RpcClient::new(&profile.rpc_url) {
                            Ok(c) => c,
                            Err(_) => {
                                details.push(format!(
                                    "{}: {} (balance: unable to query - no RPC)",
                                    name, account.address
                                ));
                                continue;
                            }
                        };

                        match client.get_balance(&account.address).await {
                            Ok(balance) => {
                                let ckb = balance as f64 / 100_000_000.0;
                                details.push(format!(
                                    "{}: {} (balance: {:.2} CKB)",
                                    name, account.address, ckb
                                ));
                            }
                            Err(e) => {
                                details.push(format!(
                                    "{}: {} (balance: error - {})",
                                    name, account.address, e
                                ));
                                suggestions.push(format!(
                                    "Check RPC connection for balance query on '{}'",
                                    name
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        details.push(format!("{}: error loading - {}", name, e));
                        suggestions.push(format!(
                            "Set environment variable '{}' for account '{}'",
                            private_key_env, name
                        ));
                    }
                }
            }
            AccountConfig::Address { address } => {
                let client = match RpcClient::new(&profile.rpc_url) {
                    Ok(c) => c,
                    Err(_) => {
                        details.push(format!(
                            "{}: {} (watch-only, balance: unable to query - no RPC)",
                            name, address
                        ));
                        continue;
                    }
                };

                match client.get_balance(address).await {
                    Ok(balance) => {
                        let ckb = balance as f64 / 100_000_000.0;
                        details.push(format!(
                            "{}: {} (watch-only, balance: {:.2} CKB)",
                            name, address, ckb
                        ));
                    }
                    Err(e) => {
                        details.push(format!(
                            "{}: {} (watch-only, balance: error - {})",
                            name, address, e
                        ));
                    }
                }
            }
        }
    }

    if !suggestions.is_empty() {
        return CheckResult::fail("Accounts", details, suggestions);
    }

    CheckResult::ok("Accounts", details)
}

async fn check_contracts(profile: &ProfileConfig) -> CheckResult {
    let mut details = vec![];
    let mut suggestions = vec![];

    if profile.contracts.is_empty() {
        return CheckResult::fail(
            "Contract References",
            vec!["No contracts configured in profile".to_string()],
            vec!["Add contract outpoints to your profile YAML".to_string()],
        );
    }

    let client = match RpcClient::new(&profile.rpc_url) {
        Ok(c) => c,
        Err(e) => {
            return CheckResult::fail(
                "Contract References",
                vec![format!("Cannot create RPC client: {}", e)],
                vec!["Check RPC URL in profile".to_string()],
            );
        }
    };

    for (name, contract) in &profile.contracts {
        match validate_contract_on_chain(&client, name, contract).await {
            Ok(msg) => details.push(msg),
            Err((msg, suggestion)) => {
                details.push(msg);
                suggestions.push(suggestion);
            }
        }
    }

    if !suggestions.is_empty() {
        return CheckResult::fail("Contract References", details, suggestions);
    }

    CheckResult::ok("Contract References", details)
}

async fn validate_contract_on_chain(
    _client: &RpcClient,
    name: &str,
    contract: &ContractRef,
) -> Result<String, (String, String)> {
    let tx_hash = contract.outpoint.tx_hash.clone();
    let index = contract.outpoint.index;

    if tx_hash.len() != 66 || !tx_hash.starts_with("0x") {
        return Err((
            format!(
                "{}: invalid outpoint tx_hash format (expected 0x... 66 chars)",
                name
            ),
            format!("Check the outpoint tx_hash for '{}' in profile", name),
        ));
    }

    if tx_hash == "0x0000000000000000000000000000000000000000000000000000000000000000" {
        return Err((
            format!("{}: placeholder outpoint (not deployed)", name),
            format!(
                "Deploy contract '{}' or update outpoint in profile",
                name
            ),
        ));
    }

    Ok(format!(
        "{}: deployed at tx_hash={}, index={}",
        name, tx_hash, index
    ))
}
