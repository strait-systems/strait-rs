# Architecture

Keep one crate while the two Binance product connectors and shared book are being
built. Separate product modules provide independent contribution areas.
Introduce workspace crates only when independent dependencies, publishing, or build
isolation justify them.

## Source Layout

Most modules are placeholders. The Spot protocol core implements decoding and a pure
sequence gate; see [its exact scope](spot-sbe.md). The following layout describes
the target architecture, not an implemented network pipeline.

```text
src/
├── lib.rs
├── market_data.rs              # Shared identity, numeric types, and book actions
├── connector.rs
├── connector/
│   ├── binance.rs              # Binance product namespace
│   └── binance/
│       ├── spot.rs
│       ├── spot/
│       │   ├── websocket.rs
│       │   ├── depth.rs
│       │   ├── snapshot.rs
│       │   └── sync.rs
│       ├── usdm.rs
│       └── usdm/
│           ├── websocket.rs
│           ├── depth.rs
│           ├── snapshot.rs
│           └── sync.rs
├── book.rs
└── book/
    └── l2.rs

tests/fixtures/binance/
├── spot/
└── usdm/
benches/
docs/
```

## Responsibilities

- `market_data`: instrument identity including venue, product, and venue symbol;
  exact prices/quantities with explicit units; snapshots, absolute level updates,
  validity, freshness, and timing metadata. Spot `BTCUSDT` and USDⓈ-M `BTCUSDT` are distinct identities.
- `connector::binance::spot`: Spot transport, wire decoding, WebSocket API SBE snapshots,
  sequence validation, synchronization, and recovery.
- `connector::binance::usdm`: USDⓈ-M perpetual transport, wire decoding, REST
  snapshots, sequence validation, synchronization, and recovery.
- `book::l2`: exchange-independent level storage, snapshot replacement,
  zero-quantity deletion, and best bid/ask queries; no network I/O or sequence rules.

Raw payloads, endpoints, subscriptions, sequence identifiers, synchronization
state, and recovery cycle identifiers stay inside each product connector.
Do not introduce a universal connector trait or shared Binance synchronizer before
both implementations validate a useful common boundary. Extract transport helpers
only when actual duplication appears; product protocol rules remain separate.

## Data Flow

```text
Product WebSocket → product decoding → product synchronization/sequence validation
                                             ↑
                                  Product-specific snapshot request
                                             ↓
                                  Ordered shared book actions
                                             ↓
                                  Shared L2 engine → consumer view
```

The pipeline runs independently per product/instrument. Instrument routing becomes
a shared responsibility in the multi-instrument phase. OKX modules and fixtures
will be introduced when multi-exchange work begins.

## Latency Constraints

The architecture must support the [latency requirements](latency.md), including
type representation, ownership, and dependency selection as well as timing.
These are implementation requirements, not claims about the partial protocol core.

- Timestamp complete application-message receipt with a monotonic clock before
  queueing or decoding. Carry its timing context through accepted book actions
  to publication; use per-message timing and identify views by version.
- Measure the reader-to-owner queue residence time, processing time, and
  publication time separately. Add consumer-observed timing when evaluating
  the complete receipt-to-view path.
- Decode, validate, normalize, and mutate the book within the owning task where
  practical. Module boundaries do not require additional tasks or queues.
- Keep snapshot requests, reconnect orchestration, blocking I/O, and synchronous log
  output off the steady-state update path. Measure tracing/metrics overhead.
- Reuse buffers where measurements justify it; avoid unnecessary payload copies,
  repeated string conversions, per-level task spawning, and full-book clones
  for best bid/ask publication. Preserve exact numeric representation.
- Size bounded queues against measured burst rates and latency budgets. Monitor
  queue age as well as depth; batching must have an explicit bounded wait policy.
- Publish validity changes promptly. A slow consumer must not stall book updates;
  consumer scheduling delay remains visible in receipt-to-view measurements.

The first implementation uses the existing single-owner task and bounded queue
contract. Alternatives such as an integrated reader/owner or different publication
mechanisms require measured latency benefits and equivalent correctness/recovery.

## Single-Writer Contract

The [production engineering contract](engineering.md#single-writer-is-the-default-ownership-model)
defines one mutation authority per instrument, including recovery, immutable consumer
views, and gates for changing the model. A task is a logical owner; dedicated threads
or shared mutable books are not implied.

## Initial Implementation Contracts

Both product implementations follow these contracts. They are design requirements
for the upcoming implementation, not currently implemented capabilities. During
parallel connector development, synchronization is tested through deterministic
inputs and emitted actions; shared book integration follows in Phase 2.

#### Book Ownership

A single market data task exclusively owns and mutates the local L2 book, sequence
state, and synchronization buffer. Other tasks provide inputs through messages;
they do not directly mutate the book. The initial implementation does not require
shared locks around book state.

```text
WebSocket reader ── bounded queue ──► Market data task
Snapshot input (REST or WS API) ──►     ├─ Synchronization state machine
                                        ├─ Local L2 book
                                        └─ Consumer view publication
```

#### Incremental Delivery and Recovery

Both the incoming depth queue and the buffer used while awaiting a snapshot have
explicit item-count and retained-byte limits. Message/snapshot sizes, level counts,
book storage, and concurrent recovery work also have configured limits; see
[memory and input limits](latency.md#memory-and-input-limits).
Incremental updates must never be silently dropped.
A sequence gap or either buffer overflowing invalidates the book and starts a
new synchronization cycle. The socket-restart path below applies when required by
the feed; multiplexed connections use instrument-scoped recovery where permitted
by the [connection contract](connections.md#multiple-connections-and-traffic):

```text
Publish Invalid
    ↓
Stop the old connection and clear its pending inputs and synchronization buffer
    ↓
Establish a new connection
    ↓
Obtain a fresh snapshot using the product protocol and synchronize updates
    ↓
Publish Live only after synchronization succeeds
```

The reader must report queue overflow through a path independent of the full
depth queue, such as its supervised task result. Once integrity is lost, further
increments from that cycle must not be applied to the book.

Each connection and synchronization cycle has an identifier. Snapshot results and
other asynchronous inputs are associated with that identifier, so late results
from an earlier cycle cannot be applied to the current book. Capacity limits are
configuration values to be tuned through measurement.

#### Synchronization Without Network I/O

The synchronization state machines in `src/connector/binance/{spot,usdm}/sync.rs`
perform no network I/O. Each maintains product-specific synchronization and sequence
state, accepts snapshots, depth updates, and failure inputs, and returns actions
such as replacing the book, applying an update, or requiring resynchronization.
The shared book is integrated in Phase 2 and owned by the same market data task.
A transition to live requires both successful synchronization and successful
application of the corresponding book actions.

The surrounding asynchronous task obtains snapshots as required by the product, performs connection
management, timeout handling, and consumer publication. Tests can drive state
transitions with recorded or synthetic inputs without a live exchange connection
or an asynchronous runtime. They must cover snapshot alignment, sequence
continuity, overflow, invalidation, and recovery into a new synchronization cycle.

#### Consumer Views and Validity

The first consumer interface publishes the latest book view, initially best bid
and ask, through `tokio::sync::watch`. Every published view carries a validity
state: `Syncing`, `Live`, or `Invalid`, plus a separate freshness status of
`Fresh`, `Stale`, or `Unknown`. Prices are usable as real-time data only when
`Live` and `Fresh`, with timing metadata checked at consumer use time. Validity
and freshness changes must be published even when no price level changes.
See [freshness and overload](latency.md#freshness-and-overload) for watchdogs,
catch-up/reconnect policies, and consumer age checks.

Consumer views may be conflated: intermediate versions can be skipped by a slow
consumer because this interface provides the latest state. This does not permit
dropping incremental inputs used to maintain the book. Consumers requiring every
event will need a separate interface with an explicit delivery policy.

## Connection Management

Apply the [connection lifecycle and capacity contract](connections.md). Connection
management owns admission, deadlines, supervised lifecycle, and retry scheduling;
product sync machines own protocol continuity and instrument writers own books.
Multiplexed sockets preserve independent instrument state. Socket failures invalidate
all dependent books; instrument-scoped recovery is preferred where the feed permits
it. The earlier connection-restart sequence applies when recovery requires a new
socket, not as a mandatory response to every instrument-specific gap.
