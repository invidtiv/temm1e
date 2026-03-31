use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Unique concern identifier.
pub type ConcernId = String;

/// Temporal context injected into LLM calls.
#[derive(Debug, Clone, Serialize)]
pub struct TemporalContext {
    pub now: DateTime<Utc>,
    pub local_time: String,
    pub uptime: Duration,
    pub idle_duration: Duration,
    pub conscience_state: String,
    pub active_concerns: Vec<ConcernSummary>,
    pub parked_tasks: Vec<ParkedTaskSummary>,
    pub next_event: Option<NextEvent>,
    pub user_active_probability: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConcernSummary {
    pub id: ConcernId,
    pub concern_type: String,
    pub name: String,
    pub source: String,
    pub state: String,
    pub schedule_desc: Option<String>,
    pub last_fired: Option<DateTime<Utc>>,
    pub next_fire: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParkedTaskSummary {
    pub id: ConcernId,
    pub reason: String,
    pub parked_since: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NextEvent {
    pub name: String,
    pub eta: Duration,
}

/// Schedule types for concerns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Schedule {
    /// One-shot at absolute time.
    At(DateTime<Utc>),
    /// Fixed interval (stored as seconds for serde compat).
    #[serde(with = "duration_secs")]
    Every(Duration),
    /// Cron expression (stored as 5-field string, converted to 7-field internally).
    Cron(String),
}

mod duration_secs {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// How a monitor checks its target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitorCheck {
    Web {
        url: String,
        selector: Option<String>,
        extract: ExtractMode,
    },
    Command {
        command: String,
        working_dir: Option<String>,
    },
    File {
        path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractMode {
    FullText,
    Selector,
    JsonPath(String),
}

/// LLM interpretation result for monitor checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interpretation {
    pub relevant: bool,
    pub urgency: Urgency,
    pub notify: bool,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Urgency {
    Low,
    Medium,
    High,
    Critical,
}

/// LLM schedule review result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleReview {
    pub action: String,
    pub new_interval_secs: Option<u64>,
    pub reasoning: String,
    pub user_recommendation: Option<String>,
}

/// Volition decision output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolitionDecision {
    pub create_concerns: Vec<VolitionConcernCreate>,
    pub cancel_concerns: Vec<ConcernId>,
    pub notifications: Vec<VolitionNotification>,
    pub state_recommendation: Option<String>,
    pub internal_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolitionConcernCreate {
    pub concern_type: String,
    pub name: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolitionNotification {
    pub chat_id: String,
    pub message: String,
}

/// Concern configuration variants for creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConcernConfig {
    Alarm {
        name: String,
        fire_at: DateTime<Utc>,
        message: String,
        notify_chat_id: String,
        notify_channel: Option<String>,
    },
    Monitor {
        name: String,
        user_intent: String,
        schedule: Schedule,
        check: MonitorCheck,
        notify_chat_id: String,
        notify_channel: Option<String>,
    },
    Recurring {
        name: String,
        cron_expr: String,
        action_description: String,
        notify_chat_id: String,
        notify_channel: Option<String>,
    },
    Initiative {
        interval_secs: u64,
    },
    SelfWork {
        kind: String,
    },
}

/// Check result from a monitor execution.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub content: String,
    pub content_hash: String,
}

/// Temporal context injection depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectionDepth {
    /// Time + state only (~30 tokens).
    Minimal,
    /// + concerns + next event (~80 tokens).
    Standard,
    /// Everything including parked tasks and user pattern (~120 tokens).
    Full,
}

impl InjectionDepth {
    pub fn parse(s: &str) -> Self {
        match s {
            "minimal" => Self::Minimal,
            "full" => Self::Full,
            _ => Self::Standard,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_serialization_roundtrip() {
        let at = Schedule::At(Utc::now());
        let json = serde_json::to_string(&at).unwrap();
        let _: Schedule = serde_json::from_str(&json).unwrap();

        let every = Schedule::Every(Duration::from_secs(300));
        let json = serde_json::to_string(&every).unwrap();
        let deserialized: Schedule = serde_json::from_str(&json).unwrap();
        match deserialized {
            Schedule::Every(d) => assert_eq!(d.as_secs(), 300),
            _ => panic!("Expected Every variant"),
        }

        let cron = Schedule::Cron("*/5 * * * *".to_string());
        let json = serde_json::to_string(&cron).unwrap();
        let _: Schedule = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn concern_config_serialization() {
        let alarm = ConcernConfig::Alarm {
            name: "test".to_string(),
            fire_at: Utc::now(),
            message: "hello".to_string(),
            notify_chat_id: "123".to_string(),
            notify_channel: None,
        };
        let json = serde_json::to_string(&alarm).unwrap();
        let _: ConcernConfig = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn injection_depth_from_str() {
        assert_eq!(InjectionDepth::parse("minimal"), InjectionDepth::Minimal);
        assert_eq!(InjectionDepth::parse("full"), InjectionDepth::Full);
        assert_eq!(InjectionDepth::parse("standard"), InjectionDepth::Standard);
        assert_eq!(InjectionDepth::parse("unknown"), InjectionDepth::Standard);
    }
}
