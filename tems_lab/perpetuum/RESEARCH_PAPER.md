# Perpetuum: An Enabling Framework for Perpetual, Time-Aware LLM Agents

> Draft research paper. Target: arXiv preprint → AAAI / NeurIPS Systems track.

---

## Abstract

Current LLM-based agents operate in a request-response paradigm: they activate when triggered, process an input, and return to dormancy. They have no internal sense of time, cannot schedule future actions, and cannot manage concurrent long-running concerns. We present **Perpetuum**, a framework that transforms LLM agents into perpetual, time-aware, autonomous entities. Perpetuum introduces five contributions: (1) **temporal cognition** — injecting time as a first-class cognitive input to LLM reasoning, enabling agents to reason about temporal context rather than merely react to triggers; (2) **LLM-cognitive scheduling** — a three-layer architecture where deterministic timers provide substrate, LLMs interpret monitoring results for relevance and urgency, and periodic LLM reviews adjust scheduling strategy based on observed patterns; (3) **concern-based multi-tasking** — formalizing an agent's concurrent responsibilities as typed, priority-scheduled concerns (monitors, alarms, recurring tasks, parked operations) that execute independently; (4) **the enabling framework principle** — a design philosophy where the framework provides infrastructure (time, persistence, concurrency) while delegating all judgment to the LLM, ensuring the system's intelligence scales with model capability without code changes; and (5) **volition** — a periodic initiative loop where the agent autonomously reasons about what it should be doing, proactively creating concerns, directing self-improvement, and making scheduling decisions without user prompting. We implement Perpetuum in TEMM1E, a production Rust AI agent runtime serving real users across messaging platforms. Evaluation demonstrates that temporal cognition improves scheduling decision quality, LLM-cognitive scheduling reduces irrelevant notifications while maintaining detection speed, and the architecture supports concurrent concern execution with no foreground latency degradation.

---

## 1. Introduction

The dominant paradigm for LLM-based agents is request-response: a user sends a message, the agent processes it through a tool-use loop, returns a response, and goes dormant until the next message arrives. This paradigm is fundamentally limiting. Real-world assistance requires persistence — the ability to monitor, schedule, wait, and act across time.

Consider what users actually ask of intelligent assistants:
- "Wake me up at 6 AM with a weather summary"
- "Watch r/claudecode for posts about MCP servers"
- "Check my Facebook page for new comments every 3 minutes"
- "Deploy staging and let me know when it's ready" (while doing other work)
- "Every weekday at 9 AM, summarize open PRs older than 3 days"

None of these can be served by a request-response agent. Each requires temporal awareness, scheduling, persistence across sessions, or concurrent operation — capabilities absent from current agent architectures.

Existing approaches to agent scheduling fall into two categories. **External scheduling** wraps a cron daemon or cloud scheduler around agent invocations (OpenClaw [1], Claude Code [2], LangGraph [3]). This treats the agent as a function called on a timer — the agent itself has no understanding of time or scheduling. **Continuous loops** run the agent perpetually without time awareness (BabyAGI [4], AutoGPT [5]). These waste resources and lack the ability to sleep, prioritize, or adapt.

Neither approach treats the agent as an entity with temporal cognition — one that reasons about time, manages its own schedule, and makes intelligent decisions about when to act, when to wait, and when to invest in self-improvement.

We present Perpetuum, a framework that bridges this gap. Our key insight is the **enabling framework principle**: the framework should provide infrastructure that LLMs cannot do themselves (count seconds, persist state, spawn concurrent tasks) while delegating all cognitive decisions to the LLM (what's relevant, what's urgent, when to adjust, when to sleep). This separation ensures that as LLMs improve, the framework's effective intelligence improves automatically — without code changes, without new heuristics, without architectural modifications.

Our contributions:

1. **Temporal Cognition.** We formalize temporal context as a structured input to LLM reasoning. The agent receives its temporal state (current time, idle duration, active concerns, user activity probability) and reasons with it — a cognitive capability rather than a reactive trigger.

2. **LLM-Cognitive Scheduling.** We propose a three-layer scheduling architecture: deterministic timers (infrastructure), LLM-powered result interpretation (per-check semantic evaluation), and LLM-powered schedule review (periodic meta-reasoning about monitoring patterns). Deterministic code is substrate; LLM is intelligence.

3. **Concern-Based Multi-Tasking.** We formalize an agent's concurrent responsibilities as typed "concerns" — Foreground (active conversation), Monitor (periodic observation), Alarm (one-shot future event), Recurring (cron-scheduled task), Parked (suspended operation awaiting async result), and SelfWork (productive idle-time activity). Each concern executes independently with priority scheduling.

4. **The Enabling Framework Principle.** We articulate a design philosophy for LLM agent frameworks: never hardcode intelligence that could be delegated to the LLM. Deterministic algorithms become fallbacks, not primary logic. Prompts present information, not rigid instructions. The framework scales with model capability by design.

5. **Volition.** We introduce an initiative loop — a periodic cognitive process where the agent reasons about what it should be doing without user prompting. Volition enables proactive concern creation (monitoring error logs after a risky merge), self-directed state transitions (choosing to sleep for self-improvement), and autonomous concern lifecycle management (retiring stale monitors). Volition is not a separate system — it is one more concern type executing through the same infrastructure, with bounded guardrails preventing runaway behavior.

We implement Perpetuum in TEMM1E, a production Rust AI agent runtime with 15 crates, 1300+ tests, and real users across Telegram, Discord, Slack, and WhatsApp. Perpetuum is not a research prototype — it is deployed infrastructure.

---

## 2. Related Work

### 2.1 Agent Scheduling Systems

**OpenClaw** [1] implements a CronService with three schedule types (at, every, cron) and a heartbeat runner for periodic autonomous execution. Jobs persist across restarts and can carry contextMessages from the creating session. However, OpenClaw treats scheduling as external infrastructure — the agent has no temporal awareness and cannot reason about its own schedule.

**Claude Code** [2] provides /schedule (cron-based remote agents) and /loop (interval-based polling) with up to 50 concurrent scheduled tasks. Tasks run on Anthropic's cloud infrastructure, decoupling execution from the user's machine. Like OpenClaw, scheduling is infrastructure, not cognition.

**LangGraph** [3] offers a CronClient for scheduled graph executions and demonstrates the concept of a "self-scheduling agent tool" where the agent can create its own cron jobs. This is the closest prior work to our self-scheduling contribution, but LangGraph's scheduling decisions are fixed once created — there is no adaptive review or temporal cognition.

**ChatGPT Scheduled Tasks** [6] provides consumer-facing scheduled AI execution with daily/weekly/monthly recurrence. The backend is infrastructure-managed (likely Kubernetes CronJobs). No adaptive scheduling or temporal reasoning.

### 2.2 Continuous Agent Loops

**BabyAGI** [4] pioneered the perpetual task loop: execute → create new tasks → prioritize → repeat. The agent runs continuously but has no time awareness, no scheduling primitives, and no ability to sleep or throttle. It generates its own work infinitely.

**AutoGPT** [5] offers "continuous mode" that bypasses user authorization for each step. Community proposals for time delays between executions and pause/save/resume capabilities were never implemented. The loop runs as fast as the LLM responds.

### 2.3 Sleep-Time Computation

**Letta** (formerly MemGPT) [7] introduces sleep-time agents: background processes that manage memory during idle periods. The primary agent handles conversation while a sleep-time agent compacts memory, manages archives, and enriches context. The key insight — decoupling memory management from conversation latency — directly informs our productive sleep architecture.

**"Let Them Sleep"** (McCrae, 2025) [8] proposes sleep as model adaptation: during "day" the agent accumulates episodic memories; during "sleep" a background pipeline performs parameter-efficient fine-tuning (LoRA/adapters). The frozen base model plus trainable overlay parallels our Dream state integration with Eigen-Tune distillation.

### 2.4 Durable Execution

**Temporal.io** [9] enables workflows to sleep for arbitrary durations (`workflow.sleep(timedelta(days=365))`) without consuming worker resources. The server persists workflow state and resumes workers when timers expire. This "durable sleep" concept inspires our task parking mechanism, adapted from workflow orchestration to LLM agent tool loops.

### 2.5 Temporal Awareness in Agents

**Emergence of Adaptive Circadian Rhythms in RL Agents** (ICML 2023) [10] demonstrates that reinforcement learning agents deployed in environments with periodic variation spontaneously develop endogenous circadian-like rhythms. These rhythms are entrainable and adaptive. Our work extends this finding from RL to LLM agents, using explicit temporal context injection rather than emergent rhythm development.

### 2.6 Positioning

No existing system combines temporal cognition, LLM-cognitive scheduling, concurrent concern management, productive sleep, and the enabling framework principle. Each prior work solves one piece. Perpetuum is the synthesis.

---

## 3. The Perpetuum Framework

### 3.1 Design Philosophy: The Enabling Framework

The core principle governing Perpetuum's architecture:

> **The framework provides infrastructure. The LLM provides intelligence. The line between them is clear and inviolable.**

Infrastructure is what LLMs cannot do: count time, persist state to disk, spawn concurrent OS threads, fire timer interrupts, make HTTP requests. These are implemented as deterministic Rust code.

Intelligence is what LLMs excel at: judging relevance, assessing urgency, recognizing patterns, understanding intent, making contextual decisions. These are delegated to the LLM via structured prompts.

This separation has a critical consequence: **the framework's effective intelligence scales with the underlying model.** When a user upgrades from a 7B parameter model to a 70B model, every scheduling decision, every monitoring interpretation, every self-improvement activity gets smarter — without changing a line of framework code.

We formalize this as three properties:

**P1: No Cognitive Ceilings.** The framework never caps intelligence with hardcoded heuristics. A formula like `if event_density < 0.1: slow_down()` is a ceiling — it limits the system to the developer's foresight. Instead, the framework presents data and lets the LLM decide.

**P2: Open-Ended Prompts.** Context injections present information, not rigid instructions. "Here is the monitoring pattern over the last 24 hours" rather than "Reduce frequency if fewer than 2 events in the last 10 checks." A more capable model extracts more nuance from the same information.

**P3: Graceful Degradation.** When the LLM is unavailable (provider outage, rate limiting), deterministic fallbacks maintain basic operation. The fallback is the brainstem — it keeps the system alive. But it is never the primary intelligence layer.

### 3.2 Entity State Machine

Perpetuum models the agent as a persistent entity with four states representing proactive choices:

**Active:** Processing user requests, executing tasks, running tools. All concerns operational. This is the primary state during user interaction.

**Idle:** No active foreground tasks. Monitors and alarms continue running independently. The entity is immediately responsive to any trigger.

**Sleep:** The entity proactively invests idle time in self-improvement. Activities include memory consolidation (compacting persistent memory, pruning stale sessions), failure analysis (LLM reviews recent errors and extracts patterns), log introspection (LLM reviews interaction history for learnings), and blueprint refinement (updating procedural memory success weights).

**Dream:** Extended self-improvement via model distillation. When sufficient training data has accumulated, the entity triggers Eigen-Tune — a closed-loop distillation system that fine-tunes local models from cloud-generated (request, response) pairs.

State transitions are proactive, not forced. The entity is always instantly available — any wake trigger (user message, alarm, monitor detection) transitions to Active within 100ms regardless of current state. Sleep and Dream are interruptible without data loss.

### 3.3 Temporal Cognition

We introduce temporal cognition as a structured context injection prepended to LLM calls:

```
[Temporal Awareness]
Time: Monday 3:47 PM PST | Uptime: 14h32m | Idle: 6h23m
State: sleep → active (woke for this message)
Active: 2 monitors (reddit/claudecode @5m, facebook/page @3m)
         1 alarm (wake-user at 6:00 AM)
Parked: 1 task (waiting for github API, parked 3m ago)
Next event: facebook check in 1m47s
User pattern: typically active 9AM-11PM PST (32% likely active now)
```

This context provides the LLM with:
- **Absolute time** (current datetime in user's timezone)
- **Relative time** (uptime, idle duration, time since events)
- **Operational state** (current concerns, parked tasks, next scheduled event)
- **Learned patterns** (user activity probability based on historical hourly data)

The temporal context is *information*, not *instruction*. Per property P2, we do not tell the LLM what to do with this information. A capable model might notice "it's 3 AM and the user is 32% likely active" and decide to batch notifications for morning delivery. A simpler model might ignore the activity probability but still benefit from knowing the time. The framework enables both.

The temporal context is generated by the **Chronos** component, which maintains an internal clock, tracks interaction history, and computes derived metrics (idle duration, user activity probability from a 168-entry hourly circular buffer covering one week).

### 3.4 LLM-Cognitive Scheduling

We propose a three-layer scheduling architecture that cleanly separates infrastructure from intelligence:

**Layer 1: Deterministic Timing (Infrastructure).** A hierarchical timing wheel fires concern triggers at scheduled intervals. This is pure code: O(1) insertion and firing, 1-second tick resolution, cron expression evaluation with timezone support. The timing wheel never makes cognitive decisions — it counts seconds and fires events.

**Layer 2: Result Interpretation (LLM Intelligence).** When a monitoring check detects new content, the LLM evaluates it against the stored user intent:

```
Monitoring: "{monitor_name}"
User's intent: "{original_request}"
New content: {raw_content}
Previous result: {last_result}

Is this relevant to the user's intent?
Rate urgency: low / medium / high / critical.
Should the user be notified? If yes, summarize concisely.
```

This enables semantic filtering that no deterministic algorithm can match. The LLM distinguishes "critical security advisory" from "casual meme" because it understands the content and the user's intent. A smarter model makes finer distinctions.

**Layer 3: Schedule Review (LLM Intelligence).** Periodically (every N checks), the LLM reviews the monitoring pattern:

```
Monitor: "{name}" | Duration: {days_active} days | Intent: "{user_intent}"
Last {N} results: {history}
Current interval: {interval}
Temporal context: {temporal_block}

Should the frequency change? Should it vary by time of day?
Is this monitor still serving its purpose?
```

The LLM can produce scheduling decisions that no formula replicates:
- "Activity concentrates during US business hours — check every 2 minutes 9AM-6PM, every 20 minutes overnight"
- "This topic peaked 4 days ago. Recommend asking the user if monitoring is still needed"
- "Sudden spike detected — temporarily increase frequency for the next 2 hours"

**Fallback (Infrastructure).** When the LLM is unavailable, a deterministic fallback maintains operation: exponential backoff when no events detected, reset on detection. This ensures monitoring never stops, even if interpretation quality degrades temporarily.

### 3.5 Concern-Based Multi-Tasking

We formalize an agent's concurrent responsibilities as typed concerns:

| Concern Type | Description | Lifecycle |
|-------------|-------------|-----------|
| **Foreground** | Active user conversation | Created on message, completed on response |
| **Monitor** | Periodic observation with change detection | Persistent until cancelled |
| **Alarm** | One-shot future event | Fires once, then removed |
| **Recurring** | Cron-scheduled repeating task | Fires on schedule, persistent |
| **Parked** | Suspended task awaiting async result | Resumes on condition, then completes |
| **SelfWork** | Productive idle-time activity | Created during Sleep/Dream, runs to completion |

Each concern executes as an independent asynchronous task. Foreground has highest priority. Monitors, alarms, and recurring tasks continue regardless of the entity's state — they are independent of the "main brain." A monitor detecting something urgent queues a notification without interrupting the active foreground conversation.

Concerns persist to SQLite. On process restart, all active concerns reload and resume. Alarms that should have fired during downtime are delivered immediately with a lateness indicator.

### 3.6 Productive Sleep

When the entity transitions to Sleep, it engages in structured self-improvement:

**Memory Consolidation** (no LLM): Compact persistent memory store, merge duplicate entries, prune sessions older than retention threshold, re-index for search performance.

**Failure Analysis** (LLM): Review recent error logs and extract recurring patterns. Update operational blueprints with failure modes and mitigations.

**Log Introspection** (LLM): Review recent interaction history. Extract learnings about user preferences, common request patterns, and effective strategies.

**Blueprint Refinement** (no LLM): Update blueprint success/failure statistics. Adjust strategy weights based on observed outcomes.

Sleep-time LLM usage is a deliberate investment: the entity spends tokens to become more effective. This is consistent with the enabling framework principle — the LLM provides the intelligence for self-improvement, the framework provides the time and context.

When sufficient training data accumulates, Sleep transitions to Dream: the entity triggers Eigen-Tune distillation, fine-tuning local models from cloud-generated high-quality (request, response) pairs using LoRA/QLoRA.

---

## 4. Implementation

Perpetuum is implemented as `temm1e-perpetuum`, a Rust crate within the TEMM1E workspace (20 crates, 1935 tests). Key implementation details:

**Runtime:** Tokio async runtime. Each concern is a `tokio::spawn`ed task. Concerns are isolated with `catch_unwind` to prevent cascading panics (per TEMM1E's resilience architecture — a prior production incident where a UTF-8 boundary panic killed the entire process).

**Timing:** Hierarchical timing wheel with 1-second resolution, implemented as bucketed vectors (second/minute/hour/day). Cron expression parsing via the `cron` crate with `chrono-tz` for timezone-aware evaluation.

**Persistence:** SQLite with WAL mode. Concern state, monitor history, transition logs, and parked task checkpoints persist across restarts. Writes are batched (flush every 5 seconds) to avoid contention under concurrent monitors.

**Integration:** Perpetuum replaces the bare HeartbeatRunner in TEMM1E's main loop. Existing heartbeat configurations are automatically converted to Recurring concerns for backward compatibility. TemporalContext is injected into the system prompt builder in the agent runtime. Monitor tools (create_alarm, create_monitor, etc.) are registered via the existing ToolDeclarations system.

**Observability:** Every concern lifecycle event, state transition, LLM evaluation, and timer firing emits structured tracing spans. This is critical for debugging concurrent behavior and was built before any scheduling logic (Phase 0).

**Resilience (24/7/365 operation):** Perpetuum is designed for indefinite autonomous operation. Key resilience measures: (1) Every concern dispatch is wrapped in `catch_unwind` — a panicking monitor cannot cascade to other concerns or the main agent. (2) LLM calls have 60-second timeouts — a hung provider cannot block Perpetuum indefinitely. (3) The Pulse timer loop auto-restarts on panic with a 5-second backoff — scheduling never permanently dies. (4) Atomic concern claiming (state transition from 'active' to 'firing') prevents duplicate fires from race conditions. (5) Per-concern error budgets disable concerns after 3 consecutive failures. (6) Perpetuum init failure does not crash the main process — Tem continues without scheduling.

---

## 5. Evaluation

### 5.1 Temporal Cognition Ablation

**Setup:** Two TEMM1E instances processing identical scheduling requests over 7 days. Instance A receives full TemporalContext injection. Instance B receives no temporal context.

**Metrics:**
- *Notification timing quality:* Does the agent avoid notifications during likely-inactive hours? (Measured as fraction of notifications sent during user's historical low-activity periods.)
- *Schedule adaptation:* Does the agent proactively suggest schedule changes based on observed patterns?
- *Contextual awareness:* Does the agent reference temporal state in its reasoning? (Measured via response annotation.)

**Hypothesis:** Temporal cognition yields measurably better scheduling decisions and more contextually appropriate notifications.

### 5.2 LLM-Cognitive vs. Fixed Scheduling

**Setup:** 10 diverse monitoring targets (high-activity Reddit subs, low-activity GitHub repos, variable-frequency APIs) monitored for 14 days with three configurations: (A) fixed 5-minute intervals with raw content passthrough, (B) fixed intervals with LLM interpretation, (C) full cognitive scheduling (LLM interpretation + periodic review).

**Metrics:**
- *Total LLM calls* (cost)
- *Relevant notification rate:* fraction of notifications the user marked as useful
- *Detection latency:* time from content appearance to user notification
- *False-positive rate:* notifications for irrelevant changes

**Hypothesis:** Configuration C achieves the highest relevant notification rate with comparable detection latency to A, while reducing false positives by >50% through semantic interpretation.

### 5.3 Productive Sleep Impact

**Setup:** TEMM1E instance running for 30 days. Weekly measurements of:
- Memory search relevance (recall@10 for test queries)
- Blueprint success rate (task completion rate for blueprint-matched tasks)
- Response consistency (LLM-as-judge scoring on a fixed evaluation set)

**Control:** Identical configuration with sleep disabled (entity idles instead of doing self-work).

**Hypothesis:** Productive sleep yields measurable improvement in memory quality and blueprint effectiveness over 30 days.

### 5.4 Concurrent Concern Throughput

**Setup:** Mixed workloads: active conversation + N background monitors (N = 0, 5, 10, 20, 50) + M pending alarms.

**Metrics:**
- Foreground response latency (p50, p95, p99)
- Monitor check timing accuracy (scheduled vs. actual fire time)
- Wake latency (time from trigger to Active state transition)

**Hypothesis:** Foreground latency remains constant regardless of background concern count, confirming that independent tokio tasks provide true concurrency isolation.

---

## 6. Discussion

### 6.1 The Enabling Framework as a Design Principle

The enabling framework principle has implications beyond Perpetuum. Most AI agent frameworks bake intelligence into code: intent classifiers, routing heuristics, scheduling formulas, relevance filters. Each hardcoded decision is a ceiling — it can never be smarter than the developer who wrote it. The enabling framework principle inverts this: code provides infrastructure, LLMs provide intelligence. As models improve (and the trajectory is steep), the same framework produces better results.

This principle is particularly powerful for scheduling decisions. Deciding "how often should I check this?" is not a computational problem — it's a judgment call that depends on content semantics, user intent, temporal patterns, and cross-concern priorities. These are precisely the capabilities where LLMs improve with each generation.

We believe this principle applies broadly to agent framework design and encourage future work to formalize the infrastructure/intelligence boundary.

### 6.2 Single Model Constraint

Perpetuum deliberately uses a single model for all operations (conversation, monitoring interpretation, schedule review, self-improvement). This avoids model routing complexity (which model for which task?) and ensures predictable behavior. When the user upgrades their model, all capabilities improve uniformly. Future work could explore tiered models (cheap for monitoring, capable for scheduling review) once model routing becomes more standardized.

### 6.3 Cost Considerations

LLM-cognitive scheduling introduces per-check and per-review token costs absent from deterministic scheduling. For a monitor checking every 5 minutes with 30% change rate, this is approximately $0.15/day at current pricing. This cost is a deliberate investment in intelligence — the alternative (deterministic scheduling) saves $0.15/day but cannot distinguish "critical security advisory" from "casual meme." As model costs decrease (a consistent historical trend), the cost argument weakens while the intelligence argument strengthens.

### 6.4 Limitations

**Provider dependency:** LLM-cognitive scheduling degrades to deterministic fallback when the provider is unavailable. Extended outages reduce monitoring intelligence to basic change detection.

**Evaluation difficulty:** Measuring "scheduling quality" is inherently subjective. LLM-as-judge introduces known biases. Human evaluation is expensive at scale.

**Single-process architecture:** Perpetuum runs within a single TEMM1E process. Concerns are tokio tasks, not distributed workers. Horizontal scaling requires process-level orchestration not addressed in this work.

### 3.7 Volition: The Initiative Loop

Volition is the agency layer — a periodic cognitive process where the agent reasons about what it should be doing without user prompting. Architecturally, Volition is one more concern type in the Cortex:

```
Concern::Initiative { interval, last_run, event_triggers }
```

When the initiative fires (periodically or after significant events), the agent receives a comprehensive state snapshot — temporal context, active concerns, recent monitor results, conversation history, error patterns — and outputs structured decisions: create/modify/cancel concerns, queue proactive notifications, set self-work priorities, write internal notes.

**Volition fires on two triggers:**
- **Periodic:** Every N minutes (configurable, default 15 minutes)
- **Event-driven:** After a conversation ends, after a monitor detects something significant, after an error occurs, or on transition to Idle

**Bounded autonomy.** Volition operates within guardrails to prevent runaway behavior:
- Maximum actions per cycle (default: 2 concern CRUD operations)
- Global concern count cap (shared with user-created concerns)
- All Volition-created concerns are tagged with `source: "volition"` for user visibility
- Volition cannot create additional Initiative concerns (no self-replication)
- Users can inspect and cancel any Volition-created concern

**Why this matters:** Without Volition, the agent is a sophisticated reactive system — it does what it's told, smartly. With Volition, the agent becomes an entity that notices things, forms intentions, and acts on them. After reviewing a risky PR, Volition might decide to monitor error logs post-merge. After noticing a monitor has been empty for a week, Volition might suggest retiring it. After detecting 3 similar errors, Volition might prioritize failure analysis.

Volition is the highest expression of the enabling framework principle: we give the LLM a regular opportunity to think and act autonomously, bounded by infrastructure guardrails. A smarter model produces better initiative decisions. The framework doesn't change.

### 6.5 Future Work: Tem Simulation

With Volition, Perpetuum delivers both infrastructure (body) and agency (mind). **Tem Simulation** extends further into a sandboxed autonomous environment where the agent runs experiments, tests hypotheses, and practices strategies during extended idle periods. The two-layer separation (autonomic infrastructure vs. executive agency) mirrors the biological distinction between autonomic function and executive function — and is itself a design pattern worth formalizing for future agent architectures.

---

## 7. Conclusion

We presented Perpetuum, a framework for perpetual, time-aware LLM agents. Our core contributions — temporal cognition, LLM-cognitive scheduling, concern-based multi-tasking, and the enabling framework principle — address the fundamental limitation of request-response agent architectures. Perpetuum transforms agents from functions called on a schedule into persistent entities that reason about time, manage concurrent responsibilities, and invest idle time in self-improvement.

The enabling framework principle — providing infrastructure while delegating intelligence — ensures that Perpetuum's capabilities grow with model improvements. This positions the framework not for today's models, but for the trajectory of LLM development: the smarter the model, the smarter the agent, without changing the framework.

Perpetuum is implemented in TEMM1E, a production Rust agent runtime, and is available as open-source software.

---

## References

[1] OpenClaw. "Cron Jobs & Automation." OpenClaw Documentation. 2025.

[2] Anthropic. "Scheduled Tasks." Claude Code Documentation. 2026.

[3] LangChain. "Cron Jobs." LangGraph Platform Documentation. 2025.

[4] Y. Nakajima. "Task-Driven Autonomous Agent (BabyAGI)." 2023.

[5] Significant Gravitas. "AutoGPT: An Autonomous GPT-4 Experiment." 2023.

[6] OpenAI. "Introducing ChatGPT Agent." 2025.

[7] C. Packer et al. "MemGPT: Towards LLMs as Operating Systems." arXiv:2310.08560, 2023.

[8] McCrae Tech. "Let Them Sleep: Adaptive LLM Agents via a Sleep Cycle." 2025.

[9] Temporal Technologies. "Orchestrating Ambient Agents with Temporal." 2025.

[10] "Emergence of Adaptive Circadian Rhythms in Deep Reinforcement Learning." ICML, 2023. arXiv:2307.12143.

---

## Appendix A: Publication Strategy

### Recommended Path

1. **arXiv preprint** (immediate) — establish priority on temporal cognition + enabling framework
2. **AAAI or NeurIPS Systems track** — full paper with evaluation results
3. **AAMAS** (Autonomous Agents) — strong fit for concern-based multi-tasking and self-scheduling

### Strongest Framing

Lead with **"The Enabling Framework Principle"** as the meta-contribution. This is the most generalizable and thought-provoking claim — it applies beyond Perpetuum to all agent framework design. Temporal cognition and LLM-cognitive scheduling are the concrete instantiations that demonstrate the principle.

### Differentiation

Most agent papers are either (a) "we built a system" (engineering, hard to publish at top venues) or (b) "we found a technique" (narrow contribution). Perpetuum bridges both: it articulates a design principle (enabling framework), implements concrete novel techniques (temporal cognition, cognitive scheduling), and validates them in a production system (TEMM1E). This combination is rare and compelling for reviewers.

### Risk: "Just Engineering"

The enabling framework principle elevates this beyond engineering. It's a *prescriptive claim about how agent frameworks should be designed* — testable, falsifiable, and applicable beyond our specific system. The temporal cognition ablation provides the scientific evidence.
