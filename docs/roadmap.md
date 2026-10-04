# Roadmap

Start with Binance Spot and USDⓈ-M perpetual futures in parallel, then validate one
shared L2 engine. Multiple exchanges, including OKX, follow the Binance milestones.
Low-latency design and measurement run through every phase; Phase 4 deepens
optimization and resilience work. Benchmarking and profiling follow the
[continuous measurement workflow](testing.md#continuous-benchmarking-and-profiling)
throughout development. See [latency requirements](latency.md).
All items below are planned; the current crate contains module placeholders only.

## Phase 0 · Shared Data Contract

Agree on the smallest boundary for independent product development:

- [ ] Instrument identity includes venue, product type, and venue symbol from day one.
- [ ] Exact price and quantity representation without floating-point rounding.
- [ ] Snapshot replacement and absolute price-level updates with zero-quantity deletion.
- [ ] Explicit quantity units and required precision metadata; preserve product semantics.
- [ ] Ordered book actions, validity transitions, and failure delivery independent of a full depth queue.
- [ ] Fixture format and offline connector acceptance criteria.
- [ ] Security/input and supervisor/cancellation/retry contracts, dependency policy,
  and trusted review rules from [security and resilience](security.md).
- [ ] Reference environment and per-product sustained/burst workload contract.
- [ ] Separate sequence validity from freshness; agree watchdogs, age checks,
  catch-up and reconnect policies before defining the consumer interface.
- [ ] Count/byte resource limits, oversized-input handling, retained-depth policy,
  and bounded recovery concurrency; values land with each component.
- [ ] First-implementation evidence gate and review criteria for complex optimizations.
- [ ] String lifetime/routing policy, normal-path allocation/copy budgets, CPU/headroom
  and syscall measurement scope; record numeric budgets as baselines become available.
- [ ] Single-writer ownership, immutable publication, and repository review workflows
  defined in the [production engineering contract](engineering.md).
- [ ] Monotonic timing boundaries, representative workloads, benchmark report format,
  and reproducible profiling procedure.
- [ ] Review numeric representation, compact instrument identifiers, buffer ownership,
  and hot-path dependency features against the latency requirements.

Wire messages, sequence identifiers, and recovery cycle identity remain inside
product modules. Changes to `market_data`, dependencies, and module exports should
receive shared-interface review and merge in small PRs before dependent work.

**Milestone:** each product workstream can implement its connector without waiting for a
completed book or a universal connector trait.

## Phase 1 · Parallel Single-Instrument Connectors

| Workstream | Initial market | Contribution paths |
| --- | --- | --- |
| Spot | Binance Spot · BTCUSDT | `src/connector/binance/spot.rs`, `src/connector/binance/spot/`, `tests/fixtures/binance/spot/` |
| USDⓈ-M perpetual | Binance USDⓈ-M perpetual · BTCUSDT | `src/connector/binance/usdm.rs`, `src/connector/binance/usdm/`, `tests/fixtures/binance/usdm/` |

Each workstream implements:

- [ ] WebSocket connection lifecycle and product-specific depth subscriptions.
- [ ] Supervised deadlines/heartbeats, generation isolation, serialized outbound
  control, capped backoff with jitter, retry/rate limits, and deterministic tests
  under the [connection contract](connections.md).
- [ ] Product-specific message decoding and REST snapshot acquisition.
- [ ] Pure synchronization state machine and sequence validation.
- [ ] Bounded buffering, explicit overflow reporting, and stale-cycle isolation.
- [ ] Reconnection and resynchronization actions.
- [ ] Ordered snapshot/update actions using the shared contract.
- [ ] Offline fixtures for normal operation and failure paths.
- [ ] Parser/numeric fuzz targets and generated sync event cases; bounded retries,
  task supervision, and fault tests for shutdown and stale-cycle races.
- [ ] Document the chosen feed and its product-specific synchronization rules.
- [ ] Document crate selection and benchmark numeric decoding, conversions, and allocations.
- [ ] Capture initial CPU/allocation profiles for each implemented hot-path component.
- [ ] Audit string conversion/copy sites, allocation/reclamation paths, and integrated
  syscall behavior; include idle CPU and kernel costs in initial resource baselines.
- [ ] Deliver runnable benchmarks, fixed fixtures/seeds, commands, and baseline reports
  alongside the first hot-path implementation; enforce input/buffer limits.
- [ ] Establish decoding/synchronization latency baselines and queue residence measurements
  for both products, with p50, p95, p99, and p99.9 results.

Do not reuse Spot snapshot alignment or continuity rules in USDⓈ-M without verifying
that they apply. A diagnostic example may print actions before the book exists.

**Milestone:** both connectors pass offline acceptance tests for alignment, updates,
gaps, disconnects, overflow, and stale-cycle inputs. Live checks are optional;
this milestone does not claim a working local order book.

## Phase 2 · Shared L2 Book and Integration

Integrate both product workstreams with a shared engine, with review of the common interface:

- [ ] Snapshot replacement, absolute updates, zero-quantity deletion, and best bid/ask.
- [ ] Tests for ordering, precision, and expected state after replay.
- [ ] Independent test-only reference book with full retained-state differential replay.
- [ ] Apply accepted actions in each owning market data task.
- [ ] Publish `Syncing`, `Live`, and `Invalid` consumer views with separate freshness
  and timing metadata; test stale detection, catch-up, inactivity, and consumer delay.
- [ ] Invalidate on lost integrity and rebuild in a new synchronization cycle.
- [ ] Replay both products through the same book implementation.
- [ ] One single-instrument example per product.
- [ ] Profile the integrated pipeline, including queueing and consumer scheduling.
- [ ] Benchmark receipt-to-publication and receipt-to-consumer-view latency under
  sustained and burst loads; agree on numerical budgets from these baselines.

**Milestone:** independent, correct Spot and USDⓈ-M BTCUSDT books recover from
disconnects and sequence gaps using one L2 engine, with reproducible latency
baselines and documented budgets for each product, enforced resource limits, and
independently verified book state and freshness behavior.

## Phase 3 · Multi-Instrument and Routing

- [ ] Extend instrument metadata, including tick size and quantity step size.
- [ ] Multiple subscriptions and routing by full instrument identity.
- [ ] Bounded multi-connection admission, multiplexing/blast-radius policy, fair
  scheduling, noisy-neighbor isolation, and mass reconnect acceptance.
- [ ] Independent book, sequence, and synchronization state per instrument.
- [ ] Per-instrument recovery and bounded queues.
- [ ] Test product identity collisions, slow consumers, and high update rates.
- [ ] Refresh profiles for routing, task scheduling, and contention at multiple instruments.
- [ ] Verify tail-latency budgets as instrument count grows, including isolation
  when one instrument bursts or resynchronizes.

**Milestone:** multiple independent local books across both Binance products.

## Phase 4 · Latency Optimization, Resilience, and Observability

- [ ] Optimize profiled bottlenecks, verify changes with before/after benchmarks,
  and enforce agreed p99/p99.9 budgets.
- [ ] Reproducible decoding, normalization, book-update, and end-to-end benchmarks.
- [ ] Regression checks on a stable benchmark runner with retained reports.
- [ ] Throughput, allocations, CPU, memory, queue depth, and backpressure measurements.
- [ ] Failure injection for delays, malformed inputs, stalls, and reconnect loops.
- [ ] Metrics and tracing by venue, product, and instrument.
- [ ] Long-running/fault tests for memory plateau, task/socket retention, latency
  drift, repeated reconnects, and recovery isolation.

See [testing and performance](testing.md) for scenarios and measurement requirements.

**Milestone:** measurable performance and predictable recovery under failure.

## Phase 5 · Multiple Exchanges

- [ ] Add OKX as the next venue and choose/document its first product and feed.
- [ ] Validate shared data types and the L2 engine against its actual protocol.
- [ ] Keep OKX snapshot, sequencing, quantity units, and recovery semantics local.
- [ ] Add offline acceptance tests, replay, and examples before claiming support.
- [ ] Establish venue-specific latency baselines and profiles; validate shared-path budgets.
- [ ] Consider Bybit or Deribit after the multi-exchange boundary is proven.

**Milestone:** another exchange reuses the shared book without forcing its protocol
into Binance-specific assumptions. Additional products follow concrete needs.
