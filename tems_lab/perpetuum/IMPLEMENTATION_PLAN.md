# Perpetuum: Implementation Plan (Final)

> Anchored from deep research + architectural design session, 2026-03-31.
> Branch: `perpetual`

---

## 1. What Is Perpetuum

Perpetuum transforms Tem from a **reactive request-response agent** into a **perpetual, time-aware, autonomous entity**. Today Tem processes a message, loops through tools, returns a response, then goes dormant. Perpetuum makes Tem always-on: aware of time, capable of scheduling and monitoring, able to juggle multiple concurrent tasks, and proactively investing idle time in self-improvement.

**This is the final architectural piece before Tem Simulation** — where Tem operates freely within its own simulation environment.

---

## 2. Design Principles

### 2.1 The Enabling Framework (GOVERNING PRINCIPLE)

Perpetuum ENABLES LLMs. It never constrains them.

**The framework provides infrastructure.** Time awareness, persistence, concurrency, scheduling substrate, monitoring scaffolding — capabilities the LLM can't do on its own (it can't count seconds or spawn threads).

**The LLM provides ALL intelligence.** Every decision that requires judgment is delegated to the LLM: What's relevant? What's urgent? Should the schedule change? Is this monitor still useful? Should I notify now or wait? The framework never answers these questions with hardcoded logic.

**No ceilings.** We never bake in heuristics or deterministic algorithms where LLM judgment would be better. A formula is a ceiling — it caps intelligence at the developer's foresight. We pass raw data and context to the LLM and let it decide. A smarter model makes smarter decisions. The framework doesn't change.

**Timeproof.** Prompts and context injection are open-ended — they present information, not rigid instructions. A more capable model extracts more value from the same context. A less capable model still functions with simpler judgments. The framework scales with model intelligence automatically.

**Deterministic code is substrate, not brain.** Timers count seconds (code). Persistence writes to SQLite (code). Tokio spawns tasks (code). But "should I check Reddit more often?" is cognition — that's the LLM. The line is clear: infrastructure is code, intelligence is LLM.

**Fallback gracefully.** When the LLM is unavailable (provider down, rate limited), simple deterministic fallbacks keep the system alive. The fallback is the brainstem — it keeps breathing. But it's never the primary intelligence.

### 2.2 Single Model

The entire Perpetuum system uses whatever single model the user has configured. No model routing, no cheap/expensive splits, no "use Haiku for monitoring and Sonnet for conversations." One model, one brain. When the user upgrades their model, everything gets smarter at once.

This is a deliberate choice:
- Users often use proxies (OpenRouter, custom endpoints) with unknown model lists
- Model routing is a maintenance nightmare and a debugging black hole
- A single model is predictable, auditable, and simple to reason about
- The enabling framework philosophy means even a smaller model produces useful results — it just makes simpler judgments

### 2.3 Proactive Choice, Not Constraint

Tem's states (Active/Idle/Sleep/Dream) represent what Tem proactively CHOOSES to do, not constraints imposed on it. There is no energy budget, no tiredness metaphor, no forced sleep. Tem is always instantly available. Sleep is productive self-improvement time that Tem enters when idle — because it's smart enough to invest idle time rather than waste it.

---

## 3. Current State (What Exists Today)

| Component | Status | Location |
|-----------|--------|----------|
| HeartbeatRunner (periodic checklist) | Integrated, working | `crates/temm1e-automation/src/heartbeat.rs` |
| ProactiveManager (triggers, cooldowns) | Built, **NOT integrated** | `crates/temm1e-agent/src/proactive.rs` |
| ConsciousnessEngine (pre/post LLM observation) | Built, integrated | `crates/temm1e-agent/src/consciousness_engine.rs` |
| EigenTune (distillation closed-loop) | Built, integrated | `crates/temm1e-distill/src/` |
| TaskQueue (SQLite checkpoint) | Built, optional | `crates/temm1e-agent/src/task_queue.rs` |
| Duration parsing ("30m", "2h") | Working | `crates/temm1e-automation/src/duration.rs` |
| CronSchedule trigger type | Type exists, **no executor** | `crates/temm1e-agent/src/proactive.rs:47` |
| train_schedule cron string | Config field, **no scheduler** | `crates/temm1e-distill/src/config.rs` |

**Gaps:** No cron executor. No time-of-day triggers. No concurrent task management. No self-sleep during long operations. No temporal reasoning. The heartbeat is the only periodic mechanism.

---

## 4. External Landscape (Research Summary)

| System | Scheduling | Novel Pattern | Limitation |
|--------|-----------|---------------|------------|
| **OpenClaw** | CronService (at/every/cron) | contextMessages (job carries context) | No adaptive scheduling, no time cognition |
| **Claude Code** | /schedule, /loop | Remote agents on Anthropic infra | Session-bound, no persistent entity |
| **LangGraph** | CronClient (5-field cron) | Self-scheduling agent tool | Platform-dependent, no sleep/dream |
| **BabyAGI** | Pure task loop | Self-generating task queue | No time awareness, runs forever blindly |
| **Letta/MemGPT** | Background agents | Sleep-time compute for memory | No scheduling, no monitoring |
| **Temporal** | Durable schedules | Durable sleep (process detaches) | Workflow orchestration, not AI-native |
| **"Let Them Sleep"** | Nightly training cycle | Sleep as model adaptation | Concept paper, not implemented |

**The gap nobody fills:** No system combines temporal cognition + LLM-cognitive scheduling + productive sleep + concurrent multi-tasking + self-scheduling in a single framework. Each does one piece.

---

## 5. Architecture

### 5.1 Entity State Machine

```
                         ┌──────────────────┐
     user msg ──────────►│      ACTIVE      │◄──── alarm / monitor / scheduled task
     any wake trigger ──►│  (serving users, executing tasks)  │
                         └────────┬─────────┘
                                  │ no active tasks, monitors on autopilot
                                  │ Tem DECIDES to rest (proactive choice)
                         ┌────────▼─────────┐
                         │       IDLE       │  monitors running, ready for work
                         └────────┬─────────┘
                                  │ Tem decides idle time → productive self-improvement
                         ┌────────▼─────────┐
                         │      SLEEP       │  memory consolidation, log analysis,
                         │                  │  failure mining, session cleanup
                         └────────┬─────────┘
                                  │ consolidation done + training data ready
                         ┌────────▼─────────┐
                         │      DREAM       │  Eigen-Tune distillation cycle
                         └──────────────────┘

  ANY STATE ──► ACTIVE  instantly via: user message, alarm fires,
                        monitor detects change, parked task resumes
```

**Critical:** Monitors and scheduled tasks NEVER stop. They run as independent tokio tasks regardless of Tem's state. Only the "main brain" transitions.

| State | What Tem Is Doing | Monitors | Wake Latency |
|-------|-------------------|----------|--------------|
| Active | Serving user, executing tasks | All running | N/A |
| Idle | Waiting, monitors on autopilot | All running | <50ms |
| Sleep | Productive self-improvement | All running | <100ms |
| Dream | Eigen-Tune distillation | All running | <100ms |

### 5.2 Component Architecture

```
┌───────────────────────────────────────────────────────────┐
│                    PERPETUUM CORE                          │
│                                                           │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ CHRONOS  │  │ CORTEX   │  │  PULSE   │  │ VOLITION │  │
│  │ (time +  │  │ (concern │  │ (timer   │  │ (agency: │  │
│  │ temporal  │  │ scheduler│  │  wheel + │  │ initiative│ │
│  │ cognition)│  │ + parking│  │  cron)   │  │  loop)   │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  │
│       └──────────────┴─────────────┴─────────────┘        │
│                          │                                │
│  ┌───────────────────────▼───────────────────────────┐    │
│  │            CONSCIENCE (State Machine)               │    │
│  │         Active ↔ Idle ↔ Sleep ↔ Dream               │    │
│  │    (Volition drives transitions cognitively)        │    │
│  └────────────────────────────────────────────────────┘    │
└───────────────────────────────────────────────────────────┘
                          │
            ┌─────────────▼─────────────┐
            │     EXISTING SYSTEMS       │
            │  AgentRuntime  (messages)   │
            │  HeartbeatRunner (compat)   │
            │  ConsciousnessEngine (obs)  │
            │  EigenTuneEngine (distill)  │
            │  ProactiveManager (triggers)│
            │  Hive (swarm, if complex)   │
            └────────────────────────────┘
```

### 5.3 Chronos — Internal Clock + Temporal Cognition

Not just `SystemTime::now()` — Chronos is Tem's subjective experience of time.

```rust
pub struct Chronos {
    timezone: chrono_tz::Tz,
    boot_time: DateTime<Utc>,
    last_user_interaction: Option<DateTime<Utc>>,
    last_state_transition: DateTime<Utc>,
    activity_log: CircularBuffer<ActivityEntry>,  // 168 entries (1 week hourly)
}

pub struct TemporalContext {
    pub now: DateTime<Utc>,
    pub local_time: String,              // "Monday 3:47 PM PST"
    pub uptime: Duration,
    pub idle_duration: Duration,
    pub conscience_state: String,        // "active", "idle", "sleep", "dream"
    pub active_concerns: Vec<ConcernSummary>,
    pub parked_tasks: Vec<ParkedTaskSummary>,
    pub next_event: Option<(String, Duration)>,
    pub user_active_probability: f64,    // Learned circadian pattern
}
```

**Temporal Context Injection** — prepended to LLM calls:
```
[Temporal Awareness]
Time: Monday 3:47 PM PST | Uptime: 14h32m | Idle: 6h23m | State: sleep → active (woke for this message)
Active: 2 monitors (reddit/claudecode @5m, facebook/page @3m), 1 alarm (wake-user at 6:00 AM)
Parked: 1 task (waiting for github API, parked 3m ago)
Next event: facebook check in 1m47s
User pattern: typically active 9AM-11PM PST (32% likely active now)
```

**Enabling framework in action:** The temporal context is presented as information, not instructions. The LLM decides what to do with it. A smarter model extracts more nuance. A simpler model still benefits from knowing the time. No ceiling.

### 5.4 Cortex — Multi-Concern Scheduler + Task Parking

```rust
pub enum Concern {
    Foreground { chat_id: String, session_id: String },
    Monitor { id, name, description, user_intent, schedule, check_fn, notify_chat_id, last_result, history },
    Alarm { id, name, fire_at, message, notify_chat_id },
    Recurring { id, name, schedule: CronSchedule, action, notify_chat_id },
    Parked { id, original_concern, state: TaskState, checkpoint },
    SelfWork { id, kind: SelfWorkKind },
}
```

**Key design: Monitors carry user intent.** When a monitor is created, the original user request is stored verbatim: `"Watch r/claudecode for anything about MCP servers."` Every interpretation and review call receives this intent. The LLM always knows WHY it's monitoring, not just WHAT.

**Monitor history:** Recent check results (last N) are stored so the LLM has pattern context during review.

All concerns persist to SQLite. On restart, Perpetuum reloads and resumes. Each concern runs as an independent tokio task.

### 5.5 LLM-Cognitive Scheduling (Replacing Deterministic Formulas)

This is the core architectural decision driven by the enabling framework principle.

**Three layers, clear separation:**

**Layer 1 — Pulse (Deterministic Substrate)**
The timer wheel fires checks on schedule. Pure code. No LLM. This is the heartbeat — it never breaks, never costs tokens, never depends on a provider.

```
Pulse tick → "time to check monitor-reddit" → fire check
```

**Layer 2 — Check Interpretation (LLM-Powered)**
When a check finds new content, the LLM evaluates it against the user's stored intent:

```
You are evaluating monitor results. The user's intent: "{user_intent}"
New content found: {raw_content}
Previous check result: {last_result}

Assess: Is this relevant to the user's intent? Rate urgency. Should the user be notified?
If yes, write a concise notification summary.
```

The LLM decides relevance, urgency, and notification worthiness. No regex patterns, no keyword matching, no heuristic rules. Pure LLM judgment. A smarter model means better filtering.

**Layer 3 — Schedule Review (LLM-Powered)**
Periodically (every N checks, configurable), the LLM reviews the monitoring pattern:

```
You've been monitoring "{name}" for {duration}. User's intent: "{user_intent}"
Last {N} check results: {history_summary}
Current interval: {interval}
Temporal context: {temporal_context}

Consider:
1. Is the current frequency appropriate given the observed activity pattern?
2. Should the frequency vary by time of day or day of week?
3. Is this monitor still serving the user's intent, or has the situation changed?
4. Any recommendations?

Respond with: adjusted_interval (or "keep"), reasoning, and any user-facing recommendation.
```

The LLM might respond:
- "Activity concentrates 9AM-6PM PST. Reduce overnight to every 20 minutes, increase to every 2 minutes during business hours."
- "This topic has gone quiet for 3 days. The user might have moved on. Recommend asking if they still need this monitor."
- "Keep current interval. Activity is steady and the user asked for exactly this frequency."

**Deterministic fallback (brainstem):** If the LLM is unavailable (provider down, rate limited), a simple fallback keeps the monitor alive:
```rust
fn deterministic_fallback(consecutive_empty: u32, current: Duration, min: Duration, max: Duration) -> Duration {
    // Exponential backoff when nothing found, reset on detection
    // This is NOT intelligence — it's life support
}
```

**Why not just use the formula?** Because a formula can't say "Reddit is quiet because it's Sunday" or "this error log entry is about the bug you're working on" or "you asked me to watch this 5 days ago and the situation has resolved." These require understanding context, intent, and meaning. That's what LLMs do.

**Cost:** One LLM call per check that finds changes + one review call every N checks. At ~$0.15/day per monitor with 5-minute intervals, this is negligible for a system that replaces manual checking hundreds of times per day. And the cost drops every year as models get cheaper.

### 5.6 Pulse — Unified Timer Engine

Hierarchical timing wheel (second/minute/hour/day buckets). Single tokio task, 1s tick resolution, O(1) insertion and firing. Cron support via the `cron` crate with timezone awareness.

### 5.7 Task Parking (Self-Sleep During Long Operations)

When Tem runs a long tool, the current approach blocks the entire agent loop. Task parking allows Tem to checkpoint the task, free itself, do other work, and resume when the async result arrives.

```rust
pub enum ParkReason {
    WaitingForApi { endpoint: String },
    WaitingForProcess { pid: u32 },
    WaitingForTimer { wake_at: DateTime<Utc> },
    WaitingForUserInput,
    WaitingForMonitorResult { monitor_id: String },
}

pub enum ResumeCondition {
    Channel(oneshot::Receiver<TaskResumeSignal>),
    At(DateTime<Utc>),
    Future(Pin<Box<dyn Future<Output = TaskResumeSignal> + Send>>),
}
```

**Deferred to Phase 4.** The most complex component. Needs simpler features solid first.

### 5.8 Conscience — Proactive State Machine

```rust
pub enum ConscienceState {
    Active,
    Idle { since: DateTime<Utc> },
    Sleep { since: DateTime<Utc>, work: SelfWorkKind, progress: f32 },
    Dream { since: DateTime<Utc>, tier: Option<EigenTier> },
}

pub enum SelfWorkKind {
    MemoryConsolidation,      // Compact lambda-Memory, prune stale sessions
    FailureAnalysis,          // LLM reviews logs, extracts failure patterns
    LogIntrospection,         // LLM reviews recent interactions, extracts learnings
    SessionCleanup,           // Archive old sessions, free storage (no LLM)
    BlueprintRefinement,      // Update blueprint weights from success/fail stats (no LLM)
}
```

Transitions are proactive. Sleep-time work that uses LLM calls (FailureAnalysis, LogIntrospection) is a deliberate investment — Tem spends tokens to become smarter. This is consistent with the enabling framework: the LLM is doing the thinking, the framework is providing the time and context.

### 5.9 Agent Tools (Self-Scheduling)

```
create_alarm(name, fire_at, message)
create_monitor(name, target, schedule, chat_id)
create_recurring(name, cron, action, chat_id)
list_concerns()
cancel_concern(id)
adjust_schedule(id, new_schedule)
park_task(reason, resume_condition)          // Phase 4
```

Natural language scheduling: users say "wake me at 6am" and the LLM calls `create_alarm`. The LLM IS the parser. No NLP pipeline, no intent classifier, no regex. Pure LLM tool calling.

### 5.10 Monitor Types

```rust
pub enum MonitorCheck {
    WebCheck { url, selector, extract: ExtractMode },
    CommandCheck { command, success_pattern },
    BrowserCheck { url, action_script },         // Prowl blueprint
    FileCheck { path, watch: FileWatchMode },
}
```

**Change detection:** Monitors store `last_result`. Raw change detection is deterministic (content hash comparison). Semantic change evaluation is LLM-powered.

---

## 6. Critical Risks & Mitigations

### 6.1 Concurrency Debugging

**Risk:** Multiple tokio tasks create non-deterministic failures.
**Mitigation:** Phase 0 builds observability FIRST. Structured tracing spans for every concern lifecycle event, state transition, timer firing, LLM evaluation.

### 6.2 Task Parking Complexity

**Risk:** Checkpoint serialization, LLM context resumption, resume failures.
**Mitigation:** Defer to Phase 4. Start with timer-based parking only.

### 6.3 SQLite Under Concurrent Writes

**Risk:** Many monitors writing results concurrently.
**Mitigation:** Batch writes on timer (flush every 5s). WAL mode. In-memory state with periodic snapshots.

### 6.4 Temporal Context Token Cost

**Risk:** ~100-150 tokens per call. Classification calls get more expensive.
**Mitigation:** Configurable injection depth. Classification gets minimal context (state + time). Full turns get complete block. The cost is intentional — temporal awareness is worth 100 tokens.

### 6.5 Monitor Isolation

**Risk:** Panicking or hanging monitor cascades (the `ẹ` incident pattern).
**Mitigation:** Each monitor: own tokio task + `catch_unwind` + per-monitor timeout + error budget (3 consecutive failures → disable + notify user).

### 6.6 State Machine Correctness

**Risk:** 4 states x ~6 triggers = 24 paths.
**Mitigation:** Exhaustive transition tests covering every combination.

### 6.7 LLM-Cognitive Scheduling Availability

**Risk:** LLM provider is down during schedule review window.
**Mitigation:** Deterministic fallback keeps monitors alive. Review is retried next window. Monitor never stops checking — only the intelligence of the review degrades gracefully.

### 6.8 Restart Recovery

**Risk:** Crash mid-operation leaves bad state.
**Mitigation:** Idempotent concern checks. Overdue alarm delivery with "[late]" flag. Checkpoint-based training recovery for EigenTune.

---

## 7. New Crate: `temm1e-perpetuum`

```
crates/temm1e-perpetuum/
  src/
    lib.rs              — Public API: Perpetuum struct, start(), concern CRUD
    chronos.rs          — Internal clock, temporal context, activity pattern learning
    cortex.rs           — Concern scheduler, priority queue, task parking
    pulse.rs            — Timing wheel, cron evaluator, timer registry
    conscience.rs       — State machine (Active/Idle/Sleep/Dream)
    concern.rs          — Concern types, lifecycle, user intent storage
    monitor.rs          — Monitor checks (web, command, browser, file)
    cognitive.rs        — LLM-powered check interpretation + schedule review
    volition.rs         — Initiative loop: perceive, evaluate, decide, act
    parking.rs          — Task parking: checkpoint, resume conditions
    store.rs            — SQLite persistence for concerns, state, history, volition notes
    types.rs            — TemporalContext, ConcernSummary, Schedule
    tools.rs            — Agent tool definitions
    self_work.rs        — Sleep-time work (consolidation, analysis, refinement)
  Cargo.toml
```

**Dependencies:** temm1e-core, temm1e-automation, chrono + chrono-tz, cron, sqlx, tokio, reqwest

**Integration:** main.rs creates Perpetuum instead of bare HeartbeatRunner. Agent tools via ToolDeclarations. TemporalContext injected in runtime.rs. LLM calls via existing provider infrastructure.

---

## 8. Implementation Phases

### Phase 0: Observability Layer
- Structured tracing for concurrent operations
- Every concern gets a tracing span: `concern_id`, `concern_type`, `state`
- State machine transition logging: `from`, `to`, `reason`, `trigger`
- LLM evaluation logging: `monitor_id`, `interpretation`, `decision`
- Build debugging infrastructure BEFORE the complexity

### Phase 1: Chronos + Pulse + Alarm
- Create `temm1e-perpetuum` crate
- Chronos (internal clock, temporal context generation, timezone)
- Pulse (timing wheel + cron evaluator)
- Store (SQLite: concerns, state, transitions)
- Alarm concern: "remind me in 5 minutes" → fires → notifies user
- ConscienceState enum + basic transition logic
- TemporalContext injection (configurable depth)
- Unit tests
- **Deliverable:** Tem can set alarms and tell you what time it is

### Phase 2: Monitors + LLM-Cognitive Scheduling
- WebCheck monitors with raw change detection
- LLM-powered check interpretation (relevance, urgency, notification)
- LLM-powered schedule review (periodic frequency adjustment)
- Deterministic fallback for when LLM unavailable
- Monitor isolation (catch_unwind, timeout, error budget)
- User intent stored per monitor
- `create_monitor`, `list_concerns`, `cancel_concern` tools
- Unit + integration tests
- **Deliverable:** "monitor reddit r/claudecode" with intelligent interpretation

### Phase 3: Integration + Full Tool Suite
- Wire Perpetuum into main.rs (replace bare heartbeat)
- Backward compat: existing `[heartbeat]` config → Recurring concern
- All agent tools registered via ToolDeclarations
- Full TemporalContext injection in runtime.rs
- Conscience state transitions (Active ↔ Idle ↔ Sleep)
- Recurring concerns (cron-based)
- CommandCheck + FileCheck + BrowserCheck monitors
- **Deliverable:** Full scheduling system live in TEMM1E

### Phase 4: Task Parking + Self-Work
- Task parking: self-sleep during long operations
- Timer-based parking first, then channel-based
- SelfWork concerns: MemoryConsolidation, SessionCleanup, BlueprintRefinement (no LLM)
- LLM-powered self-work: FailureAnalysis, LogIntrospection (deliberate token investment)
- Dream state → EigenTune distillation trigger
- **Deliverable:** Tem parks tasks, does productive self-improvement in idle time

### Phase 5: Tem Simulation Foundation
- Self-scheduling intelligence (Tem creates concerns autonomously)
- Activity pattern learning (user_active_probability in Chronos)
- Multiple simultaneous foreground concerns
- Monitor result summarization (batch findings into digest)
- State persistence across restarts
- Document Tem Simulation extension points
- **Deliverable:** Tem operates as a fully autonomous perpetual entity

---

## 9. Verification Plan

1. **Unit tests** — Chronos, Pulse, Cortex, Conscience, Store, Cognitive independently
2. **State machine exhaustive test** — every (state x trigger) combination (24 paths)
3. **LLM interpretation test** — mock provider, verify interpretation prompts and response handling
4. **Schedule review test** — verify LLM review adjusts intervals, verify deterministic fallback
5. **Integration tests** — concern lifecycle: create → fire → interpret → notify → review → adjust
6. **Task parking test** — park → do other work → resume on signal
7. **CLI self-test** — multi-turn:
   - "Set alarm for 30 seconds" → fires, notified
   - "Monitor httpbin.org/uuid every 10s" → detects changes, LLM interprets
   - "What's scheduled?" → list_concerns
   - "Cancel the monitor" → removed
   - Verify state transitions in logs
8. **Concurrent test** — monitor + user conversation simultaneously
9. **Restart recovery** — concerns survive restart
10. **Backward compat** — existing `[heartbeat]` config unchanged

---

## 10. Config Schema

```toml
[perpetuum]
enabled = true
timezone = "America/Los_Angeles"
max_concerns = 100

[perpetuum.conscience]
idle_threshold_secs = 900       # 15 min idle → Sleep (productive)
dream_threshold_secs = 3600     # 1 hr sleep → Dream if data ready

[perpetuum.pulse]
tick_resolution_secs = 1

[perpetuum.cognitive]
review_every_n_checks = 20      # LLM schedule review frequency
interpret_changes = true        # LLM evaluates monitor findings (vs raw passthrough)

[perpetuum.parking]
max_parked_tasks = 20
checkpoint_to_disk = true
```

---

## 11. Volition — The Agency Layer (IN SCOPE)

Volition is NOT a separate system. It is one more concern type in the Cortex — a special Initiative concern that fires a periodic LLM call where Tem thinks about what it should be doing.

### 5.11 Volition

```rust
Concern::Initiative {
    id: String,
    interval: Duration,           // e.g., every 10-15 minutes
    last_run: Option<DateTime<Utc>>,
    // Also fires after significant events: conversation ends, monitor finds something, error occurs
}
```

**When it fires, it makes one LLM call with:**
- TemporalContext (from Chronos)
- Active concerns summary (from Cortex)
- Recent monitor results (from Store)
- Recent conversation summaries (from Store)
- Recent errors/failures (from Store)

**The LLM outputs structured decisions:**
- Create/modify/cancel concerns (same Cortex API as user-triggered scheduling)
- Queue a proactive notification to user
- Set self-work priority / trigger state transition
- Write internal notes (persisted for next initiative cycle)

**Guardrails:**
- `max_concerns` cap — Volition can't exceed it
- Rate limit on concern creation per initiative cycle (default: max 2 new concerns)
- All Volition-created concerns tagged `source: "volition"` — user can see and override
- Configurable: `enabled = false` by default, opt-in
- `max_actions_per_cycle` — bounds what Volition can do per run
- Volition cannot create more Initiative concerns (no self-replication)

**Event-triggered initiative:** Beyond the periodic timer, Volition also fires after:
- A user conversation ends (opportunity to reflect and schedule follow-ups)
- A monitor detects a significant change (opportunity to reason about escalation)
- An error occurs (opportunity to create self-repair concerns)
- State transition to Idle (opportunity to decide what to do with idle time)

### Updated Phase 5

Phase 5 is now **Volition** — a concrete deliverable, not a placeholder:

### Phase 5: Volition (Agency)
- Implement Initiative concern type in Cortex
- Volition LLM prompt: perceive → evaluate → decide → act
- Event-triggered initiative (post-conversation, post-error, post-detection)
- Guardrails: max_actions_per_cycle, concern creation rate limit, source tagging
- Activity pattern learning (user_active_probability in Chronos)
- Conscience state transitions driven by Volition (replaces idle thresholds)
- Multiple simultaneous foreground concerns
- Internal notes persistence (Volition remembers its reasoning across cycles)
- **Deliverable:** Tem proactively creates monitors, cancels stale concerns, notifies users, and directs its own self-improvement

### Updated Config

```toml
[perpetuum.volition]
enabled = false                  # Opt-in
interval_secs = 900              # 15 min default
max_actions_per_cycle = 2        # Max concern CRUD per initiative run
event_triggered = true           # Fire after conversations, errors, detections
```

### Updated Verification

11. **Volition test** — enable volition, have conversation about risky PR, verify Tem creates a monitor proactively
12. **Volition guardrails** — verify max_concerns cap prevents runaway, verify source tagging, verify user can cancel volition-created concerns

---

## 12. Tem Simulation Alignment

With Volition included, Perpetuum delivers both the **body** (infrastructure) and the **mind** (agency). Tem Simulation extends further into:
- Sandboxed autonomous environment
- Hypothesis exploration and strategy experimentation
- Adversarial self-testing
- Multi-round distillation with automated evaluation
- Goal formation beyond current concerns

Perpetuum + Volition is the complete foundation. Simulation is the expansion.
