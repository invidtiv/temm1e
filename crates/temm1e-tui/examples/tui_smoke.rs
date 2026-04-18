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
use temm1e_core::types::message::InboundMessage;
use temm1e_tui::agent_bridge::{spawn_agent, AgentSetup};
use temm1e_tui::event::Event;
use tokio::sync::mpsc;

/// tui_smoke exit codes:
///   0  — all checks passed
///   2  — missing saved credentials
///   3  — spawn_agent returned Err
///   4  — agent did not respond within the response timeout
///   5  — agent response was empty
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

    // Event channel — spawn_agent pushes AgentResponseEvent via this.
    // We read them here to drive the exhaustive end-to-end test.
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Event>();

    eprintln!(
        "[SMOKE] calling spawn_agent(provider={}, model={})",
        provider_name, model
    );
    let handle = match spawn_agent(setup, event_tx).await {
        Ok(h) => {
            eprintln!("[SMOKE] spawn_agent returned Ok — handle produced");
            h
        }
        Err(e) => {
            eprintln!("[SMOKE FAIL] spawn_agent returned Err: {}", e);
            std::process::exit(3);
        }
    };

    // Give async init (Hive SQLite, Perpetuum startup, Eigen-Tune DB
    // migration, etc.) 3 seconds to complete and emit their logs before
    // we drive the first message.
    eprintln!("[SMOKE] waiting 3s for async init logs to drain...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // ── EXHAUSTIVE TEST: drive a real user message through the agent ──
    // This exercises: classifier → agent loop → provider call → response
    // → event_tx back to the consumer. If ANY subsystem wired into the
    // agent is broken, the response will timeout or error.
    eprintln!("[SMOKE] sending test message: 'what can you do in 1 sentence?'");
    let msg = InboundMessage {
        id: uuid::Uuid::new_v4().to_string(),
        chat_id: "tui-smoke".into(),
        user_id: "smoke-test".into(),
        username: Some("smoke".into()),
        channel: "tui-smoke".into(),
        text: Some("what can you do in 1 sentence?".into()),
        attachments: vec![],
        reply_to: None,
        timestamp: chrono::Utc::now(),
    };
    handle
        .inbound_tx
        .send(msg)
        .await
        .map_err(|e| format!("inbound send failed: {e}"))?;

    // Wait up to 90s for a response event.
    let timeout = Duration::from_secs(90);
    let deadline = tokio::time::Instant::now() + timeout;
    let mut got_response = false;
    let mut response_text = String::new();
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, event_rx.recv()).await {
            Ok(Some(Event::AgentResponse(resp))) => {
                got_response = true;
                response_text = resp.message.text;
                eprintln!(
                    "[SMOKE] usage: in={} out={} cost=${:.4}",
                    resp.input_tokens, resp.output_tokens, resp.cost_usd
                );
                break;
            }
            Ok(Some(_other)) => {
                // Non-response events (status updates, tool notifications).
                continue;
            }
            Ok(None) => {
                eprintln!("[SMOKE FAIL] event channel closed before Complete event");
                std::process::exit(4);
            }
            Err(_) => {
                eprintln!("[SMOKE FAIL] timeout waiting for agent response (90s)");
                std::process::exit(4);
            }
        }
    }

    if !got_response {
        eprintln!("[SMOKE FAIL] no response received within timeout");
        std::process::exit(4);
    }

    if response_text.trim().is_empty() {
        eprintln!("[SMOKE FAIL] agent response was empty");
        std::process::exit(5);
    }

    eprintln!("[SMOKE] AGENT RESPONDED: {} chars", response_text.len());
    eprintln!(
        "[SMOKE] first 200 chars: {}",
        &response_text.chars().take(200).collect::<String>()
    );

    eprintln!("[SMOKE] DONE — all checks passed");
    Ok(())
}
