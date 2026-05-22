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
            logger::debug!(?command, "Chain command");
            println!("chain: {:?}", command);
        }
        Commands::Token { command } => {
            logger::debug!(?command, "Token command");
            println!("token: {:?}", command);
        }
        Commands::Access { command } => {
            logger::debug!(?command, "Access command");
            println!("access: {:?}", command);
        }
        Commands::Authority { command } => {
            logger::debug!(?command, "Authority command");
            println!("authority: {:?}", command);
        }
        Commands::Plan { config } => {
            logger::info!(?config, "Plan command");
            println!("plan: {:?}", config);
        }
        Commands::Apply { config, yes } => {
            logger::info!(?config, yes, "Apply command");
            println!("apply: {:?} yes={}", config, yes);
        }
        Commands::Verify { config } => {
            logger::info!(?config, "Verify command");
            println!("verify: {:?}", config);
        }
        Commands::Report { format } => {
            logger::info!(?format, "Report command");
            println!("report: {:?}", format);
        }
    }

    Ok(())
}
