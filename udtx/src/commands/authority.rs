use crate::config::UdtxConfig;
use crate::error::TokenCliError;

pub fn authority_show(config: &UdtxConfig) -> Result<(), TokenCliError> {
    println!("Authority Configuration");
    println!("=======================");
    println!("Token Kind: {:?}", config.token.kind);
    println!("Symbol: {}", config.token.symbol);
    println!();

    let auth = &config.token.authorities;
    if auth.mint.is_none() && auth.metadata.is_none() && auth.access.is_none() {
        println!("No authorities configured.");
        return Ok(());
    }

    if let Some(ref account) = auth.mint {
        println!("  Mint Authority:     {}", account);
    } else {
        println!("  Mint Authority:     (not set)");
    }
    if let Some(ref account) = auth.metadata {
        println!("  Metadata Authority: {}", account);
    } else {
        println!("  Metadata Authority: (not set)");
    }
    if let Some(ref account) = auth.access {
        println!("  Access Authority:   {}", account);
    } else {
        println!("  Access Authority:   (not set)");
    }

    Ok(())
}

pub fn authority_update(
    config: &mut UdtxConfig,
    role: &str,
    account: &str,
) -> Result<(), TokenCliError> {
    if !config.accounts.contains_key(account) && account != "owner_lock" {
        return Err(TokenCliError::Config(
            crate::config::ConfigError::Validation(format!(
                "Authority account '{}' is not defined in accounts",
                account
            )),
        ));
    }

    match role {
        "mint" => config.token.authorities.mint = Some(account.to_string()),
        "metadata" => config.token.authorities.metadata = Some(account.to_string()),
        "access" => config.token.authorities.access = Some(account.to_string()),
        other => {
            return Err(TokenCliError::Config(
                crate::config::ConfigError::Validation(format!(
                    "Unknown authority role '{}'. Expected: mint, metadata, access",
                    other
                )),
            ))
        }
    }

    println!("Updated '{}' authority to '{}'", role, account);
    Ok(())
}

pub fn authority_drop(config: &mut UdtxConfig, role: &str, yes: bool) -> Result<(), TokenCliError> {
    if !yes {
        println!("This will remove the '{}' authority. Use --yes to confirm.", role);
        return Ok(());
    }

    match role {
        "mint" => config.token.authorities.mint = None,
        "metadata" => config.token.authorities.metadata = None,
        "access" => config.token.authorities.access = None,
        other => {
            return Err(TokenCliError::Config(
                crate::config::ConfigError::Validation(format!(
                    "Unknown authority role '{}'. Expected: mint, metadata, access",
                    other
                )),
            ))
        }
    }

    println!("Dropped '{}' authority.", role);
    Ok(())
}
