use std::sync::Arc;

use temm1e_core::types::error::Temm1eError;

use crate::cognitive::LlmCaller;
use crate::store::Store;
use crate::tracing_ext;
use crate::types::{
    ConcernSummary, TemporalContext, VolitionConcernCreate, VolitionDecision, VolitionNotification,
};

const VOLITION_SYSTEM: &str = "\
You are Tem's initiative system. You think about what Tem should be doing proactively.\n\
\n\
You can: create monitors/alarms/recurring tasks, cancel stale concerns, \
send proactive notifications to the user, write internal notes for your next cycle.\n\
\n\
Rules:\n\
- Only create concerns that genuinely serve the user based on recent context\n\
- Cancel concerns that are no longer useful\n\
- Notify the user only when you have something valuable to share\n\
- Write notes to remember reasoning for next cycle\n\
- You cannot create more initiative concerns\n\
\n\
Respond ONLY in JSON:\n\
{\"create_concerns\":[],\"cancel_concerns\":[],\"notifications\":[],\"state_recommendation\":null,\"internal_notes\":[]}";

pub struct Volition {
    caller: Arc<dyn LlmCaller>,
    store: Arc<Store>,
    max_actions_per_cycle: usize,
}

impl Volition {
    pub fn new(caller: Arc<dyn LlmCaller>, store: Arc<Store>, max_actions: usize) -> Self {
        Self {
            caller,
            store,
            max_actions_per_cycle: max_actions,
        }
    }

    /// Run one initiative cycle: perceive → evaluate → decide → act.
    pub async fn run_cycle(
        &self,
        concerns: &[ConcernSummary],
        temporal_ctx: &TemporalContext,
    ) -> Result<VolitionDecision, Temm1eError> {
        let prev_notes = self.store.get_volition_notes(5).await.unwrap_or_default();
        let prompt = self.build_prompt(concerns, temporal_ctx, &prev_notes);

        let text = self.caller.call(Some(VOLITION_SYSTEM), &prompt).await?;
        let mut decision = parse_volition_decision(&text).unwrap_or_else(|| VolitionDecision {
            create_concerns: vec![],
            cancel_concerns: vec![],
            notifications: vec![],
            state_recommendation: None,
            internal_notes: vec!["Failed to parse initiative response".to_string()],
        });

        // Enforce guardrails
        decision
            .create_concerns
            .truncate(self.max_actions_per_cycle);
        decision
            .cancel_concerns
            .truncate(self.max_actions_per_cycle);

        // Filter out any attempt to create initiative concerns (no self-replication)
        decision
            .create_concerns
            .retain(|c| c.concern_type != "initiative");

        // Persist internal notes
        for note in &decision.internal_notes {
            if let Err(e) = self
                .store
                .save_volition_note(note, "initiative_cycle")
                .await
            {
                tracing::warn!(error = %e, "Failed to save volition note");
            }
        }

        tracing_ext::trace_volition_cycle(
            decision.create_concerns.len()
                + decision.cancel_concerns.len()
                + decision.notifications.len(),
            decision.create_concerns.len(),
            decision.cancel_concerns.len(),
            decision.notifications.len(),
        );

        Ok(decision)
    }

    fn build_prompt(
        &self,
        concerns: &[ConcernSummary],
        temporal_ctx: &TemporalContext,
        prev_notes: &[String],
    ) -> String {
        let temporal = crate::chronos::Chronos::format_injection(
            temporal_ctx,
            crate::types::InjectionDepth::Full,
        );

        let concerns_text = if concerns.is_empty() {
            "No active concerns.".to_string()
        } else {
            concerns
                .iter()
                .map(|c| {
                    let sched = c.schedule_desc.as_deref().unwrap_or("once");
                    format!(
                        "- [{}] {} ({}) source={} {}",
                        c.id, c.name, c.concern_type, c.source, sched
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let notes_text = if prev_notes.is_empty() {
            "No previous notes.".to_string()
        } else {
            prev_notes
                .iter()
                .map(|n| format!("- {n}"))
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            "{temporal}\n\n\
             Active concerns:\n{concerns_text}\n\n\
             Your previous notes:\n{notes_text}\n\n\
             Think about what Tem should be doing proactively right now. \
             Consider: Are any concerns stale? Should new monitors be created based on recent context? \
             Should the user be notified about anything? Any observations to remember?"
        )
    }
}

fn parse_volition_decision(text: &str) -> Option<VolitionDecision> {
    let json_str = crate::cognitive::extract_json_from_text(text)?;
    let v: serde_json::Value = serde_json::from_str(&json_str).ok()?;

    Some(VolitionDecision {
        create_concerns: v
            .get("create_concerns")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        Some(VolitionConcernCreate {
                            concern_type: item.get("concern_type")?.as_str()?.to_string(),
                            name: item.get("name")?.as_str()?.to_string(),
                            config: item
                                .get("config")
                                .cloned()
                                .unwrap_or(serde_json::Value::Null),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
        cancel_concerns: v
            .get("cancel_concerns")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        notifications: v
            .get("notifications")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        Some(VolitionNotification {
                            chat_id: item.get("chat_id")?.as_str()?.to_string(),
                            message: item.get("message")?.as_str()?.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
        state_recommendation: v
            .get("state_recommendation")
            .and_then(|s| s.as_str())
            .map(String::from),
        internal_notes: v
            .get("internal_notes")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_decision() {
        let text = r#"{"create_concerns":[],"cancel_concerns":[],"notifications":[],"state_recommendation":null,"internal_notes":[]}"#;
        let d = parse_volition_decision(text).unwrap();
        assert!(d.create_concerns.is_empty());
        assert!(d.cancel_concerns.is_empty());
        assert!(d.notifications.is_empty());
        assert!(d.state_recommendation.is_none());
        assert!(d.internal_notes.is_empty());
    }

    #[test]
    fn parse_decision_with_actions() {
        let text = r#"{
            "create_concerns": [{"concern_type":"monitor","name":"error-log","config":{}}],
            "cancel_concerns": ["old-monitor-001"],
            "notifications": [{"chat_id":"123","message":"Heads up: error spike detected"}],
            "state_recommendation": "sleep",
            "internal_notes": ["User seems done with the PR review topic"]
        }"#;
        let d = parse_volition_decision(text).unwrap();
        assert_eq!(d.create_concerns.len(), 1);
        assert_eq!(d.create_concerns[0].name, "error-log");
        assert_eq!(d.cancel_concerns, vec!["old-monitor-001"]);
        assert_eq!(d.notifications.len(), 1);
        assert_eq!(d.state_recommendation.as_deref(), Some("sleep"));
        assert_eq!(d.internal_notes.len(), 1);
    }

    #[test]
    fn parse_invalid_returns_none() {
        assert!(parse_volition_decision("not json").is_none());
    }
}
