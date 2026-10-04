# Latency Requirements

Low latency is a core requirement of Strait, alongside correct book reconstruction
and reliable recovery. The main objective is to minimize the time between receiving
market data and making the resulting valid book view usable by a consumer, with
particular attention to p99 and p99.9 under sustained and burst load.

The project currently contains module placeholders. No measured latency results or
numerical guarantees exist yet. Measurement begins with connector implementation;
initial numerical budgets are agreed after integrated baselines, before expanding
to multiple instruments. Budgets must state hardware, workload, and operating load.

## Day 0 Operating Contract

Before parallel connector implementation, agree on the following contract in
Phase 0. Concrete values may initially be marked pending measurement, with an
owner and milestone for resolution; pending values are not acceptance guarantees.

| Contract | Required record | Resolution gate |
| --- | --- | --- |
| Reference environment | OS, CPU architecture/model, memory, runtime workers/task layout, Rust toolchain, build profile and compiler flags | Select before the first baseline; record actual settings with each run |
| Workload | Per-product sustained/offered message rate, levels per update, maintained depth, instrument count, burst rate/duration, fixture or seed | Initial cases before Phase 1 benchmarks; refine from representative feed data |
| Latency and freshness | Measurement boundaries, queue-age and feed-inactivity policies, numerical tail-latency budgets | Semantics in Phase 0; configured freshness limits before integrated publication; measured budgets in Phase 2 |
| Resource limits | Message/snapshot bytes, update levels, queue/buffer counts and retained bytes, maintained book size, concurrent recovery tasks | Policies in Phase 0; enforced values with each component |
| Evidence | Runnable benchmark, correctness fixtures, profile commands, retained baseline and comparison report | Accompany the first hot-path implementation and relevant changes |

Development machines may differ from the performance reference environment.
Performance acceptance compares the same environment and configuration; do not
combine results from different CPUs or build profiles into one regression series.
Record CPU affinity, power settings, background load, and virtualization when they
apply. Changes to the reference environment establish a new labeled baseline.

### Freshness and Overload

Sequence integrity and freshness are separate properties. `Live` means synchronized
and sequence-valid, not necessarily timely. Publish a separate freshness status
(`Fresh`, `Stale`, or `Unknown`) with its reason, timing metadata, and configured
limits. A usable real-time view requires both `Live` and `Fresh`.

Track reader-to-owner backlog age, age of the last accepted depth input, and age of
the view at consumer observation using local monotonic time. Product-specific
inactivity thresholds must account for feed behavior; heartbeats alone do not prove
that depth data is current. These checks express local freshness policy, not a
guarantee of exchange-to-consumer age. Missing timing evidence is `Unknown`.

When queue age exceeds its configured limit or the depth feed exceeds its inactivity
limit, publish `Stale` promptly even without a price change. If sequence integrity
is still intact, continue processing every update in order; return to `Fresh` only
when backlog and inactivity checks pass. Consumer delay can make a previously fresh
view stale, so consumers must recheck its timing metadata at use time.

If the backlog cannot recover within the configured catch-up timeout, or feed
inactivity persists past the configured reconnect timeout, invalidate and restart
synchronization in a new cycle. Overflow, gaps, or rejected required input invalidate
immediately. Never skip increments to catch up. Timeout/overload status must travel
independently of a full depth queue, and watchdog checks must not depend solely on
new depth arrivals. Record detection delay and state transitions in acceptance tests.

### Memory and Input Limits

Bound both item counts and retained bytes for asynchronous queues and synchronization
buffers; variable-size messages can exhaust memory within a count limit. Enforce
input limits before unbounded allocation wherever the transport/decoder permits.
Document limits for messages, snapshots, levels per update, maintained book levels,
and simultaneous snapshot/recovery work, including aggregate limits across books.

An oversized or malformed required input fails the affected cycle explicitly;
never truncate messages, levels, or snapshots to preserve latency. A book-size limit
must match a documented retained-depth policy and synchronization contract; do not
silently evict levels from a book advertised as complete. Configure timeouts and
bounded retry concurrency so recovery cannot consume unbounded CPU or memory.
Include boundary, over-limit, and concurrent-recovery workloads in validation.

## Type Design

Apply the mandatory [string, allocation, CPU, and syscall rules](engineering.md#strings-allocation-cpu-and-syscalls)
and retain [resource-cost evidence](testing.md#resource-cost-evidence) with latency baselines.

Shared domain types and product wire types must be reviewed for their effect on
latency before connector interfaces are settled. Document representation,
ownership, conversion cost, and expected allocation behavior.

- Use distinct types for instrument identity, price, quantity, and sequence state
  where they prevent invalid combinations. Resolve venue/product/symbol metadata
  at subscription time to a compact internal identifier; retain the original
  symbol in metadata rather than allocating or comparing it for every book update.
- Use exact numeric representations. Evaluate scaled integers for prices and
  quantities with documented scale, range, checked overflow, and cross-scale
  comparison rules. If a decimal type is needed, measure parsing, arithmetic,
  ordering, and conversion costs. Floating-point rounding is not an acceptable
  latency shortcut for book keys or quantities.
- Decode into typed wire structures and convert only required fields. Prefer
  borrowing from an input buffer while its lifetime permits; task/queue boundaries
  must have explicit ownership so buffer reuse cannot invalidate pending data.
  Avoid generic JSON trees and repeated string-to-number conversions on the
  steady-state path unless justified by measurements.
- Keep frequently accessed update fields together and use compact book actions
  and consumer views. Compare layout size, cache behavior, and copying cost;
  do not add packing, indirection, or whole-book cloning without a measured reason.
- Reuse or preallocate level buffers where practical. Measure capacity growth and
  oversized messages. Inline-capacity containers require workload evidence and
  an explicit overflow policy; fixed limits must never silently truncate updates.
- Prefer concrete types, enums, or static dispatch for a closed hot-path model.
  Evaluate dynamic dispatch, boxing, shared ownership, locks, and atomics where
  needed, recording their measured cost rather than banning them categorically.

Once code exists, benchmark numeric decoding, event conversion, action application,
and view publication with representative payloads and allocation counts. Type
changes are subject to the same latency regression policy as algorithm changes.

## Crate Selection and Configuration

A dependency choice is a latency design decision. Review the actual configured
call path, including enabled features and transitive dependencies, rather than
assuming a crate's popularity or throughput claims establish suitable tail latency.

- Classify dependencies as steady-state transport/decoding/processing, recovery
  and setup, or development-only. Prioritize measurements of the steady-state
  path while testing recovery interference with other live books.
- For hot-path candidates, compare representative release-build workloads using
  the same correctness checks, hardware, and arrival schedule. Report p99/p99.9,
  allocations, copies, buffering, locks, and scheduling behavior as applicable.
- Enable only required features and record runtime, TLS, buffering, compression,
  and logging configuration. Feature reduction is configuration hygiene; any
  claimed runtime latency improvement still requires a benchmark.
- Evaluate protocol correctness, supported limits, error handling, maintenance,
  unsafe-code requirements, and integration complexity alongside latency results.
  Custom code or a specialized parser needs a demonstrated benefit and equivalent
  correctness coverage before replacing a general-purpose dependency.
- Record the chosen version/features, rationale, tested alternatives, benchmark
  conditions, and results in the relevant implementation documentation or PR.
  Dependency upgrades affecting the hot path require before/after measurements
  once a baseline exists, even when the public API remains unchanged.

The current manifest is a starting set, not a validated low-latency selection:
`tokio`, `tokio-tungstenite`, and `futures-util` serve asynchronous transport;
`serde`/`serde_json` serve decoding; `reqwest` serves REST snapshot acquisition;
`tracing` serves instrumentation. `criterion` and `tracing-subscriber` are development
dependencies. Benchmark these configured paths as implementation lands; an HTTP
snapshot client is outside steady-state updates but can still contend for runtime
and CPU resources during recovery. Stage microbenchmarks must be complemented by
integrated queueing and consumer-delivery measurements.

## Measurement Boundaries

Use a monotonic clock for durations within the process. Define these timestamps:

| Timestamp | Boundary |
| --- | --- |
| `t_receive` | A complete WebSocket application message is delivered to the reader, before decoding or queueing |
| `t_process` | The owning market data task begins processing that message |
| `t_publish` | The resulting valid view is committed to the publication interface |
| `t_observe` | The benchmark consumer observes that view version |

- Queue residence: `t_process - t_receive` for the reader-to-owner boundary.
- Processing and publication: `t_publish - t_process`, with separate decoding,
  synchronization, normalization, and book-update timings where useful.
- Receipt-to-publication: `t_publish - t_receive`, including queue residence.
- Consumer delivery: `t_observe - t_publish`, including scheduling/wakeup delay.
- **Receipt-to-view:** `t_observe - t_receive`, the primary integrated metric.

These boundaries exclude exchange publishing delay, network transit, and buffering
before the application receives the complete message. Exchange timestamps may be
reported separately as feed age, but must not be subtracted from a local monotonic
clock or presented as measured one-way network latency without clock alignment.
Document any transport/frame assembly timing available in a particular benchmark.

Correlate timing with message identifiers and view versions. The `watch` interface
may conflate views: report how many versions were published, observed, and skipped.
Consumer-observed percentiles cover observed versions only; pair them with
receipt-to-publication distributions for every accepted live update to prevent
conflation from hiding slow processing. Discarded duplicates and recovery inputs
have separate outcomes and are not successful live-update latency samples.

## Budgets and Acceptance

For each product and representative workload, record:

- p50, p95, p99, and p99.9 receipt-to-publication and receipt-to-view latency.
- Queue residence and individual processing-stage distributions.
- Offered and processed message rates, update sizes, instrument count, book depth,
  queue depth/age, allocations, and consumer lag/skipped versions.
- Agreed numerical limits for p99/p99.9, queue residence, and sustained operating
  rate, with the conditions under which each limit applies.
- Budget violations, invalidations, overflows, and recovery duration separately.
- String conversion/copy costs; allocations, reallocations, frees and retained bytes;
  CPU time/headroom and syscall/kernel costs, with scoped budgets and recorded gaps.

Allocate a stage budget using measured bottlenecks and verify the overall budget
with the integrated distribution; adding stage percentiles does not produce an
end-to-end percentile. Low average latency or high throughput alone is insufficient.
A latency optimization must preserve sequence validation, exact arithmetic,
no-silent-drop delivery, and invalidation/recovery semantics.

## Benchmark Workloads

Benchmark Spot and USDⓈ-M independently using deterministic recorded or synthetic
inputs. Include steady state, exchange-like bursts, varying depth-update sizes,
multiple instruments, and slow consumers. Measure recovery and startup separately
from live steady state, while also exercising their impact on other live books.

Use an arrival schedule independent of processing completion for integrated load
tests. Include scheduling delay and backlog, record offered versus achieved load,
and avoid coordinated omission that makes an overloaded pipeline look fast.
Microbenchmarks support diagnosis; integrated runs validate consumer-facing latency.

Reports must include hardware/OS, Rust version, commit, release-build settings,
runtime/task configuration, queue capacities, fixtures/seed, offered load, run
length, warmup, sample count, histogram precision, and instrumentation configuration.
Collect enough samples to interpret p99.9 and repeat runs to quantify variability.
Measure timestamp/tracing/metrics overhead with instrumentation enabled and disabled.

## Regression Policy

Benchmarking and profiling continue throughout development. Follow the
[measurement workflow and evaluation criteria](testing.md#continuous-benchmarking-and-profiling)
for baselines, targeted profiles, repeated comparisons, and review evidence.

Retain baselines from Phase 1 and integrated budgets from Phase 2. Changes to
transport, decoding, numeric conversion, book storage, routing, queues, or publication
must provide relevant before/after latency results on the same workload and machine.

Enforce numerical regression thresholds on a stable benchmark runner once budgets
are established. Ordinary shared CI runs correctness and benchmark build/smoke
checks; noisy shared-runner timings do not establish latency guarantees. Investigate
budget violations and record the reason and revised budget explicitly when accepting
a tradeoff. Multi-instrument and new-venue milestones include latency acceptance.
