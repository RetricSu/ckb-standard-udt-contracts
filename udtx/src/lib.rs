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
    /// Start the devnet
    Up {
        /// Run in background (default: true)
        #[arg(long, default_value = "true")]
        background: bool,
    },
    /// Stop the devnet
    Down,
    /// Reset devnet data
    Reset {
        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Show chain status
    Status,
}

#[derive(Subcommand, Debug)]
pub enum TokenCommands {
    Issue {
        #[arg(short = 't', long, value_enum, default_value = "sudt")]
        token_type: config::TokenKind,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        symbol: Option<String>,
        #[arg(short, long)]
        decimals: Option<u8>,
        #[arg(short = 'S', long)]
        supply: Option<String>,
        #[arg(short, long)]
        owner: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Transfer tokens to a recipient
    Transfer {
        /// Recipient address
        #[arg(short, long)]
        to: String,
        /// Amount to transfer
        #[arg(short, long)]
        amount: String,
        /// Token type override
        #[arg(short = 't', long, value_enum)]
        token_type: Option<config::TokenKind>,
        /// Owner account name
        #[arg(short, long)]
        owner: Option<String>,
        /// Dry run - preview transaction without sending
        #[arg(long)]
        dry_run: bool,
    },
    /// Mint new tokens
    Mint {
        /// Amount to mint
        #[arg(short, long)]
        amount: String,
        /// Token type override
        #[arg(short = 't', long, value_enum)]
        token_type: Option<config::TokenKind>,
        /// Owner account name
        #[arg(short, long)]
        owner: Option<String>,
        /// Dry run - preview transaction without sending
        #[arg(long)]
        dry_run: bool,
    },
    /// Burn tokens
    Burn {
        /// Amount to burn
        #[arg(short, long)]
        amount: String,
        /// Token type override
        #[arg(short = 't', long, value_enum)]
        token_type: Option<config::TokenKind>,
        /// Owner account name
        #[arg(short, long)]
        owner: Option<String>,
        /// Dry run - preview transaction without sending
        #[arg(long)]
        dry_run: bool,
    },
    /// Show token info and balances
    Info {
        /// Owner account name
        #[arg(short, long)]
        owner: Option<String>,
        /// Token type override
        #[arg(short = 't', long, value_enum)]
        token_type: Option<config::TokenKind>,
    },
}

#[derive(Subcommand, Debug)]
pub enum AccessCommands {
    /// Show access list
    List,
    /// Add an entry
    Add {
        /// Address to add
        #[arg(short, long)]
        address: String,
    },
    /// Remove an entry
    Remove {
        /// Address to remove
        #[arg(short, long)]
        address: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum AuthorityCommands {
    /// Show current authority
    Show,
    /// Update authority
    Update {
        /// Authority role (mint, metadata, access)
        #[arg(short, long)]
        role: String,
        /// Account name to set
        #[arg(short, long)]
        account: String,
    },
    /// Drop authority (requires --yes)
    Drop {
        /// Authority role to drop
        #[arg(short, long)]
        role: String,
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
