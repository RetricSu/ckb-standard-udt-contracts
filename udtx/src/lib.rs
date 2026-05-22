pub mod commands;
pub mod config;
pub mod error;
pub mod keys;
pub mod logger;
pub mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "udtx")]
#[command(about = "CKB UDT Token Operations CLI")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new UDTX project or configuration
    Init {
        /// Project name
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Check environment and dependencies
    Doctor,
    /// Chain operations (info, status, etc.)
    Chain {
        /// Subcommand for chain operations
        #[command(subcommand)]
        command: ChainCommands,
    },
    /// Token operations (issue, transfer, mint, burn, etc.)
    Token {
        /// Subcommand for token operations
        #[command(subcommand)]
        command: TokenCommands,
    },
    /// Access list operations
    Access {
        /// Subcommand for access list operations
        #[command(subcommand)]
        command: AccessCommands,
    },
    /// Authority management
    Authority {
        /// Subcommand for authority operations
        #[command(subcommand)]
        command: AuthorityCommands,
    },
    /// Plan and preview changes
    Plan {
        /// Path to configuration file
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Apply planned changes
    Apply {
        /// Path to configuration file
        #[arg(short, long)]
        config: Option<String>,
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Verify configuration or on-chain state
    Verify {
        /// Path to configuration file
        #[arg(short, long)]
        config: Option<String>,
    },
    /// Generate reports
    Report {
        /// Report format
        #[arg(short, long, value_enum, default_value = "markdown")]
        format: ReportFormat,
    },
}

#[derive(Subcommand, Debug)]
pub enum ChainCommands {
    /// Show chain info
    Info,
    /// Show chain status
    Status,
}

#[derive(Subcommand, Debug)]
pub enum TokenCommands {
    /// Issue a new token
    Issue,
    /// Transfer tokens
    Transfer,
    /// Mint new supply
    Mint,
    /// Burn supply
    Burn,
    /// Show token info
    Info,
}

#[derive(Subcommand, Debug)]
pub enum AccessCommands {
    /// Show access list
    List,
    /// Add an entry
    Add,
    /// Remove an entry
    Remove,
}

#[derive(Subcommand, Debug)]
pub enum AuthorityCommands {
    /// Show current authority
    Show,
    /// Update authority
    Update,
    /// Drop authority (requires --yes)
    Drop {
        /// Confirm drop
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum ReportFormat {
    Markdown,
    Json,
}
