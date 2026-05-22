use crate::config::ConfigError;

/// Product-level error codes for CLI exit status and user guidance.
pub mod codes {
    pub const E_CONFIG: i32 = 10;
    pub const E_RPC: i32 = 20;
    pub const E_TX_BUILD: i32 = 30;
    pub const E_PRECHECK_CAPACITY: i32 = 40;
    pub const E_AUTH_MISSING: i32 = 50;
    pub const E_AUTH_FAILED: i32 = 51;
    pub const E_ACCESS_DENIED: i32 = 60;
    pub const E_SUPPLY_INVALID: i32 = 70;
    pub const E_OVERFLOW: i32 = 80;
    pub const E_USER_CANCELLED: i32 = 90;
    pub const E_VERIFICATION_FAILED: i32 = 100;
    pub const E_UNKNOWN: i32 = 1;
}

/// The top-level error type for the `udtx` CLI.
///
/// Every variant carries a user-friendly message with actionable guidance.
/// Use `TokenCliError::exit_code()` to map errors to process exit codes.
#[derive(Debug, thiserror::Error)]
pub enum TokenCliError {
    /// Configuration file or validation error.
    #[error("Configuration error: {0}\n  → Check udtx.yaml and profile YAML for syntax and values.")]
    Config(#[from] ConfigError),

    /// RPC communication or node response error.
    #[error("RPC error: {message}\n  → Check your network connection and verify the RPC URL in your profile.")]
    Rpc { message: String },

    /// Transaction building failed (e.g. cell collector, script group, or fee issues).
    #[error("Transaction building failed: {message}\n  → Review your config, ensure sufficient cells, and check contract references.")]
    TxBuild { message: String },

    /// Not enough CKB capacity to construct the transaction.
    #[error(
        "Insufficient capacity: required {required} CKB, available {available} CKB.\n\
         → Add more CKB to the signing account or reduce the output count."
    )]
    InsufficientCapacity { required: u64, available: u64 },

    /// A required authority role is not configured.
    #[error(
        "Missing authority: {role}\n\
         → Ensure the required account is configured in udtx.yaml under token.authorities."
    )]
    AuthMissing { role: String },

    /// Authority check failed (signature or script hash mismatch).
    #[error(
        "Authority check failed for role: {role}\n\
         → Ensure the configured account has the required permissions and the private key is correct."
    )]
    AuthFailed { role: String },

    /// Address is not allowed by the current access mode.
    #[error(
        "Access denied for address: {address}\n\
         → Check the access-list configuration or adjust the access mode in udtx.yaml."
    )]
    AccessDenied { address: String },

    /// Supply invariant violated (expected vs actual mismatch).
    #[error(
        "Supply invariant violated: expected {expected}, actual {actual}\n\
         → Verify token amounts and metadata supply tracking settings."
    )]
    SupplyInvalid { expected: u128, actual: u128 },

    /// Numeric overflow during amount calculation.
    #[error("Amount overflow.\n  → Use smaller token amounts or check for accumulation errors.")]
    Overflow,

    /// User explicitly cancelled the operation.
    #[error("Operation cancelled by user")]
    UserCancelled,

    /// On-chain or local verification failed.
    #[error("Verification failed: {message}\n  → Review the transaction details and try again.")]
    VerificationFailed { message: String },
}

impl TokenCliError {
    /// Map this error to a product-level exit code.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Config(_) => codes::E_CONFIG,
            Self::Rpc { .. } => codes::E_RPC,
            Self::TxBuild { .. } => codes::E_TX_BUILD,
            Self::InsufficientCapacity { .. } => codes::E_PRECHECK_CAPACITY,
            Self::AuthMissing { .. } => codes::E_AUTH_MISSING,
            Self::AuthFailed { .. } => codes::E_AUTH_FAILED,
            Self::AccessDenied { .. } => codes::E_ACCESS_DENIED,
            Self::SupplyInvalid { .. } => codes::E_SUPPLY_INVALID,
            Self::Overflow => codes::E_OVERFLOW,
            Self::UserCancelled => codes::E_USER_CANCELLED,
            Self::VerificationFailed { .. } => codes::E_VERIFICATION_FAILED,
        }
    }
}

/// Convenience `From` impls for common upstream error types.

impl From<std::io::Error> for TokenCliError {
    fn from(err: std::io::Error) -> Self {
        Self::Config(ConfigError::Io(err))
    }
}

impl From<serde_yaml::Error> for TokenCliError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::Config(ConfigError::Yaml(err))
    }
}

/// Helper to wrap an RPC error string.
pub fn rpc_error(message: impl Into<String>) -> TokenCliError {
    TokenCliError::Rpc {
        message: message.into(),
    }
}

/// Helper to wrap a transaction-build error string.
pub fn tx_build_error(message: impl Into<String>) -> TokenCliError {
    TokenCliError::TxBuild {
        message: message.into(),
    }
}

/// Helper to wrap a verification error string.
pub fn verification_error(message: impl Into<String>) -> TokenCliError {
    TokenCliError::VerificationFailed {
        message: message.into(),
    }
}
