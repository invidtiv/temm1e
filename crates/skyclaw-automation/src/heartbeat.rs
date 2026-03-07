//! HeartbeatRunner — periodically reads a task checklist and sends
//! synthetic messages to the agent for processing.
//!
//! Protocol (OpenClaw-compatible):
//!   1. Every `interval`, read `HEARTBEAT.md` from the workspace.
//!   2. If the file is missing or empty, skip (no work).
//!   3. If `HEARTBEAT_OK` exists in the workspace, delete it and skip
//!      that cycle (agent signalled "nothing to do, save tokens").
//!   4. Otherwise, send the checklist content as a synthetic inbound
//!      message through the unified message channel.
//!   5. If the channel is full (previous heartbeat still processing),
//!      skip — never pile up heartbeat ticks.

use std::path::PathBuf;

use chrono::Timelike;
use skyclaw_core::types::config::HeartbeatConfig;
use skyclaw_core::types::message::InboundMessage;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::duration::parse_duration;

/// Parse an "HH:MM-HH:MM" active hours window into (start_hour, start_min, end_hour, end_min).
fn parse_active_hours(s: &str) -> Option<(u32, u32, u32, u32)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return None;
    }
    let start: Vec<&str> = parts[0].trim().split(':').collect();
    let end: Vec<&str> = parts[1].trim().split(':').collect();
    if start.len() != 2 || end.len() != 2 {
        return None;
    }
    Some((
        start[0].parse().ok()?,
        start[1].parse().ok()?,
        end[0].parse().ok()?,
        end[1].parse().ok()?,
    ))
}

/// Check if the current local time is within the active hours window.
fn is_within_active_hours(active_hours: &str) -> bool {
    let (sh, sm, eh, em) = match parse_active_hours(active_hours) {
        Some(v) => v,
        None => {
            warn!(window = %active_hours, "Invalid active_hours format, ignoring");
            return true; // Invalid format → don't block
        }
    };

    let now = chrono::Local::now();
    let current = now.hour() * 60 + now.minute();
    let start = sh * 60 + sm;
    let end = eh * 60 + em;

    if start <= end {
        // Normal window: e.g. 08:00-22:00
        current >= start && current < end
    } else {
        // Overnight window: e.g. 22:00-06:00
        current >= start || current < end
    }
}

/// Heartbeat runner that produces synthetic agent messages on a timer.
pub struct HeartbeatRunner {
    config: HeartbeatConfig,
    workspace_path: PathBuf,
    /// Chat ID to attribute heartbeat messages to (for response routing).
    chat_id: String,
}

impl HeartbeatRunner {
    pub fn new(config: HeartbeatConfig, workspace_path: PathBuf, chat_id: String) -> Self {
        Self {
            config,
            workspace_path,
            chat_id,
        }
    }

    /// Start the heartbeat loop. Messages are sent to `tx`.
    ///
    /// Uses `try_send` — if the channel buffer is full (previous heartbeat
    /// still being processed), the tick is silently skipped.
    ///
    /// This method runs forever; spawn it as a tokio task.
    pub async fn run(self, tx: mpsc::Sender<InboundMessage>) {
        let interval = match parse_duration(&self.config.interval) {
            Ok(d) => d,
            Err(e) => {
                warn!(error = %e, interval = %self.config.interval, "Invalid heartbeat interval, defaulting to 30m");
                std::time::Duration::from_secs(30 * 60)
            }
        };

        info!(
            interval_secs = interval.as_secs(),
            checklist = %self.config.checklist,
            "Heartbeat started"
        );

        let mut timer = tokio::time::interval(interval);
        // Skip the first immediate tick — let the system warm up
        timer.tick().await;

        loop {
            timer.tick().await;

            // 0. Active hours check (ZeroClaw pattern — save tokens at night)
            if let Some(ref window) = self.config.active_hours {
                if !is_within_active_hours(window) {
                    debug!(window = %window, "Outside active hours — skipping heartbeat");
                    continue;
                }
            }

            // 1. HEARTBEAT_OK suppression
            let ok_path = self.workspace_path.join("HEARTBEAT_OK");
            if ok_path.exists() {
                match tokio::fs::remove_file(&ok_path).await {
                    Ok(()) => {
                        info!("Heartbeat suppressed by HEARTBEAT_OK — skipping cycle");
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to remove HEARTBEAT_OK");
                    }
                }
                continue;
            }

            // 2. Read the checklist
            let checklist_path = self.workspace_path.join(&self.config.checklist);
            let checklist = match tokio::fs::read_to_string(&checklist_path).await {
                Ok(content) if !content.trim().is_empty() => content,
                Ok(_) => {
                    debug!("Heartbeat checklist is empty — skipping");
                    continue;
                }
                Err(_) => {
                    debug!(path = %checklist_path.display(), "No heartbeat checklist found — skipping");
                    continue;
                }
            };

            // 3. Build synthetic inbound message
            let now = chrono::Utc::now();
            let msg = InboundMessage {
                id: format!("heartbeat-{}", now.timestamp()),
                channel: "heartbeat".to_string(),
                chat_id: self.chat_id.clone(),
                user_id: "system".to_string(),
                username: None,
                text: Some(format!(
                    "HEARTBEAT — You are running autonomously. \
                     Review your task checklist below and take action on any pending items. \
                     Use tools to execute tasks. When all tasks are done or you need to wait, \
                     write 'HEARTBEAT_OK' to the file HEARTBEAT_OK in your workspace to skip \
                     the next heartbeat cycle.\n\n\
                     ---\n\n\
                     {}\n\n\
                     ---\n\n\
                     Instructions:\n\
                     - Execute the next pending task (marked with `- [ ]`)\n\
                     - Mark completed tasks with `- [x]` by rewriting the checklist\n\
                     - If all tasks are done, write HEARTBEAT_OK to pause\n\
                     - If a task fails, note the error and move to the next one\n\
                     - Be concise in responses — this is autonomous execution",
                    checklist
                )),
                attachments: Vec::new(),
                reply_to: None,
                timestamp: now,
            };

            // 4. Send — skip if channel full (previous heartbeat still processing)
            match tx.try_send(msg) {
                Ok(()) => {
                    info!("Heartbeat tick sent to agent");
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    debug!("Heartbeat skipped — agent still processing previous tick");
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    warn!("Heartbeat channel closed — stopping");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> HeartbeatConfig {
        HeartbeatConfig {
            enabled: true,
            interval: "1s".to_string(),
            checklist: "HEARTBEAT.md".to_string(),
            report_to: None,
            active_hours: None,
        }
    }

    #[test]
    fn parse_active_hours_normal() {
        assert_eq!(parse_active_hours("08:00-22:00"), Some((8, 0, 22, 0)));
        assert_eq!(parse_active_hours("09:30-17:45"), Some((9, 30, 17, 45)));
    }

    #[test]
    fn parse_active_hours_overnight() {
        assert_eq!(parse_active_hours("22:00-06:00"), Some((22, 0, 6, 0)));
    }

    #[test]
    fn parse_active_hours_invalid() {
        assert_eq!(parse_active_hours("invalid"), None);
        assert_eq!(parse_active_hours("08:00"), None);
        assert_eq!(parse_active_hours(""), None);
    }

    #[test]
    fn active_hours_always_true_for_bad_format() {
        // Invalid format should not block heartbeats
        assert!(is_within_active_hours("garbage"));
    }

    #[tokio::test]
    async fn heartbeat_skips_when_no_checklist() {
        let dir = tempfile::tempdir().unwrap();
        let runner = HeartbeatRunner::new(
            test_config(),
            dir.path().to_path_buf(),
            "test-chat".to_string(),
        );

        let (tx, mut rx) = mpsc::channel(1);

        // Run heartbeat in background, give it 2 seconds
        let handle = tokio::spawn(runner.run(tx));

        // Wait a bit — should NOT receive anything (no checklist file)
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(2500),
            rx.recv(),
        )
        .await;

        assert!(result.is_err(), "Should timeout — no messages without checklist");
        handle.abort();
    }

    #[tokio::test]
    async fn heartbeat_sends_when_checklist_exists() {
        let dir = tempfile::tempdir().unwrap();
        let checklist_path = dir.path().join("HEARTBEAT.md");
        std::fs::write(&checklist_path, "- [ ] Do something\n- [ ] Do another thing").unwrap();

        let runner = HeartbeatRunner::new(
            test_config(),
            dir.path().to_path_buf(),
            "test-chat".to_string(),
        );

        let (tx, mut rx) = mpsc::channel(2);
        let handle = tokio::spawn(runner.run(tx));

        // Should receive a heartbeat message
        let msg = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            rx.recv(),
        )
        .await
        .expect("Should receive heartbeat")
        .expect("Channel should not close");

        assert_eq!(msg.channel, "heartbeat");
        assert!(msg.text.unwrap().contains("Do something"));
        handle.abort();
    }

    #[tokio::test]
    async fn heartbeat_ok_suppression() {
        let dir = tempfile::tempdir().unwrap();
        let checklist_path = dir.path().join("HEARTBEAT.md");
        std::fs::write(&checklist_path, "- [ ] Task").unwrap();

        // Write HEARTBEAT_OK to suppress
        let ok_path = dir.path().join("HEARTBEAT_OK");
        std::fs::write(&ok_path, "ok").unwrap();

        let runner = HeartbeatRunner::new(
            test_config(),
            dir.path().to_path_buf(),
            "test-chat".to_string(),
        );

        let (tx, mut rx) = mpsc::channel(2);
        let handle = tokio::spawn(runner.run(tx));

        // First tick should be suppressed, HEARTBEAT_OK should be deleted
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        assert!(!ok_path.exists(), "HEARTBEAT_OK should be deleted");

        // Second tick should send (HEARTBEAT_OK is gone)
        let msg = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            rx.recv(),
        )
        .await
        .expect("Should receive heartbeat after HEARTBEAT_OK cleared")
        .expect("Channel should not close");

        assert_eq!(msg.channel, "heartbeat");
        handle.abort();
    }
}
