use std::io;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize the global tracing subscriber.
///
/// Reads the `RUST_LOG` environment variable for log level directives.
/// Falls back to `info` if `RUST_LOG` is not set.
///
/// # Safety
/// Never logs private keys, secret material, or raw RPC payloads that may
/// contain sensitive data.
pub fn init() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(io::stderr))
        .with(filter)
        .init();
}

/// Re-export tracing macros so callers can use `crate::log::info!` etc.
pub use tracing::{debug, error, info, trace, warn};
