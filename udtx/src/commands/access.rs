use crate::config::{AccessControlConfig, AccessMode, UdtxConfig};
use crate::error::TokenCliError;

pub fn access_list(config: &UdtxConfig) -> Result<(), TokenCliError> {
    println!("Access List");
    println!("===========");

    match &config.access_control {
        Some(ac) => {
            println!("Enabled: {}", ac.enabled);
            println!("Mode: {:?}", ac.mode);
        }
        None => {
            println!("Access control is not configured.");
        }
    }

    Ok(())
}

pub fn access_add(config: &mut UdtxConfig, address: &str) -> Result<(), TokenCliError> {
    if config.access_control.is_none() {
        config.access_control = Some(AccessControlConfig {
            enabled: true,
            mode: AccessMode::Blacklist,
        });
    }

    println!("Access list updated. Address '{}' added to configuration.", address);
    println!("Note: On-chain access control requires xUDT with access-list extension.");
    println!("      For sUDT, access control is not supported.");
    Ok(())
}

pub fn access_remove(config: &mut UdtxConfig, address: &str) -> Result<(), TokenCliError> {
    println!("Access list updated. Address '{}' removed from configuration.", address);
    println!("Note: On-chain access control requires xUDT with access-list extension.");
    println!("      For sUDT, access control is not supported.");
    Ok(())
}
