/// Structured tracing helpers for Perpetuum spans.
///
/// Every concern lifecycle event, state transition, and timer firing
/// gets a structured tracing span. This is Phase 0 — observability
/// BEFORE complexity.
/// Log a concern lifecycle event.
pub fn trace_concern_event(concern_id: &str, concern_type: &str, event: &str) {
    tracing::info!(
        target: "perpetuum",
        concern_id = %concern_id,
        concern_type = %concern_type,
        event = %event,
        "concern lifecycle"
    );
}

/// Log a conscience state transition.
pub fn trace_state_transition(from: &str, to: &str, reason: &str, trigger: Option<&str>) {
    tracing::info!(
        target: "perpetuum",
        from_state = %from,
        to_state = %to,
        reason = %reason,
        trigger = trigger.unwrap_or("none"),
        "conscience transition"
    );
}

/// Log a concern firing (timer expired, check due).
pub fn trace_concern_fire(concern_id: &str, concern_type: &str) {
    tracing::debug!(
        target: "perpetuum",
        concern_id = %concern_id,
        concern_type = %concern_type,
        "concern fired"
    );
}

/// Log a monitor check result.
pub fn trace_monitor_check(concern_id: &str, name: &str, change_detected: bool, notified: bool) {
    tracing::info!(
        target: "perpetuum",
        concern_id = %concern_id,
        monitor_name = %name,
        change_detected = change_detected,
        notified = notified,
        "monitor check"
    );
}

/// Log a cognitive LLM evaluation.
pub fn trace_cognitive_eval(concern_id: &str, eval_type: &str, result_summary: &str) {
    tracing::info!(
        target: "perpetuum",
        concern_id = %concern_id,
        eval_type = %eval_type,
        result = %result_summary,
        "cognitive evaluation"
    );
}

/// Log a volition initiative cycle.
pub fn trace_volition_cycle(
    actions_taken: usize,
    concerns_created: usize,
    concerns_cancelled: usize,
    notifications_sent: usize,
) {
    tracing::info!(
        target: "perpetuum",
        actions = actions_taken,
        created = concerns_created,
        cancelled = concerns_cancelled,
        notifications = notifications_sent,
        "volition cycle"
    );
}

/// Log a pulse timer event.
pub fn trace_pulse_tick(due_count: usize, next_fire_secs: Option<f64>) {
    tracing::debug!(
        target: "perpetuum",
        due_concerns = due_count,
        next_fire_in_secs = next_fire_secs.unwrap_or(-1.0),
        "pulse tick"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracing_functions_compile() {
        // Verify all tracing functions compile with expected signatures.
        // Actual tracing output tested in integration tests with a subscriber.
        trace_concern_event("test-001", "alarm", "created");
        trace_state_transition("active", "idle", "no_foreground", None);
        trace_concern_fire("test-001", "alarm");
        trace_monitor_check("mon-001", "reddit", true, false);
        trace_cognitive_eval("mon-001", "interpret", "relevant=true");
        trace_volition_cycle(2, 1, 0, 1);
        trace_pulse_tick(3, Some(45.0));
    }
}
