use crate::config::{AccessControlConfig, AccessMode, TokenKind, UdtxConfig};
use crate::error::TokenCliError;

pub fn access_list(config: &UdtxConfig) -> Result<(), TokenCliError> {
    println!("Access List");
    println!("===========");

    match &config.access_control {
        Some(ac) => {
            println!("Enabled: {}", ac.enabled);
            println!("Mode: {:?}", ac.mode);
            println!("Addresses ({}):", ac.addresses.len());
            for addr in &ac.addresses {
                println!("  - {}", addr);
            }
            if ac.addresses.is_empty() {
                println!("  (none)");
            }
        }
        None => {
            println!("Access control is not configured.");
        }
    }

    Ok(())
}

pub fn access_add(config: &mut UdtxConfig, address: &str) -> Result<(), TokenCliError> {
    if matches!(config.token.kind, TokenKind::Sudt) {
        return Err(TokenCliError::Config(
            crate::config::ConfigError::Validation(
                "Access control is not supported for sUDT".into()
            )
        ));
    }

    if config.access_control.is_none() {
        config.access_control = Some(AccessControlConfig {
            enabled: true,
            mode: AccessMode::Blacklist,
            addresses: vec![],
        });
    }

    let ac = config.access_control.as_mut().unwrap();
    if !ac.addresses.contains(&address.to_string()) {
        ac.addresses.push(address.to_string());
    }

    println!("Added '{}' to access list.", address);
    Ok(())
}

pub fn access_remove(config: &mut UdtxConfig, address: &str) -> Result<(), TokenCliError> {
    if let Some(ac) = config.access_control.as_mut() {
        let before = ac.addresses.len();
        ac.addresses.retain(|a| a != address);
        let after = ac.addresses.len();
        if after < before {
            println!("Removed '{}' from access list.", address);
        } else {
            println!("'{}' was not in the access list.", address);
        }
    } else {
        println!("Access control is not configured.");
    }
    Ok(())
}
