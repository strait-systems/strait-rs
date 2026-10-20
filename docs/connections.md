# Connection Lifecycle and Capacity Contract

Connection correctness is part of production acceptance, alongside single-writer
ownership and low tail latency. Initial implementations are Binance Spot and USD-M
WebSockets plus product-specific snapshot requests: Spot selects a separate persistent
WebSocket API SBE session; USD-M retains REST snapshots. Spot REST SBE is a benchmark
comparison, not an automatic fallback. Future transports follow the same ownership, limits,
liveness, and recovery principles, with their actual framing/delivery semantics;
this contract does not claim they are implemented or mandate a universal transport trait.

## Lifecycle and Ownership

Define explicit states and transitions for dialing, transport established,
subscribing, synchronizing, streaming, reconnect waiting, stopping, and stopped.
A connected socket does not imply an acknowledged subscription, synchronized book,
or fresh data. Separate connection health from per-instrument validity/freshness.

Each connection has one supervised lifecycle owner and one serialized outbound write
path for subscription, heartbeat, and close messages. Readers deliver inputs to the
instrument's single writer; they never mutate book/sequence state. A manager owns
connection registry/admission/retry scheduling, not book mutations. Give connections
and instrument synchronization cycles independent generation identifiers and reject
late reads, acknowledgements, snapshots, and timers from old generations.

Configure bounded DNS/connect/TLS/handshake, subscription, snapshot, idle/liveness,
and close/shutdown deadlines as applicable. Handle clean close, abrupt EOF, resets,
reader/writer failure, and silent blackholes explicitly. Supervision and watchdogs
must run without new depth arrivals and must not queue behind saturated data traffic.
On disconnect, invalidate every dependent instrument promptly and retire old inputs.
Cancel/join old producers before activating a replacement; no duplicate live readers,
subscriptions, or reconnect attempts for the same connection generation.

## Reconnect, Backoff, and Jitter

Classify retryable network/venue failures versus unsupported configuration/protocol
errors. Do not loop on permanent errors. Every reconnect attempt is cancellation-aware
and constrained by per-connection and venue/global admission budgets.

Use configurable capped exponential backoff with full jitter as the initial policy:
for consecutive failed attempt index `n`, sample delay uniformly from zero to
`min(max_delay, initial_delay * 2^n)`. Compute the cap with saturation/checked arithmetic;
configure initial/max delays, retry budget/window, and exhaustion/cooldown behavior.
Honor applicable server retry-after minimums before adding spread. Do not reset the
failure counter just because TCP connects; reset after a configured stable healthy
streaming interval. Independently seed connection schedules so fleet recovery does
not synchronize. Tests inject a deterministic RNG and clock; production avoids a
shared identical deterministic schedule. A zero jitter sample must still respect
global attempt rate limits.

Bound concurrent dials, snapshot requests, subscriptions, and scheduled retries. Enforce
venue-wide limits across connections, including control messages where applicable.
Document feed-specific limits against official protocol documentation when implementing
or changing a feed; no venue limit or sequence formula is assumed here. Record attempts,
delays, exhausted budgets, rate-limit responses, and time to usable fresh books.

## Protocol Continuity and Resynchronization

Sequence validation is product/feed-specific and stays in each pure sync machine.
Document snapshot alignment and handling of ranges, duplicates, out-of-order inputs,
missing sequence fields, gaps, and sequence reset across generations. Do not assume
simple `last + 1` continuity or that Spot and USD-M follow identical rules.

Validate per instrument even when streams share a socket. Connection generation
identity does not replace sequence validation. Lost required input, gaps, or overflow
invalidate affected state; never skip increments to keep up. A reconnect restores
subscriptions and obtains/aligns snapshots before publishing `Live` and `Fresh`.
A healthy replacement socket alone never restores a book to usable state.

## Liveness and Stale Data

Track transport heartbeat/ping-pong health, last received complete message, last
accepted depth input per instrument, oldest queued input, and consumer view age
separately using local monotonic time. A heartbeat or another active instrument does
not prove a quiet instrument's depth is fresh. Apply
[freshness and overload](latency.md#freshness-and-overload), including `Unknown` when
evidence is missing. Product-specific inactivity rules account for quiet markets;
no-message timeout is a policy signal, not proof that the exchange missed an update.

Configure protocol-aware heartbeat response and detection deadlines. Serialize
outbound control traffic and reserve bounded scheduling capacity for it, while
respecting venue limits. Measure watchdog/control delay under flood and snapshot
recovery. Stale-but-sequence-valid backlog may catch up without dropping updates;
persistent staleness triggers the documented bounded recovery policy.

## Multiple Connections and Traffic

Record per-connection and aggregate connection count, subscriptions/instruments,
messages per second, bytes per second, levels per message, peak/burst duration,
consumer rate, book depth, and recovery workload. **Dozens of messages per second**
are an initial representative case, not a proven capacity ceiling or a sufficient
high-load definition. Large updates and many simultaneous connections can dominate
work even at that per-connection rate. Numerical supported capacity follows measurement.

Bound open/in-flight connections, tasks, timers, file descriptors, transport buffers,
queues by count/bytes, retained books, and aggregate memory. Allocate admission budgets
before establishing new resources. No task/thread per price level or unbounded queue
is allowed. Do not require a dedicated OS thread per connection; shard ownership and
runtime work with measured fairness while preserving one book writer per instrument.

A noisy or recovering connection must not starve unrelated readers, writers, control
signals, or consumers. Define blast radius: socket failure affects its subscriptions;
per-instrument sequence failure need not restart an otherwise healthy shared socket.
Prefer instrument-scoped recovery where the feed permits it. If protocol constraints
require socket-wide recovery, invalidate all affected books and report that scope.
Batch/control limits and async queues must preserve order and no-silent-drop delivery.

## Acceptance and Observability

Use fake clocks/RNGs for deterministic state/backoff tests and local controllable
servers for transport integration; CI never relies on a live exchange. Include:

- Clean/abrupt disconnect, failed dial/TLS/subscription, blackhole, missed heartbeat,
  delayed frames, failed snapshots, and cancellation at every lifecycle stage.
- Backoff cap/range and overflow, independent schedules, stability-based reset,
  server cooldowns, retry exhaustion, and global limits under mass reconnects.
- Snapshot races, duplicate/out-of-order/gapped updates, stale generation inputs,
  reused symbols across products, and multiplexed instrument isolation.
- Initial dozens-of-messages/s cases, larger sustained/burst workloads, connection
  count scaling, a noisy neighbor, slow consumers, and simultaneous recovery.
- Soak tests for task/socket/timer/memory plateaus and repeated reconnects, with
  no stale output labeled fresh and no lost increments counted as successful work.

Report connected/connecting/backoff counts, subscription health, heartbeat/depth
ages, queue age/depth, retry causes/delays, gaps, stale transitions, invalidations,
resource/admission failures, and time from failure to usable fresh state. Labels
have bounded cardinality; avoid accumulating unique generation IDs in metric labels.
Measure p99/p99.9, throughput, CPU/headroom, allocations, syscall/kernel costs, and
control/failure-detection delay for the aggregate workload and individual connections.
This contract specifies planned acceptance requirements; no capacity is certified yet.
