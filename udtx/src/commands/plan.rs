use crate::config::{UdtxConfig, ProfileConfig};
use crate::error::TokenCliError;
use crate::keys::KeyManager;

pub async fn plan(
    config: &UdtxConfig,
    _profile: &ProfileConfig,
    _key_manager: &mut KeyManager,
) -> Result<(), TokenCliError> {
    println!("Plan: Preview of Scenario Steps");
    println!("=================================");

    if config.scenario.is_empty() {
        println!("No scenario steps defined in udtx.yaml.");
        println!("Add steps under the `scenario:` key to use plan/apply.");
        return Ok(());
    }

    for (i, step) in config.scenario.iter().enumerate() {
        println!("\n  Step {}: {}", i + 1, step.action);
        for (key, value) in &step.params {
            println!("    {}: {}", key, serde_yaml::to_string(value).unwrap_or_default().trim());
        }
    }

    println!("\n  Total steps: {}", config.scenario.len());
    println!("  Use `udtx apply --yes` to execute these steps.");

    Ok(())
}
