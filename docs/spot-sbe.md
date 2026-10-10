# Binance Spot SBE Protocol Core

## Selected Design and Current Scope

The selected design is SBE Diff Depth plus a **separate persistent WebSocket API
SBE snapshot session**. Correctness is mandatory; latency drives transport selection.
REST SBE snapshots are a future measured comparison, not an automatic fallback.
The WebSocket choice is a design hypothesis, not a demonstrated advantage over a
warmed HTTP connection pool. USD-M remains independent and is not implemented.

Implemented here:

- Exact checked prices/quantities, explicitly scaled. Spot quantity is base-asset
  quantity and price is quote units per base unit. No floating-point intermediate.
  Internal units use `u128` (0 through `2^128 - 1`); price rejects zero, while
  incremental quantity permits zero for deletion. Scales support 0 through 18
  decimal places. Signed wire tails are validated before conversion; precision
  loss and overflow are rejected. The conversion factor `10^abs(exponent + scale)`
  must itself fit, including for zero values and empty groups, so shifts beyond
  magnitude 38 are unsupported. Snapshot quantities must be positive.
- SBE stream `DepthDiffStreamEvent` decoding, including symbol, sequence range and
  event timestamp validation; a caller-owned `sbe::Decoder` with reusable level storage.
- WebSocket API wrapper, depth snapshot, rate-limit and error-response decoding.
  Snapshot request IDs must match; errors retain UTC cooldown timestamps.
- Bounded JSON snapshot request construction for setup/recovery only.
- A pure sequence gate with generation/cycle rejection and explicit application
  acknowledgement. It does not mutate books or label consumer data `Live/Fresh`.
- Independent synthetic fixtures, offline replay, generated interval tests,
  deterministic mutation/random smoke fuzzing, microbenchmarks and allocation probe.

**Not implemented:** sockets/credentials, persistent-session lifecycle, supervision,
heartbeats, global admission, bounded async queues, snapshot-await buffering, retries,
watchdogs/freshness, shared L2 book, publication, CPU affinity or SPSC queues. This is
a first protocol component, not a completed Phase 1 connector or production feed.

The existing [ownership](engineering.md#single-writer-is-the-default-ownership-model),
[connection](connections.md), [security](security.md) and [evidence](testing.md)
contracts still apply. No acceptance gate is relaxed by this component's scope.

## Protocol Sources and Supported Layouts

Checked against official Binance sources on 2026-10-08:

- [SBE market streams](https://github.com/binance/binance-spot-api-docs/blob/master/sbe-market-data-streams.md)
- [Stream schema 1:0](https://github.com/binance/binance-spot-api-docs/blob/master/sbe/schemas/stream_1_0.xml)
- [API schema 3:5](https://github.com/binance/binance-spot-api-docs/blob/master/sbe/schemas/spot_3_5.xml)
- [Production API schema lifecycle](https://github.com/binance/binance-spot-api-docs/blob/master/sbe/schemas/sbe_schema_lifecycle_prod.json)
- [SBE request/response configuration](https://github.com/binance/binance-spot-api-docs/blob/master/faqs/sbe_faq.md)
- [Snapshot alignment and continuity](https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-streams.md#how-to-manage-a-local-order-book-correctly)

| Message | Schema | Template | Root bytes | Group entry/count encoding |
| --- | --- | --- | --- | --- |
| Diff Depth | 1:0 | 10003 | 26 | 16 bytes, uint16 count |
| WS wrapper | 3:5 | 50 | 3 | rate limits: 19 bytes, uint16 count |
| Nested depth response | 3:5 | 200 | 10 | 16 bytes, uint32 count |
| Nested error response | 3:5 | 100 | 18 | no groups |

All integers are little-endian. The WS wrapper contains a string request ID and a
length-delimited nested SBE message; **the depth snapshot itself has no symbol**.
The future session must bind each unique in-flight request ID to full instrument
identity, connection generation and sync cycle before delivering it to the owner.
Echoed IDs do not establish provenance by themselves.

The decoder deliberately rejects unknown schema versions/templates/block lengths,
bad UTF-8, invalid numerics, mismatched IDs/symbols, trailing bytes, excess groups
and excess levels. It does not attempt untested forward-compatible parsing. This
fail-closed policy needs an operational schema-update procedure before live use.
Future SBE compatibility must use the extension rules; unknown required input must
never silently continue as a successful update. A deprecated-schema flag is exposed.

API response errors preserve `retryAfter` and `serverTime` as **absolute UTC
microseconds**. The future manager must derive a conservative cooldown with clock
alignment, not subtract these directly from a local monotonic clock.

The stream uses `<symbol>@depth` at 20ms. `@depth20` is a top-20 snapshot product,
not the initial snapshot for an advertised deeper book. `@bestBidAsk` allows server
auto-culling. Control requests/ACKs/server-shutdown events still use JSON. API
schema 3:5 and stream schema 1:0 are separate codecs and version lifecycles.

## Sequence Gate Integration Boundary

The writer is the sole caller of `SequenceGate`. Inputs arriving before a snapshot
return `AwaitSnapshot`; **the caller must retain these inputs in a count/byte-bounded
buffer**, not discard them. That buffer and its overflow/failure path are not supplied
by this protocol component. After snapshot application/commit, replay buffered ranges
in original stream order. Covered ranges are duplicates; advancing ranges must cover
the next required update ID. A gap invalidates the cycle. Overlapping advancing
ranges are supported; out-of-order uncovered input is not reordered to hide a gap.

`Decision::Apply` stages an opaque commit. Apply the corresponding full snapshot or
increment to the book before calling `commit`. No next action can be staged until
commit; on decode, application, overflow or transport failure call `invalidate` and
publish unusable state through the future owner's independent failure path. Restart
requires a strictly newer instrument sync cycle and a nondecreasing connection
generation. This component does not generate IDs or perform automatic retries.

`Following` means only that the accepted sequence has bridged the snapshot. Book
validity, queue age, inactivity and consumer view expiry remain separate requirements.
The offline example acknowledges diagnostic actions without constructing a book and
explicitly does not publish `Live/Fresh`.

## Memory, Strings and CPU Scope

All input bytes are borrowed. Symbol/request ID/error text and rate-limit entries
borrow the complete message; a consumer cannot retain them after buffer reuse.
One symbol identity comparison is necessary in this single-instrument baseline;
there is no per-message owned string creation, normalization or hash-map lookup.
The later router will resolve full identity to compact IDs during setup/decoding.

`sbe::Decoder::new(limits)` binds fixed limits to its private level buffer.
Use `decode_depth(input, expected_symbol, scales)` and
`decode_snapshot(input, expected_request_id, scales)` on the decoder; returned
views borrow it, so a view must finish being used before the next decode.
`DepthUpdate` and successful `Snapshot` each retain one shared `market_data::Scales`
for all levels. `Price` and `Quantity` store only `u128` units; their equality is
units-only and requires matching scale contexts. Keep copied levels with their
batch/instrument context. The future writer/book must validate instrument identity
and both scales before application, and publish scales with consumer views. Scale
changes require controlled rebuilding or exact conversion; wire exponents may vary
while normalization targets stay fixed. No book-level validation is implemented yet.

`DecodeLimits` explicitly configures complete-message bytes, total bid/ask levels,
symbol bytes and rate-limit entries. One reusable Vec reserves `max_levels` slots
at setup for the combined bid/ask count. Bids and asks use the same group decoder:
append bids, record the split, then append asks. Returned sides borrow slices
without copying levels or inserting ahead of asks. Decoded totals cannot exceed
the configured limit. Reserved level storage is
`max_levels * size_of::<Level>()`, plus Vec/split metadata. On this target `Level` is
32 bytes after moving scales to batch context. Setup allocation can fail explicitly.
Warmed within-bound decoding reuses
that allocation. Cleanup occurs on the owning caller, with no pool or deferred reclaim.
Request JSON serialization may allocate during recovery until its output buffer is
large enough; this is outside increment decoding and is not measured as free.

Decode cost is O(levels + symbol/ID text bytes + rate-limit entries), with one numeric
conversion per price and quantity. Price and quantity conversion factors are
prepared once per batch; each level still checks sign, range and exact precision.
Both Diff Depth (stream schema 1:0) and snapshot responses (API schema 3:5)
use official SbeTool 1.35.6 generated Rust crates. Java is used only for
regeneration; see [stream provenance](../generated/binance-stream/README.md) and
[API provenance](../generated/binance-api/README.md). There is no handwritten
field/level/rate-limit decoder or retained handwritten test decoder in the repo.

Generated accessors assume valid lengths. A checked constant-work framing layer
validates supported headers, roots, dimensions, combined counts, variable-length
spans and complete-message consumption. Error responses check the root and each
variable-field span before the corresponding accessor, avoiding a second read of
the error message length. Framing scalar reads
also use generated accessors, after checking their spans. This layer skips group
bytes without copying or interpreting domain values. Exact numeric conversion,
identity/enum checks, snapshot ordering and error cleanup remain in Strait.
Multiply-invalid input can report a framing error before a numeric error.

The offline [comparison helper](../examples/spot_codec_digest.rs) was run against
the retained pre-migration source and generated implementation: acceptance and
decoded-value digests matched for both small/128-level depth fixtures, small API
snapshot/error fixtures, all their truncations and single-byte mutations, and a
10,000-level snapshot. Independently specified fixture and boundary/enum/time tests
provide separate correctness checks. The old handwritten implementation exists
only in temporary baseline artifacts, not in project code. Real recorded payloads
and longer coverage-guided fuzz campaigns remain evidence gaps. No unsafe
production code or speculative parser optimization was introduced; local evidence
and outstanding gates are in [the migration report](spot-sbe-generated.md).

## Reproduction

From the repository root, using the locked dependencies:

```sh
python3 tests/fixtures/binance/spot/generate.py
cargo test --locked --test spot_protocol
cargo run --locked --example spot_replay
cargo run --locked --release --example spot_fuzz_smoke
# Compare the digest output from the same helper against a retained baseline build.
cargo run --locked --release --example spot_codec_digest
cargo bench --locked --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2
cargo run --locked --release --example spot_profile -- 1
```

`spot_fuzz_smoke` mutates every byte through all 256 values in the three small
fixtures and runs 20,000 fixed-seed random cases up to 8191 bytes, including numeric
conversions. It is a runnable bounded smoke target, **not coverage-guided libFuzzer**.
Longer coverage-guided campaigns and minimized real-input regressions are pending.

On macOS, after building the release probe, capture CPU separately from baseline runs:

```sh
target/release/examples/spot_profile 6 > artifacts/spot-sbe/profiled-probe.txt &
spot_probe_pid=$!
/usr/bin/sample "$spot_probe_pid" 15 -file artifacts/spot-sbe/cpu-sample.txt
wait "$spot_probe_pid"
```

The dev-only probe's GlobalAlloc wrapper delegates unchanged operations to System
and records atomic counters without allocating. Unsafe is limited to the mandatory
allocator interface; production decoding is safe Rust. Its safety rationale is in
the example. Probe timestamps/counters and sampling perturb timing, so probe rates
are diagnostic, not acceptance latency. See the [initial local report](spot-sbe-baseline.md).
