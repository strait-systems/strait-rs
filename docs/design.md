# Design and Scope

Strait is an open-source Rust project for public exchange market data and local L2
order books. Low, predictable receipt-to-view latency under sustained and burst
load is a defining requirement. Correctness remains mandatory; performance claims
require reproducible evidence. The current crate is a skeleton, not a production
implementation.

## Scope

Initial implementation targets Binance Spot and USDⓈ-M perpetual futures, starting
with `BTCUSDT` in each product. Scope includes transport, subscriptions, decoding,
snapshot/update synchronization, sequence validation, book maintenance, freshness,
recovery, routing, observability, replay, and performance measurement.

Order entry, private account streams, order/position management, strategies, and
risk management are outside scope. Other exchanges follow proven Binance paths;
see the [roadmap](roadmap.md) for staged deliverables.

## Design Decisions

- One logical writer per instrument owns mutation, sequence/synchronization state,
  recovery cycles, and ordered publication. Consumers receive immutable views.
- Transport, product protocol, synchronization, and book storage have separate
  responsibilities. Module boundaries do not require separate tasks or queues.
- Product-specific semantics remain explicit. Learn from Spot and USDⓈ-M before
  stabilizing shared abstractions; similar wire formats do not establish equivalent
  sequence or recovery rules.
- Sequence validity and freshness are separate. Gaps, disconnects, malformed input,
  stale data, and overload are normal conditions with explicit failure behavior.
- Types, crates, memory ownership, strings, allocation, CPU work, syscalls, and
  scheduling are latency decisions. Complexity requires evidence; Rust or a design
  pattern alone does not establish high-frequency readiness.

## Detailed Contracts

This overview explains direction; detailed rules have dedicated sources:

| Question | Canonical document |
| --- | --- |
| Where do components and data ownership live? | [Architecture](architecture.md) |
| What must implementation preserve? | [Production engineering](engineering.md) |
| How are types, crate choices, load, freshness, and latency budgets defined? | [Latency requirements](latency.md) |
| How do connections recover and scale? | [Connections](connections.md) |
| How are correctness, benchmarks, profiles, and regressions verified? | [Testing and performance](testing.md) |
| What protects against untrusted input and resource exhaustion? | [Security and resilience](security.md) |

Benchmarking and profiling begin with the first implemented hot-path component and
continue through integration and maintenance. Numerical budgets follow measured
baselines; measurement boundaries and failure semantics precede implementation.
