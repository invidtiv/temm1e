//! Dev-only TUI wiring smoke test.
//!
//! Calls the exact same `spawn_agent` function that `launch_tui` calls,
//! but skips ratatui's terminal init so it runs headless. Used to
//! empirically verify the TUI parity gate per docs/RELEASE_PROTOCOL.md §7
//! without needing a real TTY.
//!
//! Usage:
//!   cargo run --example tui_smoke --features tui --release
//!
//! Expected stdout: registration logs for every feature wired into TUI.
//! The harness exits after 5 s so async init (Hive, Perpetuum, etc.) has
//! time to complete and emit its logs.

use std::time::Duration;

use temm1e_core::config::credentials;
use temm1e_core::types::config::Temm1eConfig;
use temm1e_tui::agent_bridge::{spawn_agent, AgentSetup};
use tokio::sync::mpsc;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Logs to stdout with info-level — same destination the parity grep
    // protocol expects to find registration anchors.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(true)
        .init();

    eprintln!("=== TUI smoke: spawn_agent() direct-call (no ratatui) ===");

    // Load the same config TUI would use at launch.
    let config_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".temm1e");
    let config_path = config_dir.join("config.toml");
    let config_str = std::fs::read_to_string(&config_path).unwrap_or_default();
    let config: Temm1eConfig = if config_str.is_empty() {
        Temm1eConfig::default()
    } else {
        toml::from_str(&config_str).unwrap_or_default()
    };

    // Resolve credentials from saved creds (mirrors TUI onboarding's fallback).
    let (provider_name, api_key, model) = match credentials::load_saved_credentials() {
        Some(t) => t,
        None => {
            eprintln!("[SMOKE FAIL] No saved credentials at ~/.temm1e/credentials.toml");
            std::process::exit(2);
        }
    };

    let setup = AgentSetup {
        provider_name: provider_name.clone(),
        api_key,
        model: model.clone(),
        base_url: None,
        config,
        mode: None,
    };

    // AgentHandle needs a place to send status events. We discard them —
    // this smoke only cares about startup logs.
    let (tx, _rx) = mpsc::unbounded_channel();

    eprintln!(
        "[SMOKE] calling spawn_agent(provider={}, model={})",
        provider_name, model
    );
    match spawn_agent(setup, tx).await {
        Ok(_handle) => {
            eprintln!("[SMOKE] spawn_agent returned Ok — handle produced");
        }
        Err(e) => {
            eprintln!("[SMOKE FAIL] spawn_agent returned Err: {}", e);
            std::process::exit(3);
        }
    }

    // Give async init (Hive SQLite, Perpetuum startup, etc.) 5 seconds
    // to complete and emit their logs before we exit.
    eprintln!("[SMOKE] waiting 5s for async init logs...");
    tokio::time::sleep(Duration::from_secs(5)).await;
    eprintln!("[SMOKE] DONE — grep stdout for registration anchors");
    Ok(())
}
