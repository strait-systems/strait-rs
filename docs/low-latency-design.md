# High-frequency & Low-latency Design

Strait targets predictable receipt-to-view tail latency under sustained and burst
market-data load. Today, only the **offline Binance Spot SBE protocol core** is
implemented. USD-M, network sessions, books and publication remain future work.
This walkthrough connects the current design to code and evidence; detailed
requirements remain in the [engineering](engineering.md), [latency](latency.md)
and [testing](testing.md) contracts.

## Implemented Path

```mermaid
flowchart LR
    A[Complete borrowed SBE message] --> B[Layout and resource-limit preflight]
    B --> C[Generated field accessors]
    C --> D[Checked numeric conversion]
    D --> E[Reusable level buffer]
    E --> F[Borrowed decoded view]
    F --> G[Caller invokes sequence gate]
    G --> H[Staged action]
    H --> I[Caller acknowledges application]
```

The sequence gate is a separate caller-driven step. The
[offline replay](../examples/spot_replay.rs) acknowledges diagnostic actions;
it does not apply them to a book or publish a usable view.

| Design | Why it matters for repeated updates | Tradeoff and implementation |
| --- | --- | --- |
| Schema-generated SBE accessors behind checked framing | Keeps wire layouts schema-derived while rejecting unsupported layouts, truncated spans and excessive counts before accessor use | Preflight adds work; generated accessors alone are unsafe on arbitrary input. Exact-layout rejection needs schema-update operations before live use. See [depth](../src/connector/binance/spot/depth.rs) and [snapshot](../src/connector/binance/spot/snapshot.rs). |
| Exact integer values and batch-prepared scale conversion | Avoids floating-point rounding and prepares price/quantity conversion factors once per message rather than per level | Per-level sign, overflow and precision checks remain. `u128` values consume storage; no claim that they are the fastest representation. See [numeric types](../src/market_data.rs). |
| One reserved level buffer, split into bid/ask slices | Reuses storage within configured bounds and returns views without extra level copies or per-message owned strings | Reserves memory upfront; views borrow the decoder and input, preventing reuse until consumption finishes. Async handoff ownership is still to be designed. See [Decoder and LevelBuffer](../src/connector/binance/spot/sbe.rs). |
| Explicit stage/apply/commit sequence protocol | Prevents sequence state advancing before application is acknowledged; gaps invalidate the cycle and old-cycle inputs are ignored | The caller must apply the entire action before commit and invalidate on failure. Snapshot-await buffering and consumer invalidation are not implemented. See [SequenceGate](../src/connector/binance/spot/sync.rs). |

Decode work grows with levels, validated text bytes and rate-limit entries. Layout
preflight skips level spans without decoding each entry; numeric conversion still
visits every level. High-frequency relevance comes from bounding and understanding
repeated work, not from a demonstrated production message-rate guarantee.

## Performance Evidence

Current evidence covers offline component decoding on synthetic fixtures:

- **Allocation:** the [probe](../examples/spot_profile.rs), with 16 reserved levels,
  records one 512-byte setup allocation, zero warmed allocation/reallocation/free
  calls for the small depth/snapshot/error fixtures, and one 512-byte cleanup free.
  Transport/runtime costs and large-snapshot memory behavior are outside this probe.
- **Decode timing:** the [migration report](spot-sbe-generated.md#remaining-handwritten-decoder-removal)
  retains local Criterion results, environment, workload and commands. It compares
  a retained pre-migration reader with the generated-code implementation. Results
  are mixed, including measured small-snapshot/error regressions; they do not
  establish a general speedup. Historical baseline sources and raw logs were kept
  in local temporary directories, so that comparison is not fully reproducible
  from this checkout.
- **Measurement limits:** Criterion iteration estimates are not receipt-to-view
  p99/p99.9. Probe rates include counter/clock overhead and are not acceptance
  throughput. CPU sampling failed due to inspection permissions; no successful
  CPU profile is available. Reference-runner and integrated acceptance remain open.

Reproduce current component measurements from the repository root:

```sh
cargo bench --locked --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2
cargo run --locked --release --example spot_profile -- 1
```

Correctness evidence includes independently specified synthetic fixture values,
truncation/limit/numeric/enum tests and generated sequence intervals in
[protocol tests](../tests/spot_protocol.rs), plus deterministic
[fuzz smoke](../examples/spot_fuzz_smoke.rs). Synthetic fixtures do not establish
compatibility with real captured exchange traffic. See [protocol scope and commands](spot-sbe.md).

## Planned Receipt-to-view Pipeline

One logical writer per instrument will own book mutation, sequence/cycle state
and ordered publication. Readers and snapshot workers will supply bounded,
cycle-tagged inputs; consumers will receive immutable views. This is the
[ownership contract](engineering.md#single-writer-is-the-default-ownership-model),
not a completed pipeline.

The next integration must measure queue residence, decode/apply/publication and
consumer-observed age under sustained, burst and overload conditions. Sequence
validity alone will not imply freshness: inactivity detection and consumer age
checks must work without new depth data. See [architecture](architecture.md),
[connection requirements](connections.md) and the [roadmap](roadmap.md).

Reference-runner tails, CPU time/headroom and hardware counters, syscall/kernel
attribution, real captured payloads, coverage-guided fuzzing and integrated
fault/soak evidence remain open. SBE versus JSON and persistent WebSocket snapshots
versus warmed HTTP have no measured comparison here. Production readiness and a
completed Phase 1 connector are not claimed.
