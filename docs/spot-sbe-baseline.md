# Initial Spot SBE Local Evidence

Recorded 2026-10-08. **Evidence incomplete for connector/production acceptance.**
This is a first-component baseline, not a WebSocket-versus-REST comparison or a
receipt-to-view measurement. No numerical operating budget has been agreed.

## Environment and Revision

- Base commit: `eb084f9ee4348b3931f44285e701626adb8b132b` (placeholders, no executable
  protocol component to benchmark). Candidate: the working-tree implementation
  accompanying this report; SHA-256 manifest in local `artifacts/spot-sbe/source-manifest.json`.
- macOS 15.7.7, build 24G720, x86_64; Intel Core i5-8257U @ 1.40GHz; 8 GiB RAM;
  4 physical / 8 logical CPUs. This development laptop is not a selected acceptance runner.
- rustc 1.97.1, LLVM 22.1.6; Cargo 1.97.1; default release/bench optimization.
  No task runtime, affinity, custom target flags or production queues in the measured core.
- Power/frequency policy, unrelated background work and thermal state were not controlled.
- Criterion 0.7, 30 samples per case, 1s warm-up, 2s collection target; two final
  sequential unprofiled runs, named `spot-initial-2` and `spot-initial-3`.
- Fixtures are deterministic synthetic wire messages from
  `tests/fixtures/binance/spot/generate.py`, not representative exchange recordings.
  Stage benchmarks are closed-loop; they do not measure burst backlog or offered load.

## Component Baselines

Central Criterion estimates below are operation-time estimates, **not p99/p99.9**.
Both final runs used 256 KiB input and 10,000 total-level limits, reserved at setup.
Criterion retains sample/estimate data under `target/criterion/`.

| Case | Run 2 | Run 3 |
| --- | --- | --- |
| Increment, 3 levels | 82.092 ns | 82.260 ns |
| WS snapshot, 3 levels | 114.35 ns | 114.98 ns |
| WS rate-limit error | 69.156 ns | 69.928 ns |
| Increment, 128 levels | 1.9847 us | 2.1102 us |
| WS snapshot, 10,000 levels | 167.50 us | 168.12 us |

There are outliers and visible run variation (particularly 128-level inputs).
These estimates establish only a local diagnostic starting point, not a supported
feed rate. Initial exploratory runs overlapped probes/builds or earlier correctness
changes and were excluded from the final series; their automatic Criterion
"regression" labels are not comparisons under matching conditions.

Exact final commands:

```sh
cargo bench --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline spot-initial-2
cargo bench --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline spot-initial-3
```

Raw output is local at `artifacts/spot-sbe/benchmark-run-2.txt` and
`benchmark-run-3.txt`. No artifacts have been uploaded.

## Resource Probe and CPU Sample

The small-message allocation probe used 16 total-level slots, preallocated per side.
It measured setup, three warmed paths, and destruction separately. This allocation
result is scoped to these paths; it does not include a transport, runtime, JSON
request creation, books, or consumer publication.

| Scope | Observed allocation behavior |
| --- | --- |
| Buffer setup | 2 allocations, 2048 bytes, no reallocations |
| Warmed 3-level increment | 0 allocations/reallocations/frees |
| Warmed 3-level WS snapshot | 0 allocations/reallocations/frees |
| Warmed rate-limit error | 0 allocations/reallocations/frees |
| Buffer destruction | 2 frees, 2048 bytes |

The 1s-per-path run processed 9,646,000 increments, 9,283,000 snapshots and
16,979,000 errors. These are **closed-loop diagnostic counts**, not offered-load
capacity or tail-latency evidence. Timestamp and atomic instrumentation remain in
the probe. Raw results: `artifacts/spot-sbe/allocation-probe.txt`.

CPU sampling used `/usr/bin/sample`, 15s at 1ms sampling interval, while each probe
path ran for 6s. The CPU call graph contains depth/snapshot decoding, level decoding,
exact numeric conversion and UTF-8 validation, plus probe/clock work. It supports
investigating those costs but does not establish any cost as a production bottleneck.
The 10,000-level case was benchmarked but not sampled in this initial CPU probe.
The first sandboxed attach failed; the authorized local attach succeeded. Profile:
`artifacts/spot-sbe/cpu-sample.txt`; sampled probe output:
`artifacts/spot-sbe/profiled-probe-elevated.txt`.

## Correctness and Review Disposition

- 11 integration tests passed, including independently specified decoded values,
  exact scaling/range failures, every truncated small fixture, total-level limits,
  snapshot ordering/duplicate rejection, stale cycles, pending commits, and gaps.
- Generated sequence tests cover 26,240 interval cases against an independent range
  oracle. Diagnostic replay reaches `Following` without publishing a usable book.
- The deterministic smoke fuzzer passed 100,128 cases with initial seed
  `0x535452414954`. It exercises byte substitutions, arbitrary bounded bytes,
  oversized input and numeric values, with valid decoder reuse after malformed storms.
- Formatting, Clippy with warnings denied, all-target/all-feature tests, doc tests,
  rustdoc with warnings denied and benchmark smoke checks passed locally on Rust 1.97.1.
  Rust 1.88 is configured in existing CI but not installed/tested locally.
- No dependency or lockfile changes; only a benchmark target was registered. Existing
  CI already builds all benchmark targets. No CI policy or secret handling was changed.

Contract/security review traced borrowed lifetimes, count/byte limits, exact types,
safe indexing and ordered commit/cycle gates. No demonstrated shared-book/multi-writer,
silent truncation or input-driven panic was found in this component. There is no
implemented async boundary or consumer publication to certify. Handwritten codec
acceptance remains pending an official/generated reference comparison, rather than
claiming a measured improvement without a baseline alternative.

Unresolved evidence: official generated-code/recorded-payload differential validation,
coverage-guided fuzz campaigns, Linux/reference-runner results, hardware counters,
per-thread CPU-time/headroom/idle budgets, syscall/kernel captures, integrated
independent-arrival tests, freshness/watchdogs, recovery/soak tests and p99/p99.9.
Network components and L2 storage are absent, so reference-book replay, live API-key
validation and WebSocket-versus-REST recovery comparisons are not yet executable.
Passing this local protocol baseline does not complete Phase 1 or certify readiness.

## Decoder Ownership Refactor (2026-10-08)

`Decoder` now owns fixed `DecodeLimits` and private reusable `LevelBuffer` storage.
Public decoding uses `decode_depth` / `decode_snapshot`; returned views retain the
same input/storage borrowing constraint. Parsing, exact conversions, failure cleanup
and configured limits are preserved. No dependency, queue, lock or transport change.

Baseline is the pre-refactor dirty working-tree implementation, not HEAD. An isolated
local source copy restores the original buffer API while retaining all other user
changes. Source hashes and raw logs are in `/tmp/strait-decoder/`; these temporary
artifacts are not uploaded. Environment, fixtures, limits and Criterion settings
match the initial report above. Two candidate runs preceded two verified baseline
runs; runs were sequential, not alternated, and thermal/background state was not
controlled. Earlier overlapping/exploratory runs are excluded.

Central operation-time estimates (not tail percentiles):

| Case | Baseline 1 / 2 | Decoder 1 / 2 |
| --- | --- | --- |
| Increment, 3 levels | 86.124 / 82.016 ns | 85.189 / 83.635 ns |
| WS snapshot, 3 levels | 114.77 / 112.39 ns | 117.45 / 115.32 ns |
| WS rate-limit error | 70.537 / 69.910 ns | 72.120 / 72.916 ns |
| Increment, 128 levels | 1.9893 / 2.0047 us | 1.9990 / 2.0368 us |
| WS snapshot, 10,000 levels | 176.65 / 172.56 us | 171.04 / 167.68 us |

Small snapshot/error cases show slightly higher candidate estimates; other cases
have overlapping variation. **Performance disposition: inconclusive**, with no
agreed regression tolerance or stable runner. This is not evidence of zero overhead
or production latency acceptance. Further stable-runner alternating measurements
are needed to resolve small differences; no optimization is justified by these data.

Both `spot_profile -- 1` allocation probes observed two setup allocations (2048
bytes), zero warmed allocations/reallocations/frees for all three small-message
paths, and two cleanup frees (2048 bytes). Probe throughput is not acceptance data.
No new CPU sampling or syscall/hardware-counter capture was taken; existing resource,
reference-codec, fuzz-campaign and integrated p99/p99.9 gaps remain unresolved.

Commands (baseline uses the isolated manifest and the same local target directory):

```sh
cargo bench --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline decoder-after-1
cargo bench --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline decoder-after-2
CARGO_TARGET_DIR="$PWD/target" cargo bench --manifest-path /tmp/strait-decoder/baseline-source/Cargo.toml --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline decoder-verified-before-1
CARGO_TARGET_DIR="$PWD/target" cargo bench --manifest-path /tmp/strait-decoder/baseline-source/Cargo.toml --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline decoder-verified-before-2
cargo run --locked --offline --release --example spot_profile -- 1
```

Raw logs: `after-1.txt`, `after-2.txt`, `verified-before-1.txt`,
`verified-before-2.txt`, `before-allocation.txt`, `after-allocation.txt` and
`source-manifest.json` under the temporary artifact directory.

Contract/security review found no definite violation introduced by this refactor:
limits remain bound to reserved capacity, borrowed views prevent overwrite, and all
validation/failure cleanup remains intact. Formatting, warnings-denied Clippy,
all-target/all-feature tests (11 integration tests plus benchmark smoke), doc tests,
warnings-denied rustdoc, offline replay and the 100,128-case fuzz smoke passed.
Documentation links were checked. This refactor does not complete connector or
production acceptance.

## Per-batch Scale Layout Change (2026-10-08)

Baseline is the complete pre-change dirty working tree, preserved under
`/tmp/strait-scale-baseline-source`; it is not the placeholder HEAD. Candidate
removes per-value scale and retains one `market_data::Scales` in each decoded depth
update/snapshot. No dependency, wire conversion, input limit or sequence change.
Same base commit and local x86_64-apple-darwin toolchain as above; release/bench
settings and fixtures unchanged. No runtime, queues or book in this measurement.
CPU model query was sandbox-denied in this run; hardware is the previously recorded
local laptop, not a reference acceptance runner. Frequency/power/background load
were uncontrolled. Source SHA-256 manifests and copied raw logs are in
`/tmp/strait-scale-evidence/`; Criterion samples remain under `target/criterion/`.

Central iteration-time estimates (not latency tails):

| Case | Before 1 | After 1 (contended) | Before 2 | After 2 | After 3 |
| --- | --- | --- | --- | --- | --- |
| depth_3_levels | 91.831 ns | 271.70 ns | 117.09 ns | 96.452 ns | 106.12 ns |
| ws_snapshot_3_levels | 115.24 ns | 427.88 ns | 176.87 ns | 134.20 ns | 115.08 ns |
| ws_rate_limit | 76.816 ns | 91.103 ns | 95.031 ns | 80.737 ns | 76.356 ns |
| depth_128_levels | 1.7653 µs | 2.2234 µs | 2.3479 µs | 2.6128 µs | 1.9114 µs |
| ws_snapshot_10000_levels | 199.81 µs | 422.11 µs | 197.70 µs | 173.90 µs | 175.95 µs |

After 1 overlapped other validation builds and is unsuitable for attribution.
Before 2, After 2 and After 3 were run sequentially without our other build/probe
jobs. Large run variation remains; no stable decoding-latency improvement or
production regression acceptance is established. Evidence status: **inconclusive
for timing, verified reduction in level storage**. No numerical budget is changed.

On this target, Price/Quantity now measure 16 bytes each and Level 32 bytes
(previous Level 64). The allocation probe with 16 reserved slots per side reports
setup allocations unchanged at 2, allocated bytes 2048 → 1024; cleanup frees
unchanged at 2, freed bytes 2048 → 1024. Warmed depth, snapshot and error paths
all retain zero allocation/reallocation/free calls. Decoder reservation at the
benchmark's 10,000-level bound falls from 1,280,000 to 640,000 level-storage bytes,
excluding Vec and batch metadata. The probe now prints actual layout sizes.

Reproduction (30 samples, 1s warm-up, 2s collection per fixture):

```sh
cargo bench --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline scale-before
CARGO_TARGET_DIR="$PWD/target" cargo bench --manifest-path /tmp/strait-scale-baseline-source/Cargo.toml --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline scale-before-repeat
cargo bench --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --baseline scale-before-repeat
cargo run --locked --offline --release --example spot_profile -- 1
```

Baseline source/logs are temporary local artifacts and must be retained separately
if this comparison is needed after cleanup. Allocation counters/timestamps perturb
the probe, so its rate is diagnostic. CPU sampling, hardware counters, cache/copy
traffic, per-thread CPU/headroom, syscall/kernel attribution and integrated
receipt-to-view tails were not measured for this change. USD-M has no implemented
numeric path to compare. Existing transport/book/publication and reference-runner
gaps remain; this does not establish production readiness.

Validation: formatting, Clippy with warnings denied, all-target/all-feature tests
(13 protocol tests and benchmark smoke), doctests, rustdoc with warnings denied,
and 100,128 deterministic fuzz-smoke cases passed. New tests check both batches'
shared scale context, independently specified normalized values, zero deletion and
buffer reuse across different normalization targets. Contract and security review
found no definite in-scope violation: checked conversion and rejection behavior
remain intact, borrowing still prevents premature buffer reuse, and no new unsafe
code, allocation site or concurrency boundary is introduced. Units-only equality
requires matching contexts; future book integration must enforce instrument/scale
validation and retain scales with published views. That book is not implemented.

## Prepared Batch Conversion (2026-10-08)

The baseline is the complete pre-optimization working tree copied to
`/tmp/strait-conversion-before`, preserving existing user changes rather than
using placeholder HEAD. Candidate prepares price and quantity conversion factors
once per decoded batch, then retains checked per-level conversion. Public
single-value conversion semantics, input limits, borrowed views, zero deletion,
snapshot checks and failure cleanup remain unchanged. No dependency, unsafe code,
allocation site, queue or book storage change was introduced.

Environment: the previously recorded local Intel x86_64 macOS laptop, rustc
1.97.1 / LLVM 22.1.6, default release/bench settings, Criterion 0.7, synthetic
fixtures and limits from `benches/spot_sbe.rs`. Power, thermal state and unrelated
background load were uncontrolled. No runtime, transport, book or publication is
measured. Source hashes, toolchain/environment records, raw logs and validation
output are local in `/tmp/strait-conversion-evidence/`; the baseline target
directory is `/tmp/strait-conversion-before-target`.

Two clean pairs ran sequentially in before/after/before/after order, with no other
agent build/probe jobs overlapping. An exploratory first baseline overlapped a
profile build and is excluded. Each case used 30 samples, 1s warm-up and 2s
collection target. Central iteration-time estimates, **not tail percentiles**:

| Case | Before 1 / 2 | After 1 / 2 |
| --- | --- | --- |
| Increment, 3 levels | 83.944 / 98.245 ns | 71.399 / 68.245 ns |
| Snapshot, 3 levels | 114.69 / 159.78 ns | 98.172 / 100.02 ns |
| Rate-limit error | 70.136 / 96.980 ns | 73.445 / 72.274 ns |
| Increment, 128 levels | 1.6335 / 2.1557 us | 1.0788 / 1.1349 us |
| Snapshot, 10,000 levels | 136.01 / 181.46 us | 98.291 / 98.739 us |

Both pairs show lower candidate estimates for level-bearing messages, especially
large batches. The unchanged error path and baseline variation show environmental
noise; small differences and the precise percentage benefit cannot be accepted as
stable-runner guarantees. Evidence supports local decoder improvement, with
production performance acceptance incomplete and no agreed numerical budget.

Reproduction, for rounds 1 and 2 (run one command at a time):

```sh
CARGO_TARGET_DIR=/tmp/strait-conversion-before-target cargo bench --manifest-path /tmp/strait-conversion-before/Cargo.toml --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline conversion-clean-before-1
cargo bench --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline conversion-clean-after-1
/tmp/strait-conversion-before-target/release/examples/spot_profile 1
target/release/examples/spot_profile 1
```

Before/after allocation probes both observed two setup allocations / 1024 bytes
with 16 slots per side, zero warmed allocations/reallocations/frees on the three
small-message paths, and two cleanup frees / 1024 bytes. Level remains 32 bytes.
Probe counts are closed-loop diagnostics, not capacity evidence. Reusable level
storage, symbol validation and copy sites are unchanged; no copied-byte/cache
traffic measurement was taken.

A candidate CPU sample was attempted using `spot_profile 6` and
`/usr/bin/sample "$conversion_probe_pid" 15 -file /tmp/strait-conversion-evidence/after-cpu-sample.txt`.
Sandbox attach failed; no CPU profile was produced and no privilege change was
attempted. Source inspection establishes removal of per-level factor preparation,
but does not establish its measured CPU share. CPU-time/headroom, hardware counters,
syscall/kernel attribution, generated-code differential validation, coverage-guided
fuzzing, reference-runner and integrated p99/p99.9 evidence remain gaps. USD-M has
no implemented numeric decoder to benchmark.

Validation: formatting, warnings-denied Clippy, all-target/all-feature tests
(one new numeric differential test plus 13 protocol tests and benchmark smoke),
doctests, warnings-denied rustdoc, and 100,128 fixed-seed fuzz-smoke cases passed.
The differential test compares the prepared rule with the original formula across
all 256 wire exponents, 19 supported scales and nine signed mantissa boundary cases,
including precision, overflow, zero and negative rejection. Contract/security
review found no definite in-scope violation: batch exponent validation remains
before level reads, failures clear storage, exact arithmetic and ownership remain
intact. No production readiness or Phase 1 completion is implied.

## Shared Bid/Ask Buffer Experiment — Reverted (2026-10-08)

Baseline: the complete working tree after prepared batch conversion, copied to
`/tmp/strait-buffer-before`; existing user changes were preserved. Candidate
replaced two side Vecs with one Vec and a bid-count split, appending bids then asks.
Public borrowed slices, all numeric/sequence rules, total count limits and failure
cleanup were preserved. No unsafe code, dependency, queue or book change was made.

**Decision: revert the storage change.** The allocation reduction was verified,
but focused local measurements repeatedly showed a decode-time regression for
128-level increments. No demonstrated memory pressure or agreed latency tradeoff
justifies adopting this change for the current low-latency target. The prepared
conversion optimization remains; new total-limit/side-split/reuse tests are retained.

Environment matches the prior local report: Intel x86_64 macOS laptop, rustc
1.97.1 / LLVM 22.1.6, Criterion 0.7, default bench/release build, synthetic existing
fixtures and decoder limits. Frequency, thermal state and background load were
uncontrolled. No transport/runtime/book/publication is measured. Two full benchmark
pairs alternated before/after/before/after with no other agent build/probe work
overlapping. Each case used 30 samples, 1s warm-up and 2s collection.
Central iteration-time estimates, **not p99/p99.9**:

| Case | Before 1 / 2 | Candidate 1 / 2 |
| --- | --- | --- |
| Increment, 3 levels | 65.361 / 68.201 ns | 64.176 / 64.535 ns |
| Snapshot, 3 levels | 97.693 / 100.46 ns | 99.071 / 97.072 ns |
| Rate-limit error | 71.727 / 89.313 ns | 71.015 / 73.292 ns |
| Increment, 128 levels | 1.0478 / 1.0848 us | 1.0837 / 1.2701 us |
| Snapshot, 10,000 levels | 89.793 / 99.164 us | 96.093 / 96.455 us |

Focused 128-level runs used 50 samples, 2s warm-up and 3s collection, again alternating
before/after twice. Before central estimates: 1.0146 / 1.0193 us; candidate:
1.1048 / 1.0615 us. The second pair's confidence intervals were separated
(before 1.0155–1.0238 us, candidate 1.0575–1.0670 us), indicating a local slowdown
of about 4%. A follow-up using a local mutable Vec reference inside the group loop
still measured 1.0593 us (interval 1.0571–1.0617 us). No production budget or stable
reference-runner guarantee is inferred; larger run noise remains visible elsewhere.

Allocation probes with a total bound of 16 levels verified setup allocations
2 → 1, requested bytes 1024 → 512, and cleanup frees/bytes 2/1024 → 1/512.
All three warmed small-message paths had zero allocations/reallocations/frees.
Level size remained 32 bytes. At the benchmark's 10,000-level bound, nominal
reserved level bytes would fall 640,000 → 320,000, excluding metadata and allocator
overhead; RSS and multi-decoder working-set effects were not measured.
CPU/cache/copy/syscall attribution and integrated tails remain unmeasured;
the previous sandbox CPU-sampling failure remains an evidence gap. No CPU cause
is asserted for the observed timing change. USD-M numeric decoding is absent.

Raw logs, toolchain/environment, baseline/candidate source hashes and validation
output are local in `/tmp/strait-buffer-evidence/`. The source manifest records
the initial experimental candidate, not the restored final source. Baseline files
are under `/tmp/strait-buffer-before`; compiled baseline artifacts reused
`/tmp/strait-conversion-before-target` with a forced rebuild of changed sources.
No artifact was uploaded or global tool installed.

Exact benchmark commands (repeat with round suffix 2; focused runs add
`depth_128_levels` and use 50 samples / 2s warm-up / 3s measurement):

```sh
CARGO_TARGET_DIR=/tmp/strait-conversion-before-target cargo bench --manifest-path /tmp/strait-buffer-before/Cargo.toml --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline buffer-before-1
cargo bench --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline buffer-after-1
/tmp/strait-conversion-before-target/release/examples/spot_profile 1
target/release/examples/spot_profile 1
```

The experimental candidate passed formatting, warnings-denied Clippy,
all-target/all-feature tests, doctests, warnings-denied rustdoc, and 100,128
deterministic fuzz-smoke cases. New independent wire cases cover either side
occupying the full total limit, uneven splits, empty/zero-capacity messages,
one-level overflow and reuse after a failure following both groups.
Contract/security review found no definite invariant violation in the experiment;
the measured performance cost drove rejection. Final restored code retains the
new tests; relevant checks were repeated after restoration. Existing generated
reference-codec, coverage-guided fuzzing, integrated/reference-runner evidence and
production-readiness gaps remain unchanged.

## Shared Buffer Retry — Group Decoder Adopted (2026-10-08)

This section supersedes the preceding reverted experiment's implementation decision,
without discarding its measurements. Baseline is the complete current working tree
with prepared conversions and two side Vecs, preserved at
`/tmp/strait-buffer-retry/before`. Existing user changes were retained. Two
single-Vec candidates were preserved: the earlier side-index loop at `after/`,
and the adopted group-function version at `group/`. The latter decodes bids,
records their length as the split, then calls the same group decoder for asks.
Each group checks existing length plus incoming count against the combined bound.
Numeric conversion, snapshot order checks, failure cleanup, public borrowed slices
and decoder APIs remain intact. No dependency, unsafe, async or book change.

Default release/bench build on the same local Intel x86_64 macOS laptop, rustc
1.97.1 / LLVM 22.1.6, Criterion 0.7 and existing synthetic fixtures/limits. Power,
thermal state and background load were uncontrolled. Before/after/group benchmark
executables were copied separately after forced source rebuilds; hashes and retained
source copies prevent confusion from shared target artifacts. All timed binaries
were run sequentially, without agent compilation/profiling overlapping; plots were
disabled. No integrated runtime/transport/book/publication path was measured.

The original side-index version was retested twice in before/after/after/before
order, with 100 samples, 3s warm-up, 5s measurement on 128-level increments.
Before central estimates: 1.0543, 1.3952, 1.2395, 1.0523 us.
Candidate: 1.1251, 1.1988, 1.2070, 1.1200 us.
The large environmental variation does not establish a stable universal 4% penalty.
The earlier slowdown remains local evidence, not a property of all single Vecs.

Release assembly inspection found an extra stack byte load
(`movzbl -42(%rbp), %r10d`) in the side-index candidate's per-level loop tail.
It is absent in the two-Vec loop tail and in the group decoder's corresponding
loop tail. The group decoder compiles as a separate function, called twice per
message; its loop does not carry a bid/ask side index. Register allocation and
other instructions also differ. This supports investigating generated-code costs,
but **does not prove that this one load caused the measured difference**. Static
assembly line/instruction counts include error paths and are not executed counters.

Focused group-version comparison ran before/group/group/before, with the same
100 samples / 3s warm-up / 5s measurement. Before: 1.0183 / 1.0181 us;
group: 1.0011 / 0.98800 us. Complete workload comparison then used
before/group/group/before, 50 samples / 2s warm-up / 3s measurement:

| Case | Before first / last | Group first / second |
| --- | --- | --- |
| Increment, 3 levels | 66.877 / 65.258 ns | 66.025 / 64.954 ns |
| Snapshot, 3 levels | 97.322 / 97.499 ns | 100.49 / 96.860 ns |
| Rate-limit error | 70.476 / 72.327 ns | 69.943 / 70.497 ns |
| Increment, 128 levels | 1.0185 / 1.0214 us | 0.99095 / 1.0057 us |
| Snapshot, 10,000 levels | 89.403 / 90.476 us | 85.236 / 91.041 us |

These are central iteration-time estimates, **not p99/p99.9**. Group 128-level
measurements do not reproduce the preceding sustained slowdown; small-message
differences are within visible local variation. Large snapshot results are mixed,
with a noisy second candidate run, so no stable snapshot speedup is claimed.
**Decision: adopt the group-version single Vec for verified reservation reduction,
with local timing evidence and reference-runner acceptance still incomplete.**
No production budget or tolerance is invented or relaxed.

Before/after allocation probes use 16 total slots: setup allocations 2 → 1,
requested level bytes 1024 → 512, cleanup frees 2 → 1 with matching bytes.
All three warmed small-message paths retain zero allocations/reallocations/frees.
At a 10,000-level bound, nominal reserved level bytes are 640,000 → 320,000,
excluding metadata and allocator overhead. Level remains 32 bytes. No additional
level copies or shifts were introduced; actual cache traffic, RSS, peak working
set and multi-decoder effects remain unmeasured. Probe timing/counts are diagnostic,
with atomics and clock overhead, not independent offered-load capacity evidence.

Commands (run binaries one at a time in the recorded order):

```sh
CARGO_TARGET_DIR=/tmp/strait-conversion-before-target cargo bench --manifest-path /tmp/strait-buffer-retry/before/Cargo.toml --locked --offline --bench spot_sbe --no-run
CARGO_TARGET_DIR=/tmp/strait-conversion-before-target cargo rustc --manifest-path /tmp/strait-buffer-retry/before/Cargo.toml --locked --offline --release --lib -- --emit=asm
/tmp/strait-buffer-retry/evidence/before-bench --bench --noplot depth_128_levels --sample-size 100 --warm-up-time 3 --measurement-time 5
/tmp/strait-buffer-retry/evidence/group-bench --bench --noplot depth_128_levels --sample-size 100 --warm-up-time 3 --measurement-time 5
/tmp/strait-buffer-retry/evidence/before-bench --bench --noplot --sample-size 50 --warm-up-time 2 --measurement-time 3
/tmp/strait-buffer-retry/evidence/group-bench --bench --noplot --sample-size 50 --warm-up-time 2 --measurement-time 3
cargo run --locked --offline --release --example spot_profile -- 1
```

Build the group manifest with the same build/assembly commands, saving its executable
and assembly before rebuilding another variant. Full commands, logs, fixtures/source
hashes, binary hashes and assembly are local under
`/tmp/strait-buffer-retry/evidence/`; source copies remain beside them. Final working
source differs from the measured group version only in invariant comments. Artifacts
are temporary/local and have not been uploaded. No global tools or machine settings
were changed.

Contract/security review traced the private split invariant: clear resets length
and split; successful bid decode sets split to length; asks only append; all group
and enclosing-message failures clear storage. Borrowing prevents reuse while views
are outstanding. Total/byte limits precede level retention, exact arithmetic and
snapshot ordering checks remain, and no definite in-scope invariant violation was
found. Retained independent wire tests cover uneven/full-side splits, empty and
zero-capacity inputs, overflow and post-failure reuse. Book reference replay is not
applicable to this decoder-only storage change; no L2 engine exists yet.

Final formatting, warnings-denied Clippy, all-target/all-feature tests (one numeric
unit test, 14 protocol tests and benchmark smoke), doctests, warnings-denied rustdoc
and 100,128 fixed-seed fuzz-smoke cases passed. CPU-time/headroom, executed instruction/
cache/branch counters, syscall/kernel attribution, generated-code/recorded-payload
differentials, coverage-guided campaigns and integrated/reference-runner p99/p99.9
remain evidence gaps. No new CPU sample was obtained after the previously recorded
sandbox attach failure. USD-M has no implemented numeric decoder to compare.
This change does not complete Phase 1 or establish production readiness.

## Snapshot Decode Responsibility Refactor (2026-10-09)

Moved snapshot group traversal into `snapshot.rs`, alongside its message framing;
`LevelBuffer` retains conversion, storage, side splitting and snapshot-level checks.
The `inline(never)` group boundary, numeric rules, layout/count preflight and
failure cleanup are preserved. Increment traversal is unchanged. Baseline was the
existing dirty working tree, not HEAD; only `sbe.rs` and `snapshot.rs` changed for
this refactor. Before sources and executable, source/binary hashes and raw logs
are retained locally under `/tmp/strait-layout-*`.

Local Darwin x86_64, rustc 1.97.1 / LLVM 22.1.6, default bench profile,
Criterion 0.7, existing synthetic fixtures and limits in `benches/spot_sbe.rs`.
CPU model/memory queries were denied by the sandbox; thermal state, power and
background load were uncontrolled. Runs were sequential before/after/after/before,
30 samples, 1s warm-up, 2s collection, without overlapping build/profile work.
Central iteration estimates (not receipt-to-view percentiles):

| Case | Before 1 / 2 | After 1 / 2 |
| --- | --- | --- |
| Increment, 3 levels | 72.475 / 63.960 ns | 71.217 / 68.504 ns |
| Snapshot, 3 levels | 129.24 / 112.23 ns | 102.95 / 115.32 ns |
| Rate-limit error | 89.907 / 73.746 ns | 71.025 / 65.383 ns |
| Increment, 128 levels | 1.9418 / 0.84418 us | 1.0293 / 1.0901 us |
| Snapshot, 10,000 levels | 110.71 / 86.477 us | 93.517 / 105.63 us |

Timing evidence is inconclusive: direction reverses between pairs, including the
unchanged increment path. No stable speedup or regression is established and no
numerical budget is changed. The refactor is retained for consistent responsibilities;
stable-reference-runner performance acceptance remains open.

Commands (first run before editing, then candidate; repeat saved executables in
reverse order with the same flags):

```sh
cargo bench --locked --offline --bench spot_sbe -- --noplot --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline layout-before
cargo bench --locked --offline --bench spot_sbe -- --noplot --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline layout-after
cargo run --locked --offline --release --example spot_profile -- 1
cargo run --locked --offline --release --example spot_fuzz_smoke
```

Candidate allocation probe: one setup allocation / 512 requested bytes for 16
levels, zero warmed allocations/reallocations/frees on all three small-message
paths, one cleanup free / 512 bytes; Level remains 32 bytes. No new allocation,
string work, payload copy, direct I/O or async boundary was introduced. No fresh
baseline allocation probe, CPU profile/time/headroom, hardware counters, cache/copy
traffic, syscall/kernel attribution or integrated tails were collected. USD-M has
no implemented numeric decoder to compare. Existing generated-code differential,
coverage-guided fuzzing and reference-runner gaps remain.

Formatting, warnings-denied Clippy, all-target/all-feature tests (one numeric unit
test, 16 protocol tests and benchmark smoke), doctests, warnings-denied rustdoc,
and 100,128 fixed-seed fuzz-smoke cases passed. Contract/security review traced
preflight, side splitting, borrowed views and failure clearing; no definite
in-scope violation was found. No book storage algorithm changed, so reference-book
storage tests do not apply. This evidence does not establish production readiness.

### Snapshot Rules Follow-up (2026-10-09)

Completed the responsibility split: `LevelBuffer::push_level` now serves both
message types; zero-quantity rejection and strict snapshot ordering live in
`snapshot.rs`. Numeric conversion still precedes zero rejection, preserving error
precedence and outer failure clearing. The group `inline(never)` boundary remains.
Baseline is the preceding dirty-tree refactor, saved as sources and executable at
`/tmp/strait-responsibility-before-*`; no unrelated edits were removed.

Same local toolchain/profile/fixtures as above. Sequential before/after/after/before
runs used 30 samples, 1s warm-up and 2s measurement. Central iteration estimates:

| Case | Before 1 / 2 | After 1 / 2 |
| --- | --- | --- |
| Increment, 3 levels | 96.067 / 80.283 ns | 116.30 / 124.03 ns |
| Snapshot, 3 levels | 163.80 / 163.75 ns | 294.35 / 266.48 ns |
| Rate-limit error | 83.145 / 71.763 ns | 175.73 / 133.35 ns |
| Increment, 128 levels | 1.0105 / 0.93639 us | 1.4951 / 1.3842 us |
| Snapshot, 10,000 levels | 120.27 / 93.032 us | 113.26 / 89.189 us |

A focused before/after snapshot pair with identical flags and `ws_snapshot` filter
measured 102.65 → 113.13 ns for 3 levels and 94.716 → 83.939 us for 10,000 levels.
Small snapshots show a local slowdown signal; broad-run movement in unchanged
paths also shows substantial environmental variation. The cause is unresolved:
this is a responsibility cleanup, not an accepted performance optimization.
Stable-runner confirmation and CPU attribution remain required before performance
acceptance. No numerical contract was relaxed and no production readiness follows.

Exact command for each saved binary (sequential, in the order above):

```sh
<benchmark-executable> --bench --noplot --sample-size 30 --warm-up-time 1 --measurement-time 2
<benchmark-executable> --bench --noplot ws_snapshot --sample-size 30 --warm-up-time 1 --measurement-time 2
```

Raw logs are `/tmp/strait-responsibility-{before,after}-{1,2}.log` and
`/tmp/strait-responsibility-focused-{before,after}.log`. Candidate allocation probe
(`/tmp/strait-responsibility-profile.log`) matches the preceding probe: setup
1 allocation / 512 bytes, warmed small paths zero allocations/reallocations/frees,
cleanup 1 free / 512 bytes. No new strings, copies, I/O, tasks, dependencies or
storage algorithm were introduced. CPU profiles/counters/headroom, syscall attribution,
reference hardware and integrated tails remain gaps; USD-M decoding is absent.
Formatting, Clippy, all-target/all-feature tests, doctests, warnings-denied rustdoc,
and 100,128 fixed-seed fuzz-smoke cases passed. Contract/security review found no
new definite invariant violation; input bounds and exact numeric/error behavior
remain intact. Performance evidence remains incomplete with the local slowdown
explicitly unresolved.
