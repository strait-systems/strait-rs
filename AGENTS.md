# Strait Repository Instructions

## Engineering Target

Build high-frequency production market data infrastructure with correctness and
predictable low tail latency. Functional completeness alone is insufficient.
The current repository is a skeleton; never claim production readiness or measured
performance without evidence. Preserve the initial Binance Spot / USD-M scope.

## Contract Routing

Before code changes, read `docs/engineering.md` and relevant parts of
`docs/architecture.md`. For hot-path types, transport, parsing, sequencing, books,
queues, publication, runtime, or dependencies, also read `docs/latency.md` and the
applicable measurement/acceptance sections of `docs/testing.md`. `docs/roadmap.md`
defines phase deliverables. Docs are the canonical detailed contract: reference
rather than duplicate them in skills or invent stronger numerical guarantees.

## Mandatory Invariants

- One logical writer per instrument owns book mutation, sequence/sync/cycle state,
  and ordered publication. Readers/snapshot workers send inputs; consumers receive
  immutable views. No shared locked book or multi-writer replacement by default.
- Keep validated mutation and publication ordered in the writer; no consumer
  callbacks, snapshot waits, or blocking operations in the steady-state update.
- Exact numeric types and explicit units/scales; no rounding, wrapping, silent
  truncation, or missing-update shortcuts for performance.
- Every async boundary has ordering, ownership, cancellation, cycle isolation,
  count/byte limits, and failure delivery independent of a full data queue.
- `Live` is sequence validity; real-time usability also requires `Fresh` and an
  age check at use time. Liveness/freshness detection cannot require new depth data.
- Evidence decides type/crate/pattern costs. Do not label `clone`, `Arc`, a mutex,
  dynamic dispatch, or a dependency as a measured bottleneck without context/data.
  A shared locked mutable book violates ownership regardless of measured speed.

- Apply the mandatory string/allocation/CPU/syscall contract in `docs/engineering.md`.
  Resolve symbols/labels during setup; avoid repeated string work and unreviewed
  allocation in updates. Include reclamation, transitive crate/runtime kernel work,
  CPU headroom, and explicit measurement gaps in resource evidence.

- Apply `docs/security.md` for untrusted inputs, task supervision, bounded retries,
  supply-chain policy, and CI trust boundaries.

- Connection changes also apply `docs/connections.md`: lifecycle/generations,
  backoff/jitter, product-specific sequences, stale detection, global admission,
  multi-connection isolation, and aggregate load/failure evidence.

## Required Review and Validation

For code changes, apply `skills/strait-contract-review/SKILL.md` before
completion. For a first hot-path implementation, performance-sensitive change,
benchmark work, or dependency/configuration update affecting the measured path,
also apply `skills/strait-performance-check/SKILL.md`. If skill discovery
has not refreshed, read these files directly. For transport/parser, endpoint/configuration,
dependency, unsafe, or CI changes,
also apply `skills/strait-security-review/SKILL.md`.
Documentation-only edits need link and consistency checks, not unrelated performance runs.

Run applicable formatting, Clippy, correctness and doc checks from README. CI must
build benchmark targets as they land. The first implemented hot-path component
includes a runnable benchmark, fixture/seed, commands, and initial baseline/profile.
Relevant changes include before/after evidence; retain user changes when choosing
baselines. Apply independent reference-book tests to storage optimizations.

Resolve definite violations within the authorized implementation scope and repeat
only relevant checks. A review-only request reports findings without editing. Missing
harnesses, unavailable reference hardware, or failed runs are evidence gaps: report
them, do not fabricate a pass or silently relax a budget. Use temporary/local result
artifacts and never upload or install tools globally without applicable authorization.

Final reports identify changes, validation, definite contract violations, unresolved
measurement gaps, and readiness limits. Complex optimization and ownership changes
must meet the documented review/evidence gates; surface proposed departures explicitly
instead of silently rewriting the contract to fit an implementation.

## Shared Skill Sources

`skills/<name>/SKILL.md` is the canonical Agent Skills instruction. `.agents/skills/`
provides Codex discovery and optional `agents/openai.yaml` UI metadata; `.claude/skills/`
provides Claude Code discovery. Both use relative symlinks to shared sources.
`CLAUDE.md` imports this file; do not maintain a separate Claude ruleset.
See `docs/contributing/agent-skills.md` for invocation, layout, and symlink compatibility.
