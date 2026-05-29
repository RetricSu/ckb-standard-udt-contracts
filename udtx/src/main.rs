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
    let config_path = cli.config;

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
            println!("  profiles/testnet.yaml");
            println!("  profiles/mainnet.yaml");
            println!("  artifacts/");
        }
        Commands::Doctor => {
            logger::info!("Running doctor check");
            let passed = udtx::commands::doctor::doctor_check(&config_path).await?;
            if !passed {
                std::process::exit(1);
            }
        }
        Commands::Env { command } => {
            match command {
                udtx::EnvCommands::Check => {
                    let (config, profile) = udtx::config::load_config_with_profile(&config_path)?;
                    udtx::commands::env::env_check(&config_path, &config, &profile).await?;
                }
            }
        }
        Commands::Token { command } => {
            match command {
                udtx::TokenCommands::Issue { token_type, name, symbol, decimals, supply, owner, dry_run } => {
                    let (config, profile) = udtx::config::load_config_with_profile(&config_path)?;
                    let mut key_manager = udtx::keys::KeyManager::new();
                    udtx::commands::token::create::create_token(
                        token_type, name, symbol, decimals, supply, owner, dry_run,
                        &config, &profile, &mut key_manager
                    ).await?;
                }
                udtx::TokenCommands::Transfer { to, amount, token_type, owner, dry_run } => {
                    let (config, profile) = udtx::config::load_config_with_profile(&config_path)?;
                    let mut key_manager = udtx::keys::KeyManager::new();
                    udtx::commands::token::transfer::transfer_token(
                        to, amount, token_type, owner, dry_run,
                        &config, &profile, &mut key_manager
                    ).await?;
                }
                udtx::TokenCommands::Mint { amount, token_type, owner, dry_run } => {
                    let (config, profile) = udtx::config::load_config_with_profile(&config_path)?;
                    let mut key_manager = udtx::keys::KeyManager::new();
                    udtx::commands::token::mint::mint_token(
                        amount, token_type, owner, dry_run,
                        &config, &profile, &mut key_manager
                    ).await?;
                }
                udtx::TokenCommands::Burn { amount, token_type, owner, dry_run } => {
                    let (config, profile) = udtx::config::load_config_with_profile(&config_path)?;
                    let mut key_manager = udtx::keys::KeyManager::new();
                    udtx::commands::token::burn::burn_token(
                        amount, token_type, owner, dry_run,
                        &config, &profile, &mut key_manager
                    ).await?;
                }
                udtx::TokenCommands::Info { owner, token_type } => {
                    let (config, profile) = udtx::config::load_config_with_profile(&config_path)?;
                    udtx::commands::token::info::token_info(
                        owner, token_type,
                        &config, &profile
                    ).await?;
                }
            }
        }
        Commands::Access { command } => {
            match command {
                udtx::AccessCommands::List => {
                    let (config, _profile) = udtx::config::load_config_with_profile(&config_path)?;
                    udtx::commands::access::access_list(&config)?;
                }
                udtx::AccessCommands::Add { address } => {
                    let (mut config, _profile) = udtx::config::load_config_with_profile(&config_path)?;
                    udtx::commands::access::access_add(&mut config, &address)?;
                    udtx::config::save_config(&config_path, &config)?;
                }
                udtx::AccessCommands::Remove { address } => {
                    let (mut config, _profile) = udtx::config::load_config_with_profile(&config_path)?;
                    udtx::commands::access::access_remove(&mut config, &address)?;
                    udtx::config::save_config(&config_path, &config)?;
                }
            }
        }
        Commands::Authority { command } => {
            match command {
                udtx::AuthorityCommands::Show => {
                    let (config, _profile) = udtx::config::load_config_with_profile(&config_path)?;
                    udtx::commands::authority::authority_show(&config)?;
                }
                udtx::AuthorityCommands::Update { role, account } => {
                    let (mut config, _profile) = udtx::config::load_config_with_profile(&config_path)?;
                    udtx::commands::authority::authority_update(&mut config, &role, &account)?;
                    udtx::config::save_config(&config_path, &config)?;
                }
                udtx::AuthorityCommands::Drop { role, yes } => {
                    let (mut config, _profile) = udtx::config::load_config_with_profile(&config_path)?;
                    udtx::commands::authority::authority_drop(&mut config, &role, yes)?;
                    if yes {
                        udtx::config::save_config(&config_path, &config)?;
                    }
                }
            }
        }
        Commands::Plan => {
            let (config, profile) = udtx::config::load_config_with_profile(&config_path)?;
            let mut key_manager = udtx::keys::KeyManager::new();
            udtx::commands::plan::plan(&config, &profile, &mut key_manager).await?;
        }
        Commands::Apply { yes } => {
            let (config, profile) = udtx::config::load_config_with_profile(&config_path)?;
            let mut key_manager = udtx::keys::KeyManager::new();
            udtx::commands::apply::apply(yes, &config, &profile, &mut key_manager).await?;
        }
        Commands::Verify => {
            let (config, profile) = udtx::config::load_config_with_profile(&config_path)?;
            let mut key_manager = udtx::keys::KeyManager::new();
            udtx::commands::verify::verify(&config, &profile, &mut key_manager).await?;
        }
        Commands::Report { format } => {
            let (config, profile) = udtx::config::load_config_with_profile(&config_path)?;
            let mut key_manager = udtx::keys::KeyManager::new();
            udtx::commands::report::report(format, &config, &profile, &mut key_manager).await?;
        }
    }

    Ok(())
}
