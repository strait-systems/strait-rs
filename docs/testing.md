# Testing and Performance

Market data systems need deterministic tests for behavior that is difficult to reproduce reliably against live exchanges.

Strait will use recorded exchange messages to replay scenarios such as:

```text
snapshot
    ↓
incremental updates
    ↓
sequence gap
    ↓
connection loss
    ↓
reconnect
    ↓
resynchronization
    ↓
valid local book
```

Tests should cover both normal operation and failure paths.

Where possible, local book state will be verified against known exchange snapshots or expected replay results.

## Performance Philosophy

Low latency, especially p99 and p99.9 tail latency, is a primary acceptance
criterion. Follow the [latency requirements](latency.md) for timing definitions,
workloads, numerical budgets, and regression checks. Measurement starts in Phase 1;
optimization follows evidence without compromising book correctness.

Performance work will be accompanied by reproducible benchmarks describing:

- hardware
- workload
- number of instruments
- message rate
- benchmark methodology
- latency distribution
- throughput
- allocation behavior

The goal is to make performance characteristics measurable and explainable.

## Continuous Benchmarking and Profiling

Benchmarking and profiling are part of development from the first implemented
component through every roadmap phase. Benchmarking establishes whether a change
meets performance requirements; profiling investigates where time and resources
are spent. A profile alone does not establish a latency improvement.

### When to Measure

| Development event | Required evidence |
| --- | --- |
| First implementation of a hot-path component | Correctness tests, representative benchmark, initial CPU/allocation profile |
| Change to types, parsing, dependencies, storage, routing, queues, or publication | Relevant before/after benchmarks; profile when explaining an optimization or regression |
| Stage integration or milestone | Integrated sustained/burst workloads, tail-latency report, refreshed profiles |
| Unexpected slowdown or budget violation | Repeated benchmark reproduction, targeted profile, verified fix |
| Documentation-only or code outside the measured path | Ordinary applicable checks; no unrelated benchmark run required |

Profiles should also be refreshed at milestones even if no regression is apparent,
since the dominant cost can move as components are integrated. Share the workload
and report format between the Spot and USDⓈ-M developers.

### Development Loop

1. State the question and hypothesis: for example, numeric parsing allocations
   may increase tail latency during bursts. Select a representative workload and
   the latency/resource metric that will test it.
2. Validate correctness and benchmark the baseline commit in a release build.
   Follow the [latency measurement contract](latency.md), preserving offered load,
   configuration, and fixtures. Keep raw measurements and baseline metadata.
3. Profile that workload to identify likely causes. Use CPU sampling/call stacks
   for execution cost, allocation profiling for heap churn, and timing/tracing
   for queue residence, task scheduling, and consumer delay. A CPU flame graph
   alone cannot explain time spent waiting.
4. Change one relevant factor where practical. Re-run correctness checks and the
   same benchmark for the candidate; profile again to check the proposed cause.
5. Compare repeated baseline/candidate runs and record the result. Revert or revise
   ineffective changes; preserve an accepted tradeoff and its reason in the PR.

Measure final acceptance without the profiler attached, using the documented
instrumentation configuration. Profilers can perturb timing; record capture settings
and overhead. Use optimized builds with symbols suitable for the selected profiler,
and document any difference from the acceptance build. Platform-specific capture
commands belong beside the implemented benchmark harness once available.

### How to Judge a Change

Correctness and no-silent-drop delivery are gates. Then assess:

- **Latency budgets:** p99/p99.9 receipt-to-view and receipt-to-publication must
  satisfy agreed limits under the specified operating load. Before numerical
  budgets exist, compare against the retained baseline and record any regression.
- **Repeatability:** use repeated runs under comparable machine conditions,
  preferably alternating baseline and candidate to reduce drift. Report run
  variation and sample counts; an improvement inside the noise range is inconclusive.
- **Workload coverage:** compare both products and affected update sizes, burst
  rates, book depths, and instrument counts. Do not select only the fastest case.
- **Resource and delivery effects:** report allocations, CPU, memory, queue age,
  offered/processed load, and skipped views. Higher throughput does not compensate
  for unapproved tail-latency regression or hidden backlog.
- **Explanation:** profiles should support the proposed bottleneck and show whether
  it moved. Retain before/after numbers even when profiling suggests an improvement.

Stage benchmarks diagnose local costs; integrated benchmarks decide pipeline
acceptance. Do not interpret a microbenchmark's iteration-time distribution as
consumer-facing p99/p99.9. Relative regression tolerances must be agreed from runner
variability and recorded alongside absolute budgets; no universal percentage is
assumed. A missed budget requires a fix or an explicitly reviewed tradeoff.

### Harnesses, Tools, and Artifacts

`criterion` is available for component benchmarks. `benches/spot_sbe.rs` exercises
the implemented Spot readers; offline probe and smoke commands are documented in
[Spot SBE](spot-sbe.md). Other component/integrated harnesses remain unimplemented.
The integrated harness must provide independent arrivals, per-message/view timing,
and distributions as specified in [latency requirements](latency.md). Choose CPU,
allocation, and scheduler profilers appropriate to the platform and question,
recording their versions and capture commands rather than mandating a single tool.

As harnesses land, keep reproducible benchmark definitions under `benches/` and
record exact run/profile commands with their workloads. Store large raw results and
profiles as linked CI artifacts or local artifacts; keep concise milestone summaries
and accepted budgets in documentation. A performance-sensitive PR should provide:

- Baseline/candidate commits, question, workload/seed, exact commands, and environment.
- Before/after latency distributions and run variability, achieved load, queue age,
  allocations, and delivery counts relevant to the change.
- Profile artifact links and a short explanation when profiling was required.
- Correctness results and the acceptance decision, including any budget tradeoff.

Shared CI builds and smoke-checks implemented benchmarks. A stable runner performs
numerical regression checks once baselines and budgets are established. Neither
shared CI nor development profiles constitute a production latency guarantee.

## Connector Acceptance

Each product owns offline decoding and synchronization tests beside its modules,
with recorded or synthetic inputs under `tests/fixtures/binance/{spot,usdm}/`.
Document the selected feed and its synchronization rules against official Binance
protocol documentation when implementing it. Spot and USDⓈ-M sequence rules must
be tested independently; similar payloads do not justify sharing a synchronizer.

Cover snapshot alignment, accepted updates, duplicates, out-of-order inputs,
sequence gaps, disconnects, malformed messages, delayed snapshots, queue and
snapshot-buffer overflow, stale-cycle inputs, and recovery into a new cycle.
Live feed checks are optional manual checks; CI must not depend on exchange access.

After book integration, replay both products through the same L2 engine and assert
expected levels, zero-quantity deletion, exact arithmetic, best bid/ask, and
`Syncing` / `Live` / `Invalid` transitions. Verify that identical symbols in different
products route to independent books and that invalidation reaches consumers even
when prices have not changed. Also test freshness transitions without new depth
arrivals, stale-but-sequence-valid backlog and catch-up, prolonged inactivity,
consumer-side view expiry, and count/byte limits including concurrent recovery.

## Performance and Resilience Work

Establish baselines while implementing the connectors, then expand measurements
as the book and routing become available:

- Benchmark decoding, normalization, level updates, and end-to-end latency.
- Measure throughput, allocations, queue depth, CPU, and memory usage.
- Compare `BTreeMap`, sorted vectors, and price-indexed storage with reproducible workloads.
- Inject delayed messages, snapshot delays, consumer stalls, disconnects, and reconnect loops.
- Use bounded asynchronous queues and measure slow-consumer behavior.
- Evaluate conflation only for published views, never for book-building increments.

## Resource-Cost Evidence

Follow the mandatory [string, allocation, CPU, and syscall rules](engineering.md#strings-allocation-cpu-and-syscalls).
For first implementations and affected changes, include these measurements alongside
latency results. Budget values and exceptions are retained with the reference report.

| Dimension | Evidence | Workload coverage |
| --- | --- | --- |
| Strings/copies | Allocation/conversion sites, routing/label lifetime, payload ownership, copied bytes when measurable | Numeric/text payloads, normal updates, bounded diagnostic failures |
| Heap | Allocation/reallocation/free counts and bytes, peak live memory, capacity growth and reclamation | Warmed steady state, new levels, oversized input rejection, snapshot/reconnect cleanup |
| CPU | CPU time per message/level, thread utilization, idle cost, headroom, available hardware/scheduler counters | Sustained and burst rates, varying depth/update sizes, recovery interference |
| Syscalls | Category counts/rates and attributable time, blocking waits, kernel CPU, trace configuration | Idle, sustained/burst input, publication, telemetry, reconnect |

Separate harness/setup work from the component under test, but include library,
runtime, instrumentation, and recovery interference in integrated measurements.
Use targeted captures separately when simultaneous instrumentation distorts behavior.
Record unavailable measurements and their impact on acceptance explicitly. Resource
budgets do not substitute for independent correctness and receipt-to-view tests.

## Independent Correctness Reference

Build a simple test-only reference L2 implementation independently of optimized
storage, indexing, and update application. Replay the same snapshots and accepted
actions through both and compare all retained bid/ask levels, exact quantities,
ordering, deletions, and best bid/ask after each action. Compare state within the
same documented snapshot/retained-depth scope. Do not reuse the optimized mutation
algorithm as the oracle.

Exercise deterministic generated cases as well as fixed fixtures, retaining seeds
and minimized failing inputs. Changes to parser/sequence logic also need independently
specified expected decoded values and accepted/rejected actions; feeding both books
the same wrong action cannot validate a connector. Any complex parsing or storage
optimization must pass the relevant differential and failure-path tests.

## First Implementation and Optimization Gates

Use the [repository review workflows](engineering.md#agent-review-and-evidence)
for contract and performance evidence. They supplement executable checks; they
do not certify production readiness.

The first implemented hot-path component must arrive with a runnable benchmark,
representative fixed fixture or generator seed, exact benchmark/profile commands,
and retained baseline metadata/results. An empty `benches/` directory or a written
benchmark plan does not satisfy this gate. Add only harnesses for existing components;
expand to integrated load tests as integration lands.

Performance-sensitive changes must include relevant evidence in their PR, with
shared-path changes receiving review from the affected workstreams. Before a baseline exists, establish
one rather than claiming an unmeasured benefit. Documentation-only changes do not
require performance runs.

Custom parsers, unsafe code, lock-free structures, affinity changes, and similar
complexity require a reproduced bottleneck, measured benefit beyond run noise,
equivalent correctness/recovery coverage, and a recorded maintenance tradeoff.
Unsafe changes also document safety invariants and how they are exercised. Compare
against a simpler correct implementation and accept complexity only when its benefit
is justified under the operating workload.

## Observability

Track messages received/decoded, sequence gaps, reconnects, resynchronizations,
last-message timestamps, decoding/synchronization latency, queue residence time,
receipt-to-publication latency, consumer-observed latency, and queue depth.
Report p50, p95, p99, and p99.9, together with sample counts and budget violations. Label observations
by venue, product, and instrument so Spot and USDⓈ-M remain distinguishable.

## Security, Fuzzing, and Long-Running Tests

Follow [security verification gates](security.md#verification-gates) for parser and
state-machine fuzz targets, interleaving/unsafe checks where applicable, cancellation
injection, and soak tests. These land with relevant implementations, not placeholder
targets. Dependency CI runs advisory/license/source checks independently of AI review.

## Connection Acceptance

Use the [connection acceptance matrix](connections.md#acceptance-and-observability)
for deterministic lifecycle/backoff tests, local-server transport tests, continuity
and stale-data scenarios, connection scaling, mass reconnects, and resource soak
checks. Record per-connection and aggregate messages/bytes/levels, burst durations,
failure-detection delay, time to fresh books, and tail latency. Dozens of messages/s
is one initial workload, not a substitute for aggregate capacity measurements.
