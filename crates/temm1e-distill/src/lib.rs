//! # Eigen-Tune: Self-Tuning Knowledge Distillation Engine
//!
//! A closed-loop distillation pipeline that:
//! 1. **Collects** every (request, response) pair from LLM provider calls
//! 2. **Scores** quality using user behavior signals (Beta-Binomial model)
//! 3. **Curates** datasets with diversity gating (Shannon entropy)
//! 4. **Trains** local models via pluggable backends (Unsloth/MLX -> GGUF -> Ollama)
//! 5. **Evaluates** using embedding similarity (Wilson score, 99% CI)
//! 6. **Shadows** with user behavior SPRT (Wald, 1945)
//! 7. **Monitors** with CUSUM drift detection (Page, 1954)
//!
//! Zero added LLM cost by default. Optional Teacher Mode for premium evaluation.

pub mod backends;
pub mod collector;
pub mod config;
pub mod engine;
pub mod judge;
pub mod scorer;
pub mod stats;
pub mod store;
pub mod types;

use crate::collector::{EigenTuneCollector, EigenTunePairData};
use crate::config::EigenTuneConfig;
use crate::engine::graduation::GraduationManager;
use crate::engine::monitor::ProductionMonitor;
use crate::engine::router::EigenTuneRouter;
use crate::engine::shadow::ShadowCoordinator;
use crate::scorer::EigenTuneScorer;
use crate::store::EigenTuneStore;
use crate::types::{EigenTier, EigenTuneStatus, QualitySignal, RouteDecision, TierStatusReport};
use std::sync::Arc;

/// The public API for Eigen-Tune.
///
/// Create one instance at startup. Call hooks from the agent runtime.
/// All operations are resilient — failures degrade to cloud, never silence.
pub struct EigenTuneEngine {
    store: Arc<EigenTuneStore>,
    collector: EigenTuneCollector,
    #[allow(dead_code)]
    scorer: EigenTuneScorer,
    router: EigenTuneRouter,
    shadow: ShadowCoordinator,
    monitor: ProductionMonitor,
    graduation: GraduationManager,
    config: EigenTuneConfig,
}

impl EigenTuneEngine {
    /// Create a new Eigen-Tune engine.
    pub async fn new(
        config: &EigenTuneConfig,
        database_url: &str,
    ) -> Result<Self, temm1e_core::types::error::Temm1eError> {
        let store = Arc::new(EigenTuneStore::new(database_url).await?);
        let collector = EigenTuneCollector::new(store.clone(), config.enabled);
        let scorer = EigenTuneScorer::new(store.clone());
        let router = EigenTuneRouter::new(store.clone(), config.clone());
        let shadow = ShadowCoordinator::new(store.clone(), config.clone());
        let monitor = ProductionMonitor::new(store.clone(), config.clone());
        let graduation = GraduationManager::new(store.clone(), config.clone());

        tracing::info!(enabled = config.enabled, "Eigen-Tune: engine initialized");

        Ok(Self {
            store,
            collector,
            scorer,
            router,
            shadow,
            monitor,
            graduation,
            config: config.clone(),
        })
    }

    /// Collection hook — called after every Provider.complete().
    /// Fire-and-forget: errors are logged, never propagated to user.
    pub async fn on_completion(&self, data: EigenTunePairData) {
        if let Err(e) = self.collector.collect(data).await {
            tracing::debug!(error = %e, "Eigen-Tune: collection failed (non-fatal)");
        }
    }

    /// Signal hook — called when user behavior is observed.
    pub async fn on_signal(&self, conversation_id: &str, signal: QualitySignal) {
        if let Err(e) = self.collector.observe_signal(conversation_id, signal).await {
            tracing::debug!(error = %e, "Eigen-Tune: signal failed (non-fatal)");
        }
    }

    /// Routing hook — called before Provider.complete().
    /// On ANY error, returns Cloud (safe fallback).
    pub async fn route(&self, complexity: &str) -> RouteDecision {
        match self.router.route(complexity).await {
            Ok(decision) => decision,
            Err(e) => {
                tracing::debug!(error = %e, "Eigen-Tune: routing failed, fallback to cloud");
                RouteDecision::Cloud
            }
        }
    }

    /// Shadow observation — user behavior during shadow phase.
    pub async fn on_shadow_observation(&self, tier: EigenTier, agree: bool) {
        if let Err(e) = self.shadow.observe(tier, agree).await {
            tracing::debug!(error = %e, "Eigen-Tune: shadow observation failed (non-fatal)");
        }
    }

    /// Monitor observation — user behavior on graduated tier.
    pub async fn on_monitor_observation(&self, tier: EigenTier, agree: bool) {
        match self.monitor.observe(tier, agree).await {
            Ok(true) => {
                // CUSUM alarm — demote
                if let Err(e) = self.graduation.demote(tier).await {
                    tracing::error!(error = %e, "Eigen-Tune: demotion failed");
                }
            }
            Ok(false) => {}
            Err(e) => {
                tracing::debug!(error = %e, "Eigen-Tune: monitor failed (non-fatal)");
            }
        }
    }

    /// Tick — check all tiers for state transitions.
    pub async fn tick(&self) -> Vec<(EigenTier, types::TierState, types::TierState)> {
        match self.graduation.tick().await {
            Ok(t) => t,
            Err(e) => {
                tracing::debug!(error = %e, "Eigen-Tune: tick failed (non-fatal)");
                Vec::new()
            }
        }
    }

    /// Get full status report.
    pub async fn status(&self) -> Result<EigenTuneStatus, temm1e_core::types::error::Temm1eError> {
        let total_pairs = self.store.total_pairs().await?;
        let high_quality = self
            .store
            .total_high_quality(self.config.quality_threshold)
            .await?;

        // Aggregate category counts across all tiers
        let mut all_categories: Vec<(String, i64)> = Vec::new();
        for tier_name in &["simple", "standard", "complex"] {
            let tier_cats = self.store.get_category_counts(tier_name).await?;
            for (cat, cnt) in tier_cats {
                if let Some(entry) = all_categories.iter_mut().find(|(c, _)| c == &cat) {
                    entry.1 += cnt;
                } else {
                    all_categories.push((cat, cnt));
                }
            }
        }
        let counts: Vec<u64> = all_categories.iter().map(|(_, c)| *c as u64).collect();
        let diversity_j = stats::entropy::normalized_entropy(&counts);

        let category_distribution: Vec<(String, f64)> = {
            let total: f64 = all_categories.iter().map(|(_, c)| *c as f64).sum();
            if total > 0.0 {
                all_categories
                    .iter()
                    .map(|(cat, count)| (cat.clone(), *count as f64 / total))
                    .collect()
            } else {
                Vec::new()
            }
        };

        let all_tiers = self.store.get_all_tiers().await?;
        let tiers: Vec<TierStatusReport> = all_tiers
            .iter()
            .map(|t| {
                let accuracy_ci = t.eval_accuracy.and_then(|acc| {
                    t.eval_n.map(|n| {
                        let successes = (acc * n as f64).round() as u64;
                        stats::wilson::wilson_interval(
                            successes,
                            n as u64,
                            self.config.graduation_confidence,
                        )
                    })
                });

                TierStatusReport {
                    tier: t.tier,
                    state: t.state,
                    pair_count: t.pair_count,
                    accuracy: t.eval_accuracy,
                    accuracy_ci,
                    sprt_lambda: if t.state == types::TierState::Shadowing {
                        Some(t.sprt_lambda)
                    } else {
                        None
                    },
                    sprt_progress: if t.state == types::TierState::Shadowing {
                        Some(format!("{}/{}", t.sprt_n, self.config.sprt_max_samples))
                    } else {
                        None
                    },
                    serving_model: t
                        .serving_run_id
                        .as_ref()
                        .map(|_| "eigentune-model".to_string()),
                    savings_usd: 0.0,
                }
            })
            .collect();

        Ok(EigenTuneStatus {
            enabled: self.config.enabled,
            total_pairs,
            high_quality_pairs: high_quality,
            diversity_j,
            category_distribution,
            tiers,
            total_savings_usd: 0.0,
        })
    }

    /// Format status for chat display.
    pub async fn format_status(&self) -> String {
        match self.status().await {
            Ok(status) => {
                let mut out = String::from("EIGEN-TUNE STATUS\n\n");
                out.push_str(&format!(
                    "Data: {} pairs collected | {} high-quality\n",
                    status.total_pairs, status.high_quality_pairs
                ));
                out.push_str(&format!("Diversity: J = {:.2}\n\n", status.diversity_j));

                for t in &status.tiers {
                    let icon = match t.state {
                        types::TierState::Graduated => "●",
                        types::TierState::Shadowing => "◐",
                        _ => "○",
                    };
                    out.push_str(&format!(
                        "{} {:8} {}\n",
                        icon,
                        t.tier.as_str(),
                        t.state.as_str()
                    ));
                }
                out
            }
            Err(e) => format!("Eigen-Tune: error: {}", e),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_creation() {
        let config = EigenTuneConfig::default();
        let engine = EigenTuneEngine::new(&config, "sqlite::memory:").await;
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_engine_status() {
        let config = EigenTuneConfig::default();
        let engine = EigenTuneEngine::new(&config, "sqlite::memory:")
            .await
            .unwrap();
        let status = engine.status().await.unwrap();
        assert_eq!(status.total_pairs, 0);
        assert_eq!(status.tiers.len(), 3);
    }

    #[tokio::test]
    async fn test_route_default_cloud() {
        let config = EigenTuneConfig::default();
        let engine = EigenTuneEngine::new(&config, "sqlite::memory:")
            .await
            .unwrap();
        let decision = engine.route("simple").await;
        assert!(matches!(decision, RouteDecision::Cloud));
    }

    #[tokio::test]
    async fn test_format_status_output() {
        let config = EigenTuneConfig::default();
        let engine = EigenTuneEngine::new(&config, "sqlite::memory:")
            .await
            .unwrap();
        let text = engine.format_status().await;
        assert!(text.contains("EIGEN-TUNE STATUS"));
    }
}
