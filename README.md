# Strait

**Low-latency CEX market data connectivity and local order books in Rust.**

> **Status:** Early development. Spot SBE depth/snapshot decoding and a pure sequence
> gate are implemented with offline fixtures. Network sessions, recovery orchestration,
> and local books are not implemented; APIs are unstable.

Strait is an open-source Rust project focused on public market data: exchange feeds, snapshot/update synchronization,
sequence validation, local L2 books, and recovery. **Low latency is a core design
requirement**, with particular attention to p99 and p99.9 tail latency from message
receipt to a usable consumer view. Correctness remains mandatory.

Benchmarking and profiling start with the first implemented components and continue
through every development phase; see the [measurement workflow](docs/testing.md#continuous-benchmarking-and-profiling).
Type design, crate selection, strings/copies, allocation/reclamation, CPU and syscall
costs, queueing, task handoffs, and publication are part of the latency budget. **Single writer per instrument** is the default
mutation model. High-frequency production use is the acceptance target, not a
claim about the current implementation.
Numerical targets will be set from reproducible baselines on documented hardware;
only local decoder microbenchmarks exist, not integrated latency results.
See [latency requirements](docs/latency.md).
The [Day 0 contract](docs/latency.md#day-0-operating-contract) fixes environment,
load, freshness, resource limits, and evidence required with each implementation.

## High-frequency & Low-latency Design

The first implemented component is the **offline Spot SBE protocol core**:

- **Bounded decoding:** validate message layouts and count/byte limits before using generated accessors.
- **Exact numerics:** checked integer prices/quantities, with conversion factors prepared once per batch.
- **Reusable storage:** preallocate one level buffer and return borrowed views; the small-fixture probe records zero warmed heap allocations.
- **Explicit sequence acceptance:** stage an action, then advance sequence state only after application is acknowledged.

See the [design walkthrough and performance evidence](docs/low-latency-design.md)
for source links, measured tradeoffs, and the planned single-writer pipeline.
Local microbenchmarks exist; integrated throughput and p99/p99.9 remain unmeasured.

## Initial Scope

Initial development targets **Binance Spot** and **Binance USDⓈ-M perpetual
futures** in parallel, starting with `BTCUSDT` in each market. Both connectors share
market data types and an exchange-independent L2 book, while keeping product-specific
protocol rules separate. Instrument identity includes the product type.

Order entry, private account streams, strategies, and risk management are out of scope.
OKX and other exchanges are deferred until the Binance paths are proven.

## Plan

1. Agree on shared instrument identity, exact numeric types, book actions, validity states, and latency measurement boundaries.
2. Build Spot and USDⓈ-M connectors independently, with offline tests and initial latency baselines.
3. Integrate both with one L2 engine and measure receipt-to-view latency for valid best bid/ask views.
4. Add multiple instruments, routing, observability, and reproducible performance tests.
5. Add OKX and other exchanges once the Binance implementation is reliable.

See the [detailed roadmap and work ownership](docs/roadmap.md) for acceptance criteria.

## Development

Keep a single crate with independent `connector::binance::spot` and
`connector::binance::usdm` modules. Rust 1.88 or later is required.

Contributions, bug reports, and design discussions are welcome. Changes should follow
the documented correctness, latency, and validation requirements.

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo test --locked --doc --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --all-features
```

AI skill reviews can run on PRs through GitHub Actions; see
[CI setup and report behavior](docs/maintainers/ai-review.md).

## Documentation

Start with [design and scope](docs/design.md), then [architecture](docs/architecture.md).
The [documentation guide](docs/README.md) explains where each requirement belongs.

- [Production engineering](docs/engineering.md): single writer, resource costs, and implementation contracts
- [Latency requirements](docs/latency.md): types, dependencies, operating budgets, and measurement definitions
- [Testing and performance](docs/testing.md): correctness evidence, benchmarks, and profiling workflow
- [Connections](docs/connections.md): lifecycle, continuity, freshness, and aggregate capacity
- [Security and resilience](docs/security.md): input safety, supervision, supply chain, and verification gates
- [Roadmap](docs/roadmap.md): staged deliverables and acceptance criteria
- [Spot SBE protocol core](docs/spot-sbe.md): selected feed, snapshot protocol, offline example and evidence

Contributor tooling: [shared agent skills](docs/contributing/agent-skills.md).
Maintainer setup: [AI review](docs/maintainers/ai-review.md) and
[branch protection](docs/maintainers/branch-protection.md).

## License

Licensed under [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.
