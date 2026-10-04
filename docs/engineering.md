# Production Engineering Contract

Strait is designed for high-frequency production market data with low, predictable
tail latency. Features are accepted only with applicable correctness, resource,
recovery, and performance evidence. This is an engineering target, not a claim
that the current skeleton is production-ready. Scope remains public market data.

This document defines mandatory engineering rules. [Latency requirements](latency.md)
define representations, measurements, and operating budgets; [testing](testing.md)
defines evidence and comparisons. Unknown costs remain unverified until measured.

## Single Writer Is the Default Ownership Model

Exactly one logical writer owns the mutable book, sequence state, synchronization
buffer, and recovery-cycle state for each instrument. The writer can be a task;
it need not have a dedicated OS thread. Sharding may put multiple instruments on
one writer; each instrument still has one mutation authority.

- WebSocket readers and snapshot workers send owned or lifetime-safe inputs to the
  writer. They do not mutate the book or sequence state, including during recovery.
- A cycle identifier gates asynchronous results. The writer alone accepts inputs,
  validates sequence continuity, applies ordered actions, and publishes versions.
- Keep each accepted live update's validation, mutation, and view commit as one
  ordered operation. Do not await arbitrary consumer work or snapshot I/O inside
  it. Invalid/recovery transitions cannot be hidden behind pending data traffic.
- Consumers receive immutable compact views or messages. Published views include
  instrument/cycle/version, validity, freshness, and the applicable timing metadata.
  References into mutable book storage must not escape to concurrent consumers.
- Do not use a shared `Arc<Mutex<Book>>` or `Arc<RwLock<Book>>` as the normal
  consumer/mutation interface. Replacing contention with a multi-writer atomic
  structure does not satisfy this ownership contract.
- A design changing this model requires a written ownership/ordering proof,
  differential/recovery tests, representative tail-latency evidence, and explicit
  shared-design review before adoption. A faster average alone is insufficient.

An async task may migrate between threads without breaking single-writer ownership.
Thread pinning, dedicated cores, SPSC queues, or custom runtimes are experiments
requiring evidence, not prerequisites or automatic improvements.

## Types and Memory

Apply [type-design requirements](latency.md#type-design) before settling interfaces.
Separate wire types from domain types and sequence/cycle state from book storage.
Strong price/quantity/instrument types enforce units and scale; checked ranges and
exact conversion behavior are part of their contract. Reject unsupported precision
explicitly rather than rounding or wrapping. Dynamic metadata must not change the
interpretation of existing levels without a controlled rebuild or conversion.

Document the normal-path allocation/copy budget for each implemented component and
measure it from the first benchmark. Prefer preallocated/reusable owned storage
and compact identifiers; a zero-allocation goal must state which operations and
payload bounds it covers. Allocation exceptions, capacity growth, and reclamation
costs belong in the benchmark. Avoid allocation-per-level designs without justified
measurements; do not impose a blanket zero-allocation claim on unmeasured code.

## Strings, Allocation, CPU, and Syscalls

These are mandatory review and measurement dimensions from the first hot-path
implementation. Defaults constrain avoidable work; exceptions require a scoped
reason and representative before/after evidence, not an assumed benefit.

### String Handling

Venue symbols, subscription strings, configuration, and human-readable labels are
constructed or resolved during setup. Within repeated book operations, use compact
instrument identifiers, typed numeric fields, and structured enums/error codes.
Do not repeatedly allocate, format, case-convert, or concatenate strings for routing,
level updates, metrics labels, or normal-path logs. Resolve bounded metric labels
outside the update loop; never use message IDs or prices as unbounded label values.

Borrow wire strings/bytes when lifetimes permit, validate required text encoding,
and parse exact numeric values once at the wire/domain boundary. Raw input may need
ownership across a queue; explicitly account for that copy/allocation and avoid
additional payload clones. An owned `String` in metadata is valid; per-update string
work requires an explained need and measured cost. Error paths use bounded diagnostics
and must also be exercised under malformed-input storms.

### Allocation and Reclamation

For warmed steady-state decode/apply/publish operations within declared payload and
book-size bounds, target zero avoidable heap allocations. Each component records
allocation, reallocation, deallocation counts and bytes per message/update, peak live
bytes, and copied bytes where measurable. Transport/library allocations are included
in integrated results even if the core has a zero-allocation baseline.

Reserve reusable buffers from the workload contract and enforce capacity/byte limits.
No unreviewed `collect`, `to_owned`, boxing, format allocation, buffer growth, or
whole-book clone belongs in a repeated update path. Review actual reachable behavior,
not keywords. New book levels, changing retained depth, and library-owned frames may
need allocation; bound and benchmark these cases explicitly rather than exclude them.
A component's accepted nonzero budget must list these cases and its rationale.

Measure destruction, reference-count operations, pool return, and reuse on the actual
thread/task where they occur. Include reconnect cleanup and snapshot replacement;
moving allocation off the writer does not eliminate contention or reclamation cost.
Do not repeatedly shrink reusable storage. Memory pools require bounded retention,
lifetime/reuse correctness, and demonstrated benefit. No allocator substitution or
unbounded pool is accepted as a shortcut to predictable latency.

### CPU Work and Scheduling

Record CPU time per message and per changed level, per-thread utilization, idle-load
CPU cost, and sustained operating-rate headroom. Where supported, use cycles,
instructions, branch misses, cache misses, migrations, and context switches to explain
costs. Counter availability and measurement overhead must be recorded; unsupported
counters are gaps, not zero values. Process-wide CPU percentage alone is insufficient.

Avoid repeated parsing, rescanning/rebuilding the full book for best bid/ask, and
unnecessary per-level tasks or repeated conversions. State algorithmic cost as a
function of update size and maintained depth, then measure small and large cases.
Single writer still needs bounded per-turn work and fair scheduling for controls
and other instruments. Busy polling or spin loops require explicit core/power and
headroom budgets, measured tail-latency benefit, and reviewed overload behavior.

Set numeric CPU/headroom budgets on the reference environment from baselines, alongside
latency limits. CPU reduction cannot justify missed tail budgets; lower latency at
CPU saturation is not evidence of sustainable production capacity.

### Syscall and Kernel Interaction

The pure decoder, synchronizer, and book-update components perform no direct network,
file, console, sleep, or blocking synchronization calls. Transport, runtime wakeups,
publication, clock access, and instrumentation may introduce kernel interaction;
measure these on the integrated path rather than claiming syscall-free operation
from source inspection or assuming every API call maps to a syscall.

Capture syscall counts/rates and attributable time by category, idle and loaded
behavior, blocking waits, and user/kernel CPU time when supported. Include transitive
crate/runtime behavior and failed/retried calls; counts per message must identify
aggregation intervals and offered/processed load. Unnecessary per-update flushes,
log writes, timer registration, and wakeups require redesign or measured justification.

Coalescing I/O or notifications may reduce kernel interaction but must have a bounded
wait policy, preserve delivery semantics, and satisfy tail-latency/freshness budgets.
More syscalls do not alone establish a regression. Validate tracing/profiling overhead
and confirm acceptance latency in unprofiled runs with normal instrumentation.

## Processing and Concurrency

Keep decode, validate, normalize, apply, and publish close to the writer. A module
boundary does not imply a task, queue, trait object, or package boundary. Add
abstractions only for demonstrated product differences or tested reuse; account for
indirection and allocation costs. Consumer callbacks never execute arbitrary work
on the writer. Blocking I/O, synchronous log sinks, snapshots, and retry orchestration
remain outside the steady-state operation.

Every async boundary states producer/consumer ownership, ordering, count/byte limits,
overflow delivery, cancellation, and cycle isolation. Concurrent producers need an
explicit ordering contract; channel arrival order is not exchange sequence order.
Batching has a bounded time/size policy and must preserve ordered book actions.
Long decode/replay/recovery operations require a measured fairness policy so one
instrument cannot starve liveness checks or unrelated live books.

Apply [freshness and overload](latency.md#freshness-and-overload), not just overflow
handling. Single-writer ownership does not eliminate scheduling or queue delay.

## Dependencies and Tool Selection

Apply [crate-selection requirements](latency.md#crate-selection-and-configuration)
to libraries, runtime settings, parsers, allocators, channels, and instrumentation.
Separate setup/recovery and steady-state costs. Do not introduce a dependency or
pattern solely because it is conventional in general-purpose application code.
Do not reject a general-purpose crate solely by reputation either: benchmark the
configured path, verify protocol/error semantics, and document the choice.

Custom parsers, unsafe code, lock-free publication, allocator changes, SIMD, CPU
specific build flags, and CPU affinity require a reproduced bottleneck, measured
benefit, equivalent correctness coverage, and maintenance/portability tradeoffs.
Unsafe code must state safety and lifetime invariants and how they are validated.
Designs with shared atomics must explain publication/reclamation ordering. The
project does not prescribe a library winner without implementation evidence.

## Production Acceptance

Before describing an implementation as production-ready, retain evidence for:

- Independent reference-book replay and product-specific protocol/failure tests.
- Freshness and invalidation detection under backlog, inactivity, and consumer lag.
- Sustained, burst, overload, and long-running tests on the reference environment,
  with p99/p99.9 budgets, achieved load, memory bounds, and instrumentation enabled.
- Bounded reconnect/snapshot retries and concurrency, cancellation/shutdown,
  stale-cycle rejection, and recovery isolation between instruments.
- Actionable metrics for queue age, freshness, gaps, retries, resource-limit failures,
  budget violations, and time to recovery; telemetry itself has measured cost.
- Reproducible commands, configurations, operating limits, and failure/recovery
  procedures. Known gaps are listed; passing microbenchmarks is not readiness.

Evidence must match the release configuration and supported operating workload.
No numerical guarantee or high-frequency readiness follows from Rust, single writer,
a chosen crate, or an attractive benchmark in isolation.

## Agent Review and Evidence

Repository [AGENTS.md](../AGENTS.md) makes contract checks part of coding work.
Three repository skills provide the repeatable workflows:

- `$strait-contract-review`: inspect a diff against ownership, type, flow, dependency,
  freshness, resource, and correctness contracts; identify required changes.
- `$strait-performance-check`: run or assess relevant baselines, load tests, and
  profiles; classify budget compliance, regressions, and missing evidence.

- `$strait-security-review`: apply [security and resilience](security.md) for
  protocol inputs, unsafe behavior, dependency supply chain, and CI trust.

Canonical instructions live under `skills/`, with discovery entries in
`.agents/skills/` for Codex and `.claude/skills/` for Claude Code. Only the Codex
adapter carries `agents/openai.yaml`; see [shared skill layout](contributing/agent-skills.md).
Agent review is advisory evidence, not proof of correctness. The
[AI review workflow](maintainers/ai-review.md) has a disposition gate; tests and
benchmarks provide separate executable checks. Review-only requests do not authorize edits;
implementation requests include fixing applicable findings within their scope.

## Connection Requirements

The [connection contract](connections.md) defines supervised lifecycle, serialized
outbound control, generations, backoff/jitter, continuity, stale detection, bounded
multi-connection management, and transport/load acceptance. All three review skills
apply its relevant requirements; connection changes do not bypass single-writer or
performance gates.
