use udtx::{Cli, Commands};
use udtx::config;
use udtx::error::TokenCliError;
use udtx::logger;
use clap::Parser;
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    logger::init();

    if let Err(e) = run().await {
        logger::error!(error = %e, "udtx failed");
        eprintln!("Error: {}", e);
        std::process::exit(e.exit_code());
    }
}

async fn run() -> Result<(), TokenCliError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => {
            let project_name = name.unwrap_or_else(|| {
                env::current_dir()
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                    .unwrap_or_else(|| "udtx-project".to_string())
            });
            let project_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            config::init_project(&project_path, &project_name)?;
            logger::info!(project = %project_name, path = %project_path.display(), "Initialized UDTX project");
            println!("Initialized UDTX project '{}' at {}", project_name, project_path.display());
            println!("  udtx.yaml");
            println!("  profiles/devnet.yaml");
            println!("  artifacts/");
        }
        Commands::Doctor => {
            logger::info!("Running doctor check");
            let passed = udtx::commands::doctor::doctor_check().await?;
            if !passed {
                std::process::exit(1);
            }
        }
        Commands::Chain { command } => {
            match command {
                udtx::ChainCommands::Up { background } => {
                    udtx::commands::chain::chain_up(background).await?;
                }
                udtx::ChainCommands::Down => {
                    udtx::commands::chain::chain_down().await?;
                }
                udtx::ChainCommands::Reset { yes } => {
                    udtx::commands::chain::chain_reset(yes).await?;
                }
                udtx::ChainCommands::Status => {
                    udtx::commands::chain::chain_status().await?;
                }
            }
        }
        Commands::Token { command } => {
            match command {
                udtx::TokenCommands::Issue { token_type, name, symbol, decimals, supply, owner, dry_run } => {
                    let (config, profile) = udtx::config::load_config_with_profile("udtx.yaml")?;
                    let mut key_manager = udtx::keys::KeyManager::new();
                    udtx::commands::token::create::create_token(
                        token_type, name, symbol, decimals, supply, owner, dry_run,
                        &config, &profile, &mut key_manager
                    ).await?;
                }
                udtx::TokenCommands::Transfer => {
                    return Err(TokenCliError::NotImplemented {
                        feature: "token transfer".into(),
                    });
                }
                udtx::TokenCommands::Mint => {
                    return Err(TokenCliError::NotImplemented {
                        feature: "token mint".into(),
                    });
                }
                udtx::TokenCommands::Burn => {
                    return Err(TokenCliError::NotImplemented {
                        feature: "token burn".into(),
                    });
                }
                udtx::TokenCommands::Info => {
                    return Err(TokenCliError::NotImplemented {
                        feature: "token info".into(),
                    });
                }
            }
        }
        Commands::Access { command } => {
            logger::debug!(?command, "Access command");
            return Err(TokenCliError::NotImplemented {
                feature: format!("access {:?}", command).to_lowercase(),
            });
        }
        Commands::Authority { command } => {
            logger::debug!(?command, "Authority command");
            return Err(TokenCliError::NotImplemented {
                feature: format!("authority {:?}", command).to_lowercase(),
            });
        }
        Commands::Plan { config } => {
            logger::info!(?config, "Plan command");
            return Err(TokenCliError::NotImplemented {
                feature: "plan".into(),
            });
        }
        Commands::Apply { config, yes } => {
            logger::info!(?config, yes, "Apply command");
            return Err(TokenCliError::NotImplemented {
                feature: "apply".into(),
            });
        }
        Commands::Verify { config } => {
            logger::info!(?config, "Verify command");
            return Err(TokenCliError::NotImplemented {
                feature: "verify".into(),
            });
        }
        Commands::Report { format } => {
            logger::info!(?format, "Report command");
            return Err(TokenCliError::NotImplemented {
                feature: "report".into(),
            });
        }
    }

    Ok(())
}
