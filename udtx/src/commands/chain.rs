use crate::error::TokenCliError;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

const OFFCKB_RPC_URL: &str = "http://127.0.0.1:8114";

fn find_offckb() -> Result<String, TokenCliError> {
    match Command::new("which").arg("offckb").output() {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path.is_empty() {
                return Err(TokenCliError::Config(
                    crate::config::ConfigError::Validation(
                        "offckb not found in PATH. Install with: npm install -g @offckb/cli".into()
                    )
                ));
            }
            Ok(path)
        }
        _ => Err(TokenCliError::Config(
            crate::config::ConfigError::Validation(
                "offckb not found in PATH. Install with: npm install -g @offckb/cli".into()
            )
        )),
    }
}

fn check_offckb() -> Result<String, TokenCliError> {
    let offckb = find_offckb()?;
    let output = Command::new(&offckb)
        .arg("--version")
        .output()
        .map_err(|e| TokenCliError::Config(
            crate::config::ConfigError::Validation(format!("Failed to run offckb: {}", e))
        ))?;
    
    if !output.status.success() {
        return Err(TokenCliError::Config(
            crate::config::ConfigError::Validation("offckb --version failed".into())
        ));
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub async fn chain_up(background: bool) -> Result<(), TokenCliError> {
    let version = check_offckb()?;
    println!("Using {}", version);
    
    if is_devnet_running().await {
        println!("Devnet is already running at {}", OFFCKB_RPC_URL);
        return Ok(());
    }
    
    println!("Starting offckb devnet...");
    
    let mut cmd = Command::new("offckb");
    cmd.arg("node");
    
    if background {
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
    }
    
    let mut child = cmd.spawn().map_err(|e| TokenCliError::Config(
        crate::config::ConfigError::Validation(format!("Failed to start offckb: {}", e))
    ))?;
    
    if !background {
        let status = child.wait().map_err(|e| TokenCliError::Config(
            crate::config::ConfigError::Validation(format!("offckb exited unexpectedly: {}", e))
        ))?;
        if !status.success() {
            return Err(TokenCliError::Config(
                crate::config::ConfigError::Validation("offckb node failed".into())
            ));
        }
    } else {
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        match timeout(Duration::from_secs(10), wait_for_rpc()).await {
            Ok(Ok(())) => {
                println!("Devnet started successfully!");
                println!("RPC URL: {}", OFFCKB_RPC_URL);
            }
            _ => {
                println!("Warning: devnet may not have started yet. Check status with `udtx chain status`");
            }
        }
    }
    
    Ok(())
}

pub async fn chain_down() -> Result<(), TokenCliError> {
    let _ = check_offckb()?;
    
    println!("Stopping offckb devnet...");
    
    let output = Command::new("pkill")
        .args(&["-f", "offckb node"])
        .output();
    
    match output {
        Ok(output) => {
            if output.status.success() {
                println!("Devnet stopped.");
            } else {
                println!("No running devnet found.");
            }
        }
        Err(e) => {
            return Err(TokenCliError::Config(
                crate::config::ConfigError::Validation(format!("Failed to stop devnet: {}", e))
            ));
        }
    }
    
    Ok(())
}

pub async fn chain_reset(yes: bool) -> Result<(), TokenCliError> {
    if !yes {
        println!("This will reset all devnet data. Use --yes to confirm.");
        return Ok(());
    }
    
    let _ = check_offckb()?;
    
    println!("Resetting devnet data...");
    
    let _ = chain_down().await;
    
    let output = Command::new("offckb")
        .arg("clean")
        .output()
        .map_err(|e| TokenCliError::Config(
            crate::config::ConfigError::Validation(format!("Failed to run offckb clean: {}", e))
        ))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TokenCliError::Config(
            crate::config::ConfigError::Validation(format!("offckb clean failed: {}", stderr))
        ));
    }
    
    println!("Devnet data reset. Run `udtx chain up` to start fresh.");
    
    Ok(())
}

pub async fn chain_status() -> Result<(), TokenCliError> {
    if is_devnet_running().await {
        println!("Devnet is running at {}", OFFCKB_RPC_URL);
        
        match get_chain_info().await {
            Ok(info) => println!("Chain info: {}", info),
            Err(_) => println!("(RPC not responding yet)"),
        }
    } else {
        println!("Devnet is not running.");
        println!("Run `udtx chain up` to start it.");
    }
    
    Ok(())
}

async fn is_devnet_running() -> bool {
    get_chain_info().await.is_ok()
}

async fn get_chain_info() -> Result<String, TokenCliError> {
    let output = Command::new("curl")
        .args(&[
            "-s", "-X", "POST",
            "-H", "Content-Type: application/json",
            "-d", r#"{"id":1,"jsonrpc":"2.0","method":"get_tip_block_number","params":[]}"#,
            OFFCKB_RPC_URL,
        ])
        .output();
    
    match output {
        Ok(output) if output.status.success() => {
            let response = String::from_utf8_lossy(&output.stdout);
            Ok(response.to_string())
        }
        _ => Err(TokenCliError::Rpc {
            message: "RPC not available".into(),
        }),
    }
}

async fn wait_for_rpc() -> Result<(), TokenCliError> {
    for _ in 0..30 {
        if get_chain_info().await.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Err(TokenCliError::Rpc {
        message: "Timed out waiting for RPC".into(),
    })
}
