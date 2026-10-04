---
name: strait-performance-check
description: Benchmark or assess performance-sensitive Strait changes against latency budgets using comparable baselines, representative loads, and targeted profiling. Use for first hot-path implementations, optimizations, and measured-path dependency or configuration changes.
---

# Strait Performance Check

Evaluate whether evidence supports low-latency acceptance; use profiles to explain
costs and benchmarks to assess effects. Do not infer production readiness from a
microbenchmark or create numerical limits without an agreed contract.

## Establish the Question

Locate the repository root. Read `AGENTS.md`, `docs/engineering.md`, and the relevant
sections of `docs/latency.md` and `docs/testing.md`. Identify affected hot paths,
products, first-implementation gates, reference environment, operating workloads,
and established budgets. Honor review-only versus authorized implementation scope.

Find existing harnesses, fixtures/seeds, documented commands, retained reports, and
the requested baseline/candidate. Inspect benchmark validity, not just command success.
Use an isolated baseline checkout if needed; preserve user edits and never reset the
working tree. Resolve comparison scope from the task; do not invent a clean baseline
by dropping unrelated dirty-tree changes. Label unavailable comparisons explicitly.

## Measure and Diagnose

Choose the smallest representative set covering affected components and integrated
flows. Use the documented release configuration, independent arrival schedule for
load tests, timing boundaries, message/view correlation, and delivery counters.
Check optimized-away work, fixture/setup cost, closed-loop omission, instrumentation
cost, warmup, sample counts, and histogram resolution. Criterion iteration averages
are not receipt-to-view p99/p99.9.

Run applicable correctness checks first. The first hot-path implementation requires
a runnable benchmark and initial CPU/allocation profile, not placeholder evidence.
Repeat comparable baseline/candidate measurements; alternate runs where practical.
Cover relevant bursts/update sizes/instrument counts and slow consumers. Compare both
Binance products for shared changes. Track failures, stale/invalid states, backlog,
consumer-skipped views, and offered versus processed load so fast survivors cannot
hide overload. Keep startup/recovery separate and measure their impact on live books.

Apply the resource-cost evidence matrix in `docs/testing.md`. Include strings/copies,
allocation/reallocation/free behavior, live memory, CPU time/headroom/idle cost, and
syscall/kernel behavior for affected paths. Capture setup versus warmed steady state,
new levels, and cleanup separately; include transitive crate/runtime costs in integrated
results. Record unavailable counters explicitly and avoid equating syscall count or
process CPU percentage with latency acceptance.

Choose targeted CPU, allocation, syscall, or scheduling/queue profiles for the question.
Record exact commands and versions; consult official tool documentation when needed.
Do not install tools globally, alter machine settings, or upload results outside the
authorized scope. Measure acceptance without the profiler and record instrumentation
configuration/overhead. Absent profilers or reference hardware are explicit gaps,
not reasons to fabricate results or claim equivalent environment acceptance.

Assess relevant long-running/fault evidence from `docs/security.md`: memory plateau,
task/socket retention, repeated recovery interference, telemetry overhead, and tail
latency drift. Record duration, offered load, thresholds, and absent harnesses.

Apply `docs/connections.md` capacity acceptance for connection-related changes:
per-connection and aggregate messages/bytes/levels, sustained/burst scaling, control
delay, noisy neighbors, mass reconnects, and time to usable fresh books. Dozens of
messages/s alone does not prove capacity. Check task/socket/timer/memory plateaus.

## Decision and Report

Record commits, fixture/seed, commands, hardware/build/runtime/queue configuration,
repeated-run variation, sample counts, relevant stage and integrated distributions,
resource/delivery counts, and profile locations. Compare agreed absolute limits and
relative tolerances; small differences inside run noise are inconclusive. Do not add
stage percentiles or infer global acceptance from faster local averages.

Give an evidence status: compliant for the specified tested workload, measured
regression, evidence incomplete, or inconclusive. State coverage and excluded cases;
no status certifies production readiness beyond the documented evidence. Suggest a
specific next measurement or change for failures. Authorized optimization work fixes
in-scope problems and repeats relevant checks; review-only work reports findings.
Never revise a budget silently or add unrelated optimization work to manufacture a pass.
