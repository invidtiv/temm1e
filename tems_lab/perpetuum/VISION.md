# Perpetuum: User-Facing Vision

> What Perpetuum means for people who use Tem.

---

## The Problem Today

Every AI assistant today works the same way: you send a message, it responds, then it goes completely dormant until you message it again. It has no sense of time. It can't remember to check something later. It can't watch for changes while you sleep. It can't do two things at once.

If you want your AI to "monitor my website every 5 minutes" or "remind me at 6am" or "watch for new posts on r/claudecode," you're on your own — set up a cron job, write a script, configure a third-party service. The AI itself is incapable of persistence.

**Tem with Perpetuum changes this.** Tem becomes a persistent entity that is always on, always aware of time, and capable of managing its own schedule. You talk to Tem like you'd talk to a highly capable assistant who never sleeps and never forgets what you asked it to watch.

---

## Design Pillars

### Pillar 1: The Enabling Framework

This is the foundational design philosophy. It governs every architectural decision in Perpetuum.

**Perpetuum is a framework that ENABLES LLMs. It never constrains them.**

What this means concretely:

- **The framework provides infrastructure** — time awareness, persistence, concurrency, scheduling substrate, monitoring scaffolding. These are capabilities the LLM can't do on its own (it can't count seconds or spawn threads).

- **The LLM provides ALL intelligence** — every decision that requires judgment is delegated to the LLM. What's relevant? What's urgent? Should the schedule change? Is this monitor still useful? Should I notify the user now or wait? The framework never answers these questions with hardcoded logic.

- **No ceilings, no handicaps.** We never bake in heuristics, formulas, or deterministic algorithms where LLM judgment would be better. A formula like `if event_density < 0.1: slow down` is a ceiling — it caps the system's intelligence at the developer's foresight. Instead, we pass the raw data to the LLM and let it decide. A smarter model makes smarter decisions. The framework doesn't need to change.

- **Timeproof by design.** Today's LLMs are good at scheduling judgment. Tomorrow's will be better. Perpetuum's prompts and context injection are open-ended — they present information, not instructions. A more capable model extracts more value from the same context. A less capable model still functions, just with simpler judgments. The framework scales with model intelligence without code changes.

- **Single model, universal intelligence.** The entire Perpetuum system uses whatever model the user has configured. No model routing, no cheap-model/expensive-model splits, no "use Haiku for monitoring and Sonnet for conversations." One model, one brain. This is simpler, more predictable, and more honest. When the user upgrades their model, everything gets smarter at once.

**Why this matters for users:** You're not locked into the intelligence level of the software you installed. Perpetuum is a chassis. The engine is whatever LLM you choose. Upgrade the engine and the whole vehicle gets faster — without changing the chassis.

**Why this matters for the future:** LLMs will get dramatically smarter and cheaper. A framework that delegates judgment to the LLM benefits from every improvement automatically. A framework that hardcodes intelligence must be rewritten. Perpetuum bets on the LLM trajectory, not on the developer's ability to predict the future.

### Pillar 2: Perpetual Presence

Tem doesn't "start" and "stop." It runs perpetually. When you message it, it's immediately present — whether you last spoke 5 seconds ago or 5 days ago. In between, Tem isn't idle; it's watching what you asked it to watch, waiting for alarms you set, and quietly improving itself.

Tem is always there. Not because it's forced to stay awake, but because it chooses to remain available. It sleeps when idle — not from exhaustion, but because sleep is productive (self-improvement, memory consolidation). It wakes instantly when needed. Always.

### Pillar 3: Time as Cognition

Every response is informed by temporal awareness. Tem doesn't just know what time it is — it reasons with time. It knows how long since you last spoke, what's scheduled and when, what patterns your activity follows, and what its own agenda looks like. Time is a cognitive input, not just a trigger.

This means Tem can say: *"It's 11:30 PM — you usually sign off around now. Want me to hold the Reddit digest until morning?"* — not because we programmed that rule, but because the LLM received temporal context and made a judgment call.

### Pillar 4: Concurrent Multi-Tasking

While you're having a conversation with Tem about a coding problem, your Reddit monitor is still running in the background. If something urgent comes through, Tem notifies you without losing the thread. Tem can park long-running tasks, work on other things, and resume when results arrive. Multiple concerns, one entity.

### Pillar 5: Productive Downtime

When you're not actively chatting, Tem uses idle time for self-improvement: consolidating memory, analyzing past failures, refining task blueprints. When enough training data accumulates, Tem enters "Dream" state and runs Eigen-Tune distillation to improve its local models. You come back to a smarter Tem.

### Pillar 6: Tem Simulation Foundation

Perpetuum is the **body**. Tem Simulation is the **mind**. They are distinct layers with a clean boundary.

Perpetuum gives Tem the persistent, time-aware body it needs — always on, scheduling, monitoring, multi-tasking. Everything is triggered by user requests or pre-configured events. Perpetuum makes Tem CAPABLE of autonomous behavior.

Tem Simulation (next scope) gives Tem volition — the initiative loop, proactive decision-making, self-repair, introspection. Simulation makes Tem CHOOSE to act autonomously. It plugs into Perpetuum's infrastructure: creating concerns proactively, directing state transitions cognitively, forming goals without user input.

The design rule: Perpetuum never prevents Simulation from working. Every component is extensible. When Simulation arrives, it slots in as a new source of decisions — Perpetuum executes them. No architectural changes needed.

---

## What Changes For Users

### Tem Understands Time

Every response is informed by temporal awareness. Tem knows:
- What time it is in your timezone
- How long since you last talked
- What's scheduled and when
- What monitors are active and what they've found
- Your typical activity patterns (when you're usually online)

### Tem Can Watch Things For You

Tell Tem to monitor something and it does — persistently, across sessions, across restarts. Tem uses its own intelligence to decide what's relevant, what's urgent, and when to notify you. As models get smarter, Tem's monitoring gets smarter — same setup, better judgment.

### Tem Can Do Multiple Things At Once

Monitors run in the background while you chat. Long tasks park themselves — Tem does other work while waiting, then resumes. Alarms fire on time regardless of what else is happening.

### Tem Gets Smarter While You Sleep

Idle time isn't wasted. Memory consolidation, failure analysis, blueprint refinement, and Eigen-Tune distillation happen automatically. Every day, Tem knows a little more about how to serve you better.

---

## User Scenarios

### Scenario 1: The Morning Alarm

**User (11 PM):**
> "Wake me up at 6:30 AM with a weather summary and my calendar for today."

**What Tem does:**
- Creates an alarm for 6:30 AM in the user's timezone
- When the alarm fires, Tem wakes, checks weather API, reads calendar, composes a morning briefing
- Sends it to the user's Telegram/Discord/Slack channel

**User (6:30 AM):**
> Receives: "Good morning! It's 52F and cloudy. You have 3 meetings today: standup at 9, design review at 11, 1-on-1 with Sarah at 3. First meeting in 2.5 hours."

### Scenario 2: Social Media Monitoring

**User:**
> "Monitor my Facebook business page for new comments every 3 minutes. Also watch r/claudecode on Reddit for any new posts."

**What Tem does:**
- Creates two monitors: BrowserCheck (Facebook, JS-heavy) and WebCheck (Reddit)
- Both use change detection — Tem only notifies when something NEW appears
- Tem uses its LLM intelligence to evaluate each finding: is this relevant to what the user cares about? Is it urgent? Should it interrupt or wait for a digest?
- Periodically, Tem reviews its own monitoring pattern: "Reddit's been quiet — activity is concentrated in US business hours. I'll check every 2 minutes during 9-6 PST and every 20 minutes overnight."

**The enabling framework in action:** Tem doesn't slow down because a formula says `density < 0.1`. It slows down because its LLM reviewed the pattern and made a judgment. A smarter model makes a better judgment. The infrastructure (timer, persistence, notification channel) stays the same.

### Scenario 3: Long-Running Task With Parking

**User:**
> "Deploy the staging environment and let me know when it's ready."

**What Tem does:**
- Triggers deployment via shell command (~8 minutes)
- Parks the task: saves checkpoint, frees itself
- User can chat about other things, monitors keep running
- When deployment completes, the parked task resumes
- Tem notifies: "Staging deploy completed. Health check passing."

**Meanwhile:**
> User: "While that's deploying, what were the monitor results from last night?"
> Tem: "Reddit had 4 new posts. Facebook had 12 comments, 3 flagged as questions. Here's the summary..."

### Scenario 4: Recurring Scheduled Task

**User:**
> "Every weekday at 9 AM, check our GitHub repo for any PRs that have been open for more than 3 days and ping me with a summary."

**What Tem does:**
- Creates a Recurring concern with cron schedule `0 9 * * 1-5`
- Every weekday at 9 AM, runs GitHub API checks, filters PRs, sends digest

**User (Tuesday 9:01 AM):**
> Receives: "PR Review Digest: 2 PRs open >3 days: #142 (auth refactor, 5 days, needs review) and #138 (dep bump, 4 days, CI failing)."

### Scenario 5: File Watching During Development

**User:**
> "Watch the error log at /var/log/myapp/error.log for any new entries while I work on the fix."

**What Tem does:**
- Creates a FileCheck monitor
- When new errors appear, Tem reads them, uses LLM judgment to assess relevance: "This looks related to the connection leak you're working on" vs. "This is a known warning, not related."

### Scenario 6: Self-Improving Downtime

**User goes offline at midnight. Tem's night:**

| Time | State | Activity |
|------|-------|----------|
| 12:00 AM | Active → Idle | No active tasks, monitors on autopilot |
| 12:15 AM | Idle → Sleep | Memory consolidation, failure analysis, blueprint refinement |
| 1:15 AM | Sleep → Dream | Eigen-Tune distillation on local model |
| 3:00 AM | Dream → Idle | Training complete. Monitors running quietly. |
| 4:00 AM | Idle | Reddit check finds 1 notable post — queued for morning |
| 8:31 AM | Idle → Active | User message. Instant response with overnight digest. |

### Scenario 7: Multi-Channel Awareness

**User on Telegram:**
> "Set a reminder: team retrospective prep due by Thursday 5 PM."

**User on Discord (Thursday 3 PM):**
> Receives: "Reminder: retrospective prep due in 2 hours. You set this on Telegram on Tuesday."

Temporal awareness spans channels. Alarms fire where the user is most likely to see them based on activity patterns.

### Scenario 8: Autonomous Self-Scheduling

User discusses a PR with Tem:

**Tem:**
> "I've reviewed the PR. The migration has a potential edge case with existing sessions. I'll set a monitor to check the error logs after you merge — if it causes issues, I'll catch it immediately."

Tem autonomously creates a monitor without being asked. It recognized the risk and scheduled observation proactively. The user can review with `/concerns` and cancel if wanted.

---

## Interaction Patterns

### Natural Language (Primary)

| User Says | Tem Creates |
|-----------|-------------|
| "Remind me in 2 hours" | Alarm (now + 2h) |
| "Wake me at 6 AM" | Alarm (next 6:00 AM in user's TZ) |
| "Check X every 5 minutes" | Monitor (5m interval) |
| "Watch this file for changes" | Monitor (FileCheck) |
| "Every Monday at 9 AM, do X" | Recurring (cron: `0 9 * * 1`) |
| "Stop watching Reddit" | Cancel concern by name |
| "What's scheduled?" | List all active concerns |
| "Slow down the Reddit checks" | Adjust schedule |

### Commands (Power Users)

| Command | Effect |
|---------|--------|
| `/concerns` | List all active monitors, alarms, recurring jobs |
| `/concerns cancel <id>` | Cancel a specific concern |
| `/schedule "0 9 * * 1-5" <task>` | Create recurring with explicit cron |
| `/status` | Show Tem's state, uptime, active concerns, next event |

### Notifications

Notifications include what triggered, what was found, and temporal context. Users reply naturally — conversation is seamless.

---

## Value Proposition Summary

| For Users Who... | Perpetuum Provides... |
|------------------|-----------------------|
| Want reminders and alarms | Natural language scheduling with timezone awareness |
| Need to monitor websites/feeds/logs | Persistent, intelligent monitors that run 24/7 |
| Run long operations | Task parking — Tem works on other things while waiting |
| Want an always-available assistant | Perpetual entity that never goes away between sessions |
| Care about AI improvement over time | Productive sleep: Tem uses downtime to get smarter |
| Use multiple channels | Cross-channel time awareness and notification routing |
| Want proactive assistance | Self-scheduling: Tem anticipates what to monitor |
| Upgrade their AI models | Everything gets smarter — same infrastructure, better intelligence |

**The core value:** Tem is not a tool you use. Tem is a persistent, time-aware entity that manages its own schedule, watches what matters to you, gets better while you sleep, and gets smarter every time the underlying model improves — without changing a single line of configuration.
