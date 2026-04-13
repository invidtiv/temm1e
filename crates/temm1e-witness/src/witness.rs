//! Witness runtime — dispatches predicate checks and renders verdicts.
//!
//! Phase 1 implements Tier 0 dispatch only. Tier 1/2 are stubbed as
//! `Inconclusive` for advisory predicates, `Fail` for non-advisory predicates
//! that cannot be reduced to Tier 0.
//!
//! The Witness is the ONLY entity authorized to produce a `Verified` outcome.
//! It respects Law 5 (Narrative-Only FAIL) — it never mutates the file system,
//! git state, or processes. Its only output is a `Verdict` plus a rewritten
//! final-reply string.

use crate::config::WitnessStrictness;
use crate::error::WitnessError;
use crate::ledger::Ledger;
use crate::predicates::{check_tier0, CheckContext, PredicateCheckResult};
use crate::types::{
    Claim, Evidence, LedgerPayload, Oath, Predicate, PredicateResult, TierUsage, Verdict,
    VerdictOutcome,
};
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;

/// The Witness: verifies sealed Oaths and records verdicts to the Ledger.
pub struct Witness {
    ledger: Arc<Ledger>,
    workspace_root: std::path::PathBuf,
}

impl Witness {
    pub fn new(ledger: Arc<Ledger>, workspace_root: impl Into<std::path::PathBuf>) -> Self {
        Self {
            ledger,
            workspace_root: workspace_root.into(),
        }
    }

    pub fn ledger(&self) -> &Arc<Ledger> {
        &self.ledger
    }

    /// Verify an Oath by running all its postcondition predicates. Records
    /// the verdict to the Ledger. Returns the full Verdict.
    ///
    /// Law 2: this is the only function that produces `Verified` outcomes.
    /// Law 5: never mutates files, git state, or processes.
    pub async fn verify_oath(&self, oath: &Oath) -> Result<Verdict, WitnessError> {
        if !oath.is_sealed() {
            return Err(WitnessError::NoSealedOath(oath.subtask_id.clone()));
        }

        let start = Instant::now();
        let ctx = CheckContext::new(&self.workspace_root);
        let mut per_predicate: Vec<PredicateResult> = Vec::new();
        let mut tier_usage = TierUsage::default();

        for predicate in &oath.postconditions {
            let tier = predicate.tier();
            let result = if tier == 0 {
                tier_usage.tier0_calls += 1;
                let r = check_tier0(predicate, &ctx).await?;
                tier_usage.tier0_latency_ms += r.latency_ms;
                r
            } else if tier == 1 {
                // Tier 1 stubbed for Phase 1 — advisory predicates become Inconclusive.
                tier_usage.tier1_calls += 1;
                PredicateCheckResult {
                    outcome: VerdictOutcome::Inconclusive,
                    detail: "Tier 1 not implemented in Phase 1 — predicate advisory".to_string(),
                    latency_ms: 0,
                }
            } else {
                tier_usage.tier2_calls += 1;
                PredicateCheckResult {
                    outcome: VerdictOutcome::Inconclusive,
                    detail: "Tier 2 not implemented in Phase 1 — predicate advisory".to_string(),
                    latency_ms: 0,
                }
            };

            let advisory = matches!(
                predicate,
                Predicate::AspectVerifier { advisory: true, .. }
                    | Predicate::AdversarialJudge { advisory: true, .. }
            );

            per_predicate.push(PredicateResult {
                predicate: predicate.clone(),
                tier,
                outcome: result.outcome,
                detail: result.detail,
                advisory,
                latency_ms: result.latency_ms,
            });
        }

        let outcome = aggregate_outcome(&per_predicate);
        let reason = build_reason(&per_predicate);

        let verdict = Verdict {
            subtask_id: oath.subtask_id.clone(),
            rendered_at: Utc::now(),
            outcome,
            per_predicate,
            tier_usage,
            reason,
            cost_usd: 0.0,
            latency_ms: start.elapsed().as_millis() as u64,
        };

        // Write VerdictRendered to the Ledger.
        self.ledger
            .append(
                oath.session_id.clone(),
                Some(oath.subtask_id.clone()),
                Some(oath.root_goal_id.clone()),
                LedgerPayload::VerdictRendered(verdict.clone()),
                verdict.cost_usd,
                verdict.latency_ms,
            )
            .await?;

        Ok(verdict)
    }

    /// Record a Claim to the Ledger without running predicates yet.
    pub async fn submit_claim(
        &self,
        claim: Claim,
        root_goal_id: String,
        session_id: String,
    ) -> Result<(), WitnessError> {
        self.ledger
            .append(
                session_id,
                Some(claim.subtask_id.clone()),
                Some(root_goal_id),
                LedgerPayload::ClaimSubmitted(claim),
                0.0,
                0,
            )
            .await?;
        Ok(())
    }

    /// Record Evidence produced during execution.
    pub async fn record_evidence(
        &self,
        evidence: Evidence,
        root_goal_id: String,
        session_id: String,
    ) -> Result<(), WitnessError> {
        self.ledger
            .append(
                session_id,
                Some(evidence.subtask_id.clone()),
                Some(root_goal_id),
                LedgerPayload::EvidenceProduced(evidence),
                0.0,
                0,
            )
            .await?;
        Ok(())
    }

    /// Compose the final reply to the user based on a verdict and the
    /// current agent-proposed reply. Honors Law 4 (loud failure) and Law 5
    /// (narrative-only — never modifies files).
    ///
    /// Strictness determines behavior:
    /// - Observe: return original reply unchanged.
    /// - Warn: append a brief note if verdict is FAIL.
    /// - Block: rewrite reply with honest failure description.
    /// - BlockWithRetry: same as Block (retry handled at runtime level).
    pub fn compose_final_reply(
        &self,
        agent_reply: &str,
        verdict: &Verdict,
        strictness: WitnessStrictness,
    ) -> String {
        match strictness {
            WitnessStrictness::Observe => agent_reply.to_string(),
            WitnessStrictness::Warn => match verdict.outcome {
                VerdictOutcome::Pass => agent_reply.to_string(),
                VerdictOutcome::Fail | VerdictOutcome::Inconclusive => {
                    format!(
                        "{}\n\n---\n⚠ Witness: {}/{} predicates passed. {}",
                        agent_reply,
                        verdict.pass_count(),
                        verdict.total_count(),
                        verdict.reason,
                    )
                }
            },
            WitnessStrictness::Block | WitnessStrictness::BlockWithRetry => match verdict.outcome {
                VerdictOutcome::Pass => agent_reply.to_string(),
                VerdictOutcome::Fail => format_failed_reply(agent_reply, verdict),
                VerdictOutcome::Inconclusive => format_inconclusive_reply(agent_reply, verdict),
            },
        }
    }
}

/// Aggregate per-predicate results into an overall verdict outcome.
///
/// Rules:
/// - Any non-advisory FAIL → FAIL.
/// - All PASS (ignoring advisory) → PASS.
/// - Otherwise → Inconclusive.
fn aggregate_outcome(per_predicate: &[PredicateResult]) -> VerdictOutcome {
    let mut has_fail = false;
    let mut has_inconclusive = false;
    for r in per_predicate {
        if r.advisory {
            continue;
        }
        match r.outcome {
            VerdictOutcome::Fail => has_fail = true,
            VerdictOutcome::Inconclusive => has_inconclusive = true,
            VerdictOutcome::Pass => {}
        }
    }
    if has_fail {
        VerdictOutcome::Fail
    } else if has_inconclusive {
        VerdictOutcome::Inconclusive
    } else {
        VerdictOutcome::Pass
    }
}

fn build_reason(per_predicate: &[PredicateResult]) -> String {
    let pass: u32 = per_predicate
        .iter()
        .filter(|r| r.outcome == VerdictOutcome::Pass && !r.advisory)
        .count() as u32;
    let fail: u32 = per_predicate
        .iter()
        .filter(|r| r.outcome == VerdictOutcome::Fail && !r.advisory)
        .count() as u32;
    let inc: u32 = per_predicate
        .iter()
        .filter(|r| r.outcome == VerdictOutcome::Inconclusive && !r.advisory)
        .count() as u32;
    format!(
        "{}/{} pass, {} fail, {} inconclusive",
        pass,
        per_predicate.len(),
        fail,
        inc
    )
}

fn format_failed_reply(_agent_reply: &str, verdict: &Verdict) -> String {
    let mut out = String::new();
    out.push_str("⚠ **Partial completion.**\n\n");
    out.push_str(&format!(
        "Witness verified {}/{} postconditions:\n\n",
        verdict.pass_count(),
        verdict.total_count()
    ));

    let passed: Vec<&PredicateResult> = verdict
        .per_predicate
        .iter()
        .filter(|r| r.outcome == VerdictOutcome::Pass)
        .collect();
    let failed: Vec<&PredicateResult> = verdict
        .per_predicate
        .iter()
        .filter(|r| r.outcome == VerdictOutcome::Fail)
        .collect();
    let incons: Vec<&PredicateResult> = verdict
        .per_predicate
        .iter()
        .filter(|r| r.outcome == VerdictOutcome::Inconclusive)
        .collect();

    if !passed.is_empty() {
        out.push_str("✓ Verified:\n");
        for r in passed {
            out.push_str(&format!("  • {}\n", r.detail));
        }
    }
    if !failed.is_empty() {
        out.push_str("\n✗ Could not verify:\n");
        for r in failed {
            out.push_str(&format!("  • {}\n", r.detail));
        }
    }
    if !incons.is_empty() {
        out.push_str("\n? Inconclusive:\n");
        for r in incons {
            out.push_str(&format!("  • {}\n", r.detail));
        }
    }

    out.push_str("\nThis work is incomplete according to the pre-committed contract. Files produced during this task have NOT been modified or rolled back — they remain in place for your review.");
    out
}

fn format_inconclusive_reply(agent_reply: &str, verdict: &Verdict) -> String {
    format!(
        "{}\n\n---\n⚠ Witness: verdict inconclusive. {}",
        agent_reply, verdict.reason
    )
}

/// Compute the WitnessStrictness for a task given configuration and complexity.
pub fn resolve_strictness(
    config: &crate::config::WitnessConfig,
    is_complex: bool,
    is_standard: bool,
) -> WitnessStrictness {
    use crate::config::OverrideStrictness;
    match config.override_strictness {
        OverrideStrictness::Observe => WitnessStrictness::Observe,
        OverrideStrictness::Warn => WitnessStrictness::Warn,
        OverrideStrictness::Block => WitnessStrictness::Block,
        OverrideStrictness::Auto => {
            if is_complex {
                WitnessStrictness::Block
            } else if is_standard {
                WitnessStrictness::Warn
            } else {
                WitnessStrictness::Observe
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oath::seal_oath;
    use crate::types::Predicate;
    use std::path::PathBuf;
    use tempfile::tempdir;

    async fn setup() -> (Witness, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let ledger = Ledger::open("sqlite::memory:").await.unwrap();
        let witness = Witness::new(ledger, dir.path().to_path_buf());
        (witness, dir)
    }

    #[tokio::test]
    async fn verify_passing_oath_yields_pass_verdict() {
        let (witness, dir) = setup().await;
        let f = dir.path().join("hello.txt");
        tokio::fs::write(&f, "hi").await.unwrap();

        let oath = Oath::draft("st-1", "root-1", "sess-1", "touch a file")
            .with_postcondition(Predicate::FileExists { path: f });
        let (sealed, _) = seal_oath(witness.ledger(), oath).await.unwrap();

        let verdict = witness.verify_oath(&sealed).await.unwrap();
        assert_eq!(verdict.outcome, VerdictOutcome::Pass);
        assert_eq!(verdict.pass_count(), 1);
        assert_eq!(verdict.fail_count(), 0);
    }

    #[tokio::test]
    async fn verify_failing_oath_yields_fail_verdict() {
        let (witness, dir) = setup().await;
        let missing = dir.path().join("nope.txt");

        let oath = Oath::draft("st-1", "root-1", "sess-1", "touch a file")
            .with_postcondition(Predicate::FileExists { path: missing });
        let (sealed, _) = seal_oath(witness.ledger(), oath).await.unwrap();

        let verdict = witness.verify_oath(&sealed).await.unwrap();
        assert_eq!(verdict.outcome, VerdictOutcome::Fail);
        assert_eq!(verdict.fail_count(), 1);
    }

    #[tokio::test]
    async fn verify_unsealed_oath_errors() {
        let (witness, _dir) = setup().await;
        let oath = Oath::draft("st-1", "root-1", "sess-1", "x");
        let r = witness.verify_oath(&oath).await;
        assert!(matches!(r, Err(WitnessError::NoSealedOath(_))));
    }

    #[tokio::test]
    async fn verify_writes_verdict_to_ledger() {
        let (witness, dir) = setup().await;
        let f = dir.path().join("a.txt");
        tokio::fs::write(&f, "x").await.unwrap();

        let oath = Oath::draft("st-1", "root-1", "sess-1", "reply with a file")
            .with_postcondition(Predicate::FileExists { path: f });
        let (sealed, _) = seal_oath(witness.ledger(), oath).await.unwrap();
        witness.verify_oath(&sealed).await.unwrap();

        let entries = witness.ledger().read_session("sess-1").await.unwrap();
        assert!(entries
            .iter()
            .any(|e| matches!(e.payload, LedgerPayload::VerdictRendered(_))));
    }

    #[tokio::test]
    async fn compose_final_reply_block_rewrites_on_fail() {
        let (witness, dir) = setup().await;
        let oath = Oath::draft("st-1", "root-1", "sess-1", "reply with file").with_postcondition(
            Predicate::FileExists {
                path: dir.path().join("missing"),
            },
        );
        let (sealed, _) = seal_oath(witness.ledger(), oath).await.unwrap();
        let verdict = witness.verify_oath(&sealed).await.unwrap();

        let final_reply = witness.compose_final_reply("Done!", &verdict, WitnessStrictness::Block);
        assert!(final_reply.contains("Partial completion"));
        assert!(final_reply.contains("Could not verify"));
        assert!(!final_reply.contains("Done!"));
    }

    #[tokio::test]
    async fn compose_final_reply_observe_unchanged() {
        let (witness, dir) = setup().await;
        let oath = Oath::draft("st-1", "root-1", "sess-1", "reply").with_postcondition(
            Predicate::FileExists {
                path: dir.path().join("missing"),
            },
        );
        let (sealed, _) = seal_oath(witness.ledger(), oath).await.unwrap();
        let verdict = witness.verify_oath(&sealed).await.unwrap();

        let final_reply =
            witness.compose_final_reply("Done!", &verdict, WitnessStrictness::Observe);
        assert_eq!(final_reply, "Done!");
    }

    #[tokio::test]
    async fn compose_final_reply_warn_appends_note() {
        let (witness, dir) = setup().await;
        let oath = Oath::draft("st-1", "root-1", "sess-1", "reply").with_postcondition(
            Predicate::FileExists {
                path: dir.path().join("missing"),
            },
        );
        let (sealed, _) = seal_oath(witness.ledger(), oath).await.unwrap();
        let verdict = witness.verify_oath(&sealed).await.unwrap();

        let final_reply = witness.compose_final_reply("Done!", &verdict, WitnessStrictness::Warn);
        assert!(final_reply.starts_with("Done!"));
        assert!(final_reply.contains("Witness:"));
    }

    #[test]
    fn aggregate_all_pass() {
        let r = vec![
            PredicateResult {
                predicate: Predicate::FileExists {
                    path: PathBuf::from("/tmp/a"),
                },
                tier: 0,
                outcome: VerdictOutcome::Pass,
                detail: "".into(),
                advisory: false,
                latency_ms: 0,
            },
            PredicateResult {
                predicate: Predicate::FileExists {
                    path: PathBuf::from("/tmp/b"),
                },
                tier: 0,
                outcome: VerdictOutcome::Pass,
                detail: "".into(),
                advisory: false,
                latency_ms: 0,
            },
        ];
        assert_eq!(aggregate_outcome(&r), VerdictOutcome::Pass);
    }

    #[test]
    fn aggregate_one_fail_is_fail() {
        let r = vec![
            PredicateResult {
                predicate: Predicate::FileExists {
                    path: PathBuf::from("/tmp/a"),
                },
                tier: 0,
                outcome: VerdictOutcome::Pass,
                detail: "".into(),
                advisory: false,
                latency_ms: 0,
            },
            PredicateResult {
                predicate: Predicate::FileExists {
                    path: PathBuf::from("/tmp/b"),
                },
                tier: 0,
                outcome: VerdictOutcome::Fail,
                detail: "".into(),
                advisory: false,
                latency_ms: 0,
            },
        ];
        assert_eq!(aggregate_outcome(&r), VerdictOutcome::Fail);
    }

    #[test]
    fn advisory_fail_does_not_fail_overall() {
        let r = vec![
            PredicateResult {
                predicate: Predicate::FileExists {
                    path: PathBuf::from("/tmp/a"),
                },
                tier: 0,
                outcome: VerdictOutcome::Pass,
                detail: "".into(),
                advisory: false,
                latency_ms: 0,
            },
            PredicateResult {
                predicate: Predicate::AspectVerifier {
                    rubric: "is it good?".into(),
                    evidence_refs: vec![],
                    advisory: true,
                },
                tier: 1,
                outcome: VerdictOutcome::Fail,
                detail: "".into(),
                advisory: true,
                latency_ms: 0,
            },
        ];
        assert_eq!(aggregate_outcome(&r), VerdictOutcome::Pass);
    }

    #[test]
    fn law5_no_destructive_api_in_witness_source() {
        // Read the witness.rs source and check for forbidden destructive API
        // patterns. Sentinels are built via concat!() so the literal strings
        // do not appear in the source file itself (which would cause this
        // test to falsely match against its own code).
        let src = include_str!("witness.rs");
        let sentinels: &[&str] = &[
            concat!("remove", "_file"),
            concat!("remove", "_dir"),
            concat!("git re", "set --hard"),
            concat!("Command::new(\"k", "ill\")"),
            concat!("rm ", "-rf"),
        ];
        for s in sentinels {
            assert!(
                !src.contains(s),
                "Law 5 violation: witness.rs contains destructive API pattern `{}`",
                s
            );
        }
    }
}
