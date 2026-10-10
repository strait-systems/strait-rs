# Spot generated-code migrations

The initial Diff Depth migration below is historical. For the current all-generated
implementation and removed handwritten decoder, see [the subsequent cleanup](#remaining-handwritten-decoder-removal).

## Initial Diff Depth migration (historical)

On 2026-10-09 the selected Diff Depth decoder was switched to Rust code emitted by
official SbeTool 1.35.6 from Binance stream schema 1:0. The local `spot_stream`
dependency has no dependencies/build scripts/unsafe code. Its schema, generator
hashes, license and reproduction instructions are retained in
[the generated crate](../generated/binance-stream/README.md). Normal Rust builds
need no Java or network access. The separate API snapshot reader is unchanged.

## Correctness and review

The generated codec's indexing is protected by checked framing preflight: exact
schema/version/template/root length, both 16-byte group layouts, combined count,
complete entry bytes, bounded nonempty symbol, and no trailing bytes. Preflight
runs before generated accessors; it does not decode or copy entries. Domain
conversion still rejects sign/range/precision errors and validates empty-group
exponents. Buffer cleanup on every error and borrowed-view ownership are preserved.
There are no new tasks, queues, book writers or publication paths.

A test-only retained pre-migration reader independently compares acceptance and
all successful metadata/levels for two synthetic depth fixtures, every truncation
and every one-byte mutation through 256 values: **564,376 comparisons**. Error
precedence for multiply-invalid inputs can differ because framing is checked first.
Both readers must reject those messages; the exact first error is not compared.

Validation passed locally:

- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo test --locked --all-targets --all-features` (2 unit and 14 protocol tests,
  plus benchmark smoke execution and example targets)
- `cargo test --locked --doc --all-features` (zero doctests)
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --all-features`
- `cargo bench --locked --no-run --all-targets --all-features`
- `cargo run --locked --release --example spot_fuzz_smoke`
  (100,128 cases, seed `0x535452414954`, no input-driven panic)
- Pinned generator regeneration with `--check` reproduced all retained `.rs` files
  and the manifest after formatting.

The contract, performance and security skill reviews found no demonstrated
in-scope invariant violation. The only lockfile change is the dependency-free
local crate. Runtime dependency sources/features and CI trust boundaries are
unchanged. Generated code's Clippy allowances are generator output; Strait's own
Clippy warnings remain errors. `cargo-deny` is unavailable locally, so the
repository's independent dependency CI remains required. This review is not a
security certification.

## Local measurements

The baseline preserves the entire starting dirty-tree implementation, including
user changes; it is not HEAD. Before changing the decoder, its source was retained;
a comparison copy reconstructed at `/tmp/strait-sbe-tools/before-source` restores
those files and removes only the new local dependency. Raw logs, source hashes,
probe results and this baseline are in `/tmp/strait-sbe-tools/` (temporary artifacts,
not permanent release evidence). Criterion baselines are under `target/criterion`.

Environment: macOS 15.7.7 (24G720), x86_64-apple-darwin, Rust 1.97.1
(`8bab26f4f`), LLVM 22.1.6, default release/bench profiles. CPU model retrieval was
denied by the sandbox. Power/frequency, thermal state, affinity and background
load were not controlled; this is not a reference runner. No runtime/network or
integrated pipeline is benchmarked.

Each run uses the unchanged `benches/spot_sbe.rs`, one decoder/instrument, borrowed
98-byte/3-level and 2098-byte/128-level synthetic fixtures, 30 samples, 1-second
warmup and 2-second measurement. Runs alternate before/after twice. Values below
are Criterion point estimates of iteration time, **not p99/p99.9**.

| Workload | Before 1 | Generated 1 | Before 2 | Generated 2 |
| --- | --- | --- | --- | --- |
| 3 levels | 69.399 ns | 64.506 ns | 91.131 ns | 65.795 ns |
| 128 levels | 1.0377 us | 1.0676 us | 1.0397 us | 0.95129 us |

The first 128-level comparison showed no significant change; the second showed
an improvement. The second 3-level baseline was noisy (confidence interval
66.316–127.77 ns). These results do not establish a stable speed advantage or a
production performance budget. No repeatable decoder regression was established;
adoption is for schema-derived decoding and reduced manual field maintenance.

The existing allocation probe (`spot_profile -- 1`) measured zero warmed
allocations/reallocations/frees/allocated bytes for depth in both implementations.
Setup retained one 512-byte allocation for 16 reserved levels; cleanup freed it
once on the caller. No pool, payload/string ownership, or additional copy was
introduced. Instrumented operation counts are diagnostic and are not acceptance
throughput; per-level conversion remains O(levels), with constant framing work.

Commands (from repository root):

```sh
cargo bench --locked --bench spot_sbe -- depth --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline generated-before
cargo bench --locked --bench spot_sbe -- depth --sample-size 30 --warm-up-time 1 --measurement-time 2 --baseline generated-before
CARGO_TARGET_DIR="$PWD/target" cargo bench --manifest-path /tmp/strait-sbe-tools/before-source/Cargo.toml --locked --offline --bench spot_sbe -- depth --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline generated-before-repeat
cargo bench --locked --bench spot_sbe -- depth --sample-size 30 --warm-up-time 1 --measurement-time 2 --baseline generated-before-repeat
cargo run --locked --release --example spot_profile -- 1
```

Remaining evidence gaps: real captured payloads, generated API snapshot comparison,
coverage-guided fuzz campaigns, Rust 1.88/Linux/reference-runner checks, CPU counters,
CPU time/headroom, syscall/kernel measurements, multi-decoder memory effects and
integrated sustained/burst/tail-latency/failure tests. Generated source inspection
shows no direct I/O in the used codec; that is not a measured syscall claim.
USD-M decoding is unimplemented and unaffected. No production-readiness claim is made.

## Remaining handwritten decoder removal

The subsequent 2026-10-09 cleanup removes the remaining handwritten field reader,
level-group parser, byte-offset rate-limit decoder, and test-only old depth codec.
Both stream and API messages now use official generated Rust accessors. API schema
3:5 is retained and pinned, with only templates 50/100/200 selected for generation;
all selected official fields/types/layouts are preserved. See
[API provenance and regeneration](../generated/binance-api/README.md).

`LayoutCursor` is a bounds/layout guard: even its unsigned framing reads and header
access delegate to generated code, after proving the span is present. Wrapper and
depth layouts are validated before their generated accessors. Errors validate the
fixed root and the uint16 message-length prefix, read that length once using the
generated accessor, then check the text span and zero optional-data length before
returning a response. Exact domain conversion, enums, identity, ordering, limits,
borrowed ownership and cleanup remain mandatory. No generator/source allowance
weakens Strait's input-validation contract. The private rate-limit iterator keeps
its generated parent for its whole lifetime; its internal missing-parent branch
is unreachable from validated immutable input.

### Validation and review

All final README checks passed: formatting, Clippy with warnings denied, all-target
and all-feature tests, doctests, documentation with warnings denied, and benchmark
builds. There are 16 protocol tests plus the existing numeric unit test. New tests
exercise every byte value for API bool/rate enums, repeatable immutable rate views,
optional-time null/negative/zero/max boundaries, bad text and unsupported nested
data. The final bounded fuzz smoke passed 100,128 cases.

The identical [digest helper](../examples/spot_codec_digest.rs) ran in optimized
builds against the retained starting source and final implementation. Output matched
for **619,636 cases**: four fixtures, all their truncations and one-byte mutations,
valid reuse, and the 10,000-level snapshot. It compares acceptance and 64-bit hashes
of successful metadata/levels/error/rate fields, not first-error precedence. Hash
comparison is supplementary evidence; independently specified fixture/enum/time
assertions remain the exact-value checks. The old decoder is retained only under
`/tmp/strait-sbe-cleanup/before-source`, not in project code. Both generated crates
also reproduced under the pinned regeneration script's `--check` mode.

Contract/security review found no demonstrated in-scope ownership, numeric, bounds,
input-driven panic or cleanup violation. The API crate has no runtime dependencies,
build scripts or unsafe code. The lockfile adds only that local crate relative to
the starting implementation. No network session, queue, retry, writer or publication
behavior changed. Dependency advisory/license CI remains necessary: `cargo-deny`
is unavailable locally. Rust 1.88 and Linux/reference-runner checks were not run.

### Performance work and limits

Starting source preserves the user's dirty-tree implementation, including the
previous generated stream migration; HEAD is not the baseline. Raw logs, baseline
copy, decoded digests, source manifest, emitted assembly and allocation probes are
in `/tmp/strait-sbe-cleanup/`. These are temporary artifacts, not durable release
acceptance evidence. Environment and workloads match the initial migration above:
macOS x86_64, Rust 1.97.1, default optimized profiles, one decoder, unchanged fixtures
and Criterion harness, 30 samples, 1-second warmup, 2-second measurement. CPU model
retrieval is denied; frequency/thermal/background load is uncontrolled.

The initial straightforward generated API adapter showed slower error/small
messages and some slower large snapshots. Small validated text/root helpers were
inlined; error-message framing was revised to avoid rereading its length. Whole-group
snapshot normalization was then factored out of the two generated-side loops.
Emitted assembly contains one merged `append_snapshot_group` body called twice,
rather than duplicated conversion loops. This is a normalizer over generated values,
not a replacement wire parser. The targeted group experiment measured 69.080 us for
10,000 levels versus the repeated baseline's 81.823 us; a small snapshot measured
107.36 ns versus 92.504 ns. Final repeated results are recorded below. Improvements
in one workload do not cancel regressions in another.

The final allocation probe retained one 512-byte setup allocation for 16 levels,
zero warmed allocation/reallocation/free calls or bytes on the depth/snapshot/error
fixtures, and one caller-side cleanup free. Input/symbol/error/rate views stay borrowed;
no payload copy, owned string, pool, deferred reclamation or new async work is added.
Large-snapshot allocation counts, CPU time/headroom and integrated resource costs
were not measured by that small-fixture probe. Sampling the owned probe with
`/usr/bin/sample` failed due to process-inspection permissions; the failure log is
retained. Assembly is explanatory evidence, not a CPU sampling result.

The remaining production evidence gaps listed above still apply, including real
payloads, coverage-guided fuzzing, hardware/reference runner, syscall/kernel and CPU
counters/headroom, integrated bursts/tail latency, fault/soak behavior and publication.
No numerical budget is relaxed or production-readiness claim made. Scope remains
Binance Spot/USD-M, with USD-M decoding still unimplemented.

Reproduction (from repository root, using the retained baseline directory):

```sh
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path /tmp/strait-sbe-cleanup/before-source/Cargo.toml --locked --offline --release --example spot_codec_digest > /tmp/strait-sbe-cleanup/before-digests.txt
cargo run --locked --release --example spot_codec_digest > /tmp/strait-sbe-cleanup/final-digests.txt
cmp /tmp/strait-sbe-cleanup/before-digests.txt /tmp/strait-sbe-cleanup/final-digests.txt
CARGO_TARGET_DIR="$PWD/target" cargo bench --manifest-path /tmp/strait-sbe-cleanup/before-source/Cargo.toml --locked --offline --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --save-baseline cleanup-before-repeat
cargo bench --locked --bench spot_sbe -- --sample-size 30 --warm-up-time 1 --measurement-time 2 --baseline cleanup-before-repeat
cargo bench --locked --bench spot_sbe -- ws_snapshot --sample-size 30 --warm-up-time 1 --measurement-time 2 --baseline cleanup-before-repeat
cargo run --locked --release --example spot_profile -- 1
```

The helper source was copied unchanged into the baseline for that comparison.

Final local Criterion point estimates (iteration time, not receipt-to-view tails):

| Workload | Starting baseline 1 | Starting baseline 2 | Final full run | Final snapshot repeat |
| --- | --- | --- | --- | --- |
| Depth, 3 levels | 65.749 ns | 63.721 ns | 63.782 ns | — |
| Snapshot, 3 levels | 99.620 ns | 92.504 ns | 137.01 ns | 118.80 ns |
| Rate-limit error | 70.851 ns | 69.102 ns | 82.781 ns | — |
| Depth, 128 levels | 1.0036 us | 1.0438 us | 0.94457 us | — |
| Snapshot, 10,000 levels | 84.466 us | 81.823 us | 75.483 us | 68.812 us |

**Performance status: mixed, with measured small-snapshot/error regressions; not
accepted as a production performance result.** Large-snapshot performance recovered
with the factored group adapter, and depth did not show a stable slowdown in the
final runs. Small snapshots/error responses remain slower than the retained reader.
The full-run small-snapshot confidence interval was 129.18–147.13 ns; its repeat
was noisier at 106.23–139.98 ns. Timing variation prevents an exact speed guarantee,
but does not erase the observed regressions. The requested removal is implemented;
these costs are a visible maintenance/safety tradeoff requiring review and further
reference-runner measurement before performance acceptance. No relative tolerance
or latency budget has been invented to classify the regression as a pass.
