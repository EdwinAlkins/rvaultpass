//! Headless vault service. It serves one `.vault` file over HTTP.

use std::path::PathBuf;

use rvault_server::{serve, ServerConfig, DEFAULT_IDLE_SECS};
use tokio::sync::oneshot;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let config = match parse_args() {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{}", message);
            std::process::exit(2);
        }
    };
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let _ = shutdown_tx.send(());
    });
    if let Err(error) = serve(config, shutdown_rx, None).await {
        eprintln!("Vault server failed: {}", error);
        std::process::exit(1);
    }
}

fn parse_args() -> Result<ServerConfig, String> {
    let mut bind = "127.0.0.1:8443".to_string();
    let mut vault_path = None;
    let mut cors_origins = Vec::new();
    let mut idle_lock_secs = DEFAULT_IDLE_SECS;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bind" => bind = required(&mut args, "--bind")?,
            "--vault" => vault_path = Some(PathBuf::from(required(&mut args, "--vault")?)),
            "--cors-origin" => cors_origins.push(required(&mut args, "--cors-origin")?),
            "--idle-lock" => {
                let value = required(&mut args, "--idle-lock")?;
                idle_lock_secs = value
                    .parse()
                    .map_err(|_| format!("--idle-lock expects a number of seconds, got {}", value))?;
            }
            "--help" | "-h" => return Err(help()),
            other => return Err(format!("Unknown argument {}\n{}", other, help())),
        }
    }
    let vault_path = vault_path.ok_or_else(|| format!("--vault is required\n{}", help()))?;
    if idle_lock_secs < 1 {
        return Err("--idle-lock must be at least 1 second".to_string());
    }
    Ok(ServerConfig {
        bind,
        vault_path,
        cors_origins,
        idle_lock_secs,
    })
}

fn required(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("{} requires a value", flag))
}

fn help() -> String {
    "rvault-server --vault FILE [--bind ADDR] [--cors-origin URL] [--idle-lock SECONDS]\n\n\
     Serves an unlocked .vault file over HTTP. The master password is sent only to\n\
     POST /v1/vault/unlock. Terminate TLS in a reverse proxy."
        .to_string()
}
