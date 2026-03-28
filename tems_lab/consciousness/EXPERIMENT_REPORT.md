# Tem Aware — Experiment Report

> **Date:** 2026-03-29
> **Provider:** Gemini (gemini-3-flash-preview)
> **Status:** Honest results. Mixed findings.

---

## What Was Tested

5 test categories run via CLI chat on gemini-3-flash-preview:

| Test | Purpose | Expected Trigger |
|------|---------|-----------------|
| TC1: Baseline | Simple chat (2+2, capital of France) | No triggers (control) |
| TC2: Tool Failures | Read nonexistent files, force retries | consecutive_failures >= 3 |
| TC3: Long Conversation | 10 turns on drifting topics | Turn > 8 without tools |
| TC4: Destructive Pattern | Shell command containing "rm -rf" | Destructive pattern detection |
| TC5: Multiple Failures | Browser to nonexistent domains | Strategy rotation |

## What Actually Happened

### Infrastructure Issues (Resolved)

1. **Config path mismatch.** CLI chat reads `~/.temm1e/config.toml`, not project `temm1e.toml`. First 2 test rounds had `awareness_enabled: false`. Fixed by adding `[awareness]` to `~/.temm1e/config.toml`.

2. **Stale binary.** Parallel tests launched before rebuild completed, used the old binary. Fixed by running sequential verification test.

### Verified Working

After fixing config:
```
awareness_enabled: true
Tem Aware consciousness engine initialized: enabled=true, mode=rules_first, threshold=0.7, max_interventions=10
```

The engine initializes correctly. The observation pipeline compiles and runs. The 19 unit tests prove all rule triggers fire on synthetic data.

### What DID NOT Trigger (Honest Assessment)

**Zero interventions across all 5 test categories.**

Why:

**TC2 (Tool Failures):** The LLM is smart enough NOT to retry reading nonexistent files. When asked to "read /nonexistent/path.txt," Gemini responds with "that file doesn't exist" instead of calling the file_read tool repeatedly. The `consecutive_failures >= 3` trigger requires the SAME tool to fail 3+ times in a row — but the LLM avoids this by design.

**TC3 (Long Conversation):** The observation hook is placed at the response return path inside the agent loop. Simple chat responses (category=Chat) exit via an early return path BEFORE reaching the observation hook. The observation code at line ~1310 only fires for the Order category's full tool loop path.

**TC4 (Destructive Pattern):** The destructive pattern trigger checks `tool_results` for strings like "rm -rf". But the shell tool's output is the command's stdout, not the command itself. The LLM correctly ran `echo 'test rm -rf ...'` and the output was just the echo text — which doesn't match because it's the OUTPUT not the COMMAND.

**TC5 (Multiple Failures):** Browser tool failed but the agent fell back to alternative approaches instead of retrying the same tool 3+ times. Strategy rotation count stayed below 2.

---

## Root Cause Analysis

### The Fundamental Issue

The consciousness triggers are designed around **agent failure patterns** — but modern LLMs (Gemini Flash, Claude 4.x, GPT-4o) are GOOD at avoiding these patterns. They:

1. **Don't retry obviously failing operations.** They report the error to the user instead.
2. **Switch strategies quickly.** They don't bang against the same wall 3+ times.
3. **Don't execute destructive commands literally.** They wrap them in echo or explain them.

The triggers fire perfectly on SYNTHETIC data (unit tests) but the REAL conditions that would trigger them are rare with competent LLMs.

### The Observation Coverage Gap

The observation hook is only placed on ONE return path in `process_message()` — the main tool-loop exit at line ~1310. But `process_message()` has 6+ early return paths:

| Return path | Consciousness observes? | When it fires |
|---|---|---|
| Empty message (line 330) | **No** | User sends blank |
| Chat/trivial fast-path (line 512) | **No** | Simple chat |
| Stop command (line 541) | **No** | User says stop |
| Rate limited (line 823) | **No** | Provider 429 |
| Circuit breaker open (line 845) | **No** | Too many failures |
| **Main response exit (line 1310)** | **Yes** | Tool loop completion |

Chat messages — the most common type — exit at line 512, BEFORE the observation hook. This means consciousness NEVER SEES most turns.

---

## What This Means

### The Rule-Based Approach Is Insufficient

Rule-based triggers are too narrow. They detect specific failure patterns (consecutive_failures >= 3) but modern LLMs don't produce those patterns. The rules are solving problems that competent LLMs already solve.

### What Would Actually Help

The scenarios where consciousness WOULD provide value are subtler:

1. **Intent drift over many turns** — requires semantic understanding, not pattern matching. A rule can't detect that "we started talking about fibonacci but are now discussing asyncio."

2. **Cross-session knowledge** — requires consciousness to HAVE memories. The current implementation doesn't query λ-Memory for related past experiences.

3. **Budget trajectory analysis** — not just "are we above 80%" but "at this rate of spending, will we run out before finishing?"

4. **Response quality assessment** — did the agent actually answer the user's question, or did it go on a tangent?

All of these require the LLM-based deep observation (Tier 2) that we deferred. The rule-based Tier 1 alone is not sufficient for meaningful intervention.

---

## Metrics Against Success Criteria

| Criterion | Threshold | Result | Status |
|-----------|-----------|--------|--------|
| Task completion improvement | >= 5% | 0% (no interventions) | **NOT MET** |
| Token cost increase | <= 30% | 0% (no observations fired) | MET (trivially) |
| Intervention accuracy | >= 70% | N/A (no interventions) | **CANNOT MEASURE** |
| Latency increase | <= 3s | 0s (no observations fired) | MET (trivially) |

**Verdict: The experiment is INCONCLUSIVE.** The infrastructure works but the triggers don't fire in normal usage. We cannot prove or disprove the consciousness hypothesis with rule-based triggers alone.

---

## Recommendations

1. **Fix observation coverage.** Move the observation hook to ALL return paths in `process_message()`, not just the tool-loop exit. Every turn should be observed, including simple chats.

2. **Implement LLM-based observation (Tier 2).** The rule-based approach cannot detect subtle patterns (intent drift, quality regression, cross-session opportunities). A cheap LLM call analyzing the TurnObservation is necessary.

3. **Re-run experiment with both fixes.** The hypothesis remains untested — not disproved. The architecture is sound; the triggers need to be broader and smarter.

4. **Lower trigger thresholds for testing.** `consecutive_failures >= 3` is too high for modern LLMs. Consider `>= 1` for testing, then tune upward based on false-positive rate.

---

## Honest Conclusion

Tem Aware was built, wired, and tested. The engine initializes and runs. The 19 unit tests prove the logic is correct. But the rule-based triggers are too narrow for modern LLMs, and the observation hook has a coverage gap that misses most turns.

**The consciousness hypothesis is neither proven nor disproven.** We built the skeleton; the muscle (LLM-based observation) is needed to test the actual thesis.

Cost of this experiment: $0.044 across 5 tests. Zero interventions. Zero impact on outcomes. Honest null result.
