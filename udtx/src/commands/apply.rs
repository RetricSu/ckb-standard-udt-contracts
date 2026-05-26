use crate::config::{UdtxConfig, ProfileConfig, TokenKind};
use crate::error::TokenCliError;
use crate::keys::KeyManager;

pub async fn apply(
    _config_path: Option<String>,
    yes: bool,
    config: &UdtxConfig,
    profile: &ProfileConfig,
    key_manager: &mut KeyManager,
) -> Result<(), TokenCliError> {
    if config.scenario.is_empty() {
        println!("No scenario steps to apply.");
        return Ok(());
    }

    if !yes {
        println!(
            "This will execute {} scenario step(s). Use --yes to confirm.",
            config.scenario.len()
        );
        return Ok(());
    }

    for (i, step) in config.scenario.iter().enumerate() {
        println!("\n[Step {}/{}] Action: {}", i + 1, config.scenario.len(), step.action);

        match step.action.as_str() {
            "issue" => {
                let amount = step.params.get("amount")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0")
                    .to_string();
                let token_type = step.params.get("token_type")
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "sudt" => TokenKind::Sudt,
                        "xudt" => TokenKind::Xudt,
                        _ => config.token.kind.clone(),
                    });
                let owner = step.params.get("owner")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                crate::commands::token::create::create_token(
                    token_type.unwrap_or(config.token.kind.clone()),
                    step.params.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    step.params.get("symbol").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    step.params.get("decimals").and_then(|v| v.as_u64()).map(|d| d as u8),
                    Some(amount),
                    owner,
                    false,
                    config,
                    profile,
                    key_manager,
                ).await?;
            }
            "transfer" => {
                let to = step.params.get("to")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| TokenCliError::TxBuild { message: "transfer step missing 'to'".into() })?
                    .to_string();
                let amount = step.params.get("amount")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| TokenCliError::TxBuild { message: "transfer step missing 'amount'".into() })?
                    .to_string();
                let token_type = step.params.get("token_type")
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "sudt" => TokenKind::Sudt,
                        "xudt" => TokenKind::Xudt,
                        _ => config.token.kind.clone(),
                    });
                let owner = step.params.get("owner")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                crate::commands::token::transfer::transfer_token(
                    to,
                    amount,
                    token_type,
                    owner,
                    false,
                    config,
                    profile,
                    key_manager,
                ).await?;
            }
            "mint" => {
                let amount = step.params.get("amount")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| TokenCliError::TxBuild { message: "mint step missing 'amount'".into() })?
                    .to_string();
                let token_type = step.params.get("token_type")
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "sudt" => TokenKind::Sudt,
                        "xudt" => TokenKind::Xudt,
                        _ => config.token.kind.clone(),
                    });
                let owner = step.params.get("owner")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                crate::commands::token::mint::mint_token(
                    amount,
                    token_type,
                    owner,
                    false,
                    config,
                    profile,
                    key_manager,
                ).await?;
            }
            "burn" => {
                let amount = step.params.get("amount")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| TokenCliError::TxBuild { message: "burn step missing 'amount'".into() })?
                    .to_string();
                let token_type = step.params.get("token_type")
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "sudt" => TokenKind::Sudt,
                        "xudt" => TokenKind::Xudt,
                        _ => config.token.kind.clone(),
                    });
                let owner = step.params.get("owner")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                crate::commands::token::burn::burn_token(
                    amount,
                    token_type,
                    owner,
                    false,
                    config,
                    profile,
                    key_manager,
                ).await?;
            }
            "info" => {
                let token_type = step.params.get("token_type")
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "sudt" => TokenKind::Sudt,
                        "xudt" => TokenKind::Xudt,
                        _ => config.token.kind.clone(),
                    });
                let owner = step.params.get("owner")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                crate::commands::token::info::token_info(
                    owner,
                    token_type,
                    config,
                    profile,
                ).await?;
            }
            other => {
                println!("  Skipping unknown action: {}", other);
            }
        }
    }

    println!("\nScenario execution complete.");
    Ok(())
}
