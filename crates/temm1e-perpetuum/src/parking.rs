use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

/// Why a task was parked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParkReason {
    WaitingForProcess { description: String },
    WaitingForTimer { wake_at: DateTime<Utc> },
    WaitingForExternalResult { description: String },
}

impl ParkReason {
    pub fn description(&self) -> String {
        match self {
            Self::WaitingForProcess { description } => format!("process: {description}"),
            Self::WaitingForTimer { wake_at } => format!("timer: until {wake_at}"),
            Self::WaitingForExternalResult { description } => format!("external: {description}"),
        }
    }
}

/// Signal sent when a parked task should resume.
#[derive(Debug)]
pub struct ResumeSignal {
    pub result: Option<String>,
}

/// A parked task handle — holds the resume sender.
pub struct ParkedTask {
    pub id: String,
    pub reason: ParkReason,
    pub parked_at: DateTime<Utc>,
    pub resume_tx: Option<oneshot::Sender<ResumeSignal>>,
}

impl ParkedTask {
    /// Create a new parked task, returning the task and a receiver for the resume signal.
    pub fn new(id: String, reason: ParkReason) -> (Self, oneshot::Receiver<ResumeSignal>) {
        let (tx, rx) = oneshot::channel();
        let task = Self {
            id,
            reason,
            parked_at: Utc::now(),
            resume_tx: Some(tx),
        };
        (task, rx)
    }

    /// Resume the parked task by sending a signal.
    pub fn resume(mut self, result: Option<String>) -> bool {
        if let Some(tx) = self.resume_tx.take() {
            tx.send(ResumeSignal { result }).is_ok()
        } else {
            false
        }
    }

    /// Check if the parked task is still waiting (sender not dropped).
    pub fn is_waiting(&self) -> bool {
        self.resume_tx.is_some()
    }
}

/// Timer-based parking: park until a specific time, then auto-resume.
pub async fn park_until(wake_at: DateTime<Utc>) -> ResumeSignal {
    let duration = (wake_at - Utc::now())
        .to_std()
        .unwrap_or(std::time::Duration::ZERO);
    tokio::time::sleep(duration).await;
    ResumeSignal {
        result: Some("timer expired".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn park_reason_description() {
        let r = ParkReason::WaitingForProcess {
            description: "deploy".into(),
        };
        assert!(r.description().contains("deploy"));
    }

    #[tokio::test]
    async fn parked_task_resume() {
        let (task, rx) = ParkedTask::new(
            "t-001".into(),
            ParkReason::WaitingForExternalResult {
                description: "API call".into(),
            },
        );

        assert!(task.is_waiting());
        assert!(task.resume(Some("done".into())));

        let signal = rx.await.unwrap();
        assert_eq!(signal.result.unwrap(), "done");
    }

    #[tokio::test]
    async fn park_until_immediate() {
        let past = Utc::now() - chrono::Duration::seconds(1);
        let signal = park_until(past).await;
        assert!(signal.result.is_some());
    }
}
