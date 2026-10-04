---
name: strait-contract-review
description: Review Strait Rust code changes against its Day 0 production and single-writer contracts, including types, update flow, dependencies, correctness, freshness, and resource limits. Use during implementation or an explicit code review.
---

# Strait Contract Review

Review applicable code changes against the repository's actual contract, with
single-writer ownership as a mandatory invariant. Do not turn review suggestions
into unsupported latency claims.

## Scope and Sources

Locate the Strait repository root and read `AGENTS.md`, `docs/engineering.md`, and
relevant architecture sections. Read `docs/latency.md` for type/dependency/timing
changes and `docs/testing.md` for acceptance evidence. Use these canonical files;
do not use a separate copied checklist. If absent, report the missing contract.

Establish the requested diff/base, working-tree status, and affected callers/tests.
Include untracked files when reviewing a working-tree implementation. Do not assume
that `HEAD` or the entire dirty tree is the intended change. Inspect enough surrounding
code to establish actual ownership and call paths, without sweeping unrelated work.

## Review

Trace an update from receipt through decoding, synchronization, book application,
and consumer publication. Identify the sole mutation authority per instrument,
including snapshots, reconnects, timers, cancellation, and late results. Check the
single-writer contract in `docs/engineering.md`; a channel or atomic does not alone
prove ownership, ordering, or freshness.

Apply the relevant documented requirements for exact typed numerics and scale/range,
wire/domain conversion, lifetimes and buffer reuse, compact identifiers, allocation
behavior, async handoffs, pattern costs, configured dependencies, count/byte bounds,
retained-depth scope, and failure/recovery. Verify tests cover independently specified
protocol outcomes and reference-book state where applicable.

Apply the string, allocation/reclamation, CPU, and syscall sections of
`docs/engineering.md`: trace symbol/label lifetime, numeric conversion, buffer growth,
free/drop behavior, bounded work, and direct/transitive I/O or wakeups. Check scoped
budgets and required resource evidence; do not infer syscall counts from API names
or classify every owned string/allocation as a violation.

Check validity and freshness independently, including no-arrival watchdog behavior,
consumer age checks, overflow control paths, and recovery starvation. Distinguish
an established invariant violation from a suspected resource/performance cost.
Search tools may identify candidates; keyword matches are not findings by themselves.

Apply lifecycle requirements in `docs/security.md`: task supervision, writer exit,
cancellation/join order, bounded retries/backoff, venue limits, late-cycle inputs,
and fault/soak evidence. Coordinate with security review for input/credential risks;
do not duplicate its findings.

Trace connection states, serialized outbound ownership, generations, deadlines,
backoff/jitter and reset rules, per-instrument sequence/freshness, socket blast radius,
and bounded registry/admission against `docs/connections.md`. Check deterministic
clock/RNG and local-server tests; socket-connected is not book-live-and-fresh.

## Findings and Resolution

Report actionable findings in severity order. Each finding includes file/line,
canonical contract section, concrete trigger and impact, required change, and a
validation method. Separate:

- **Contract violation:** demonstrated break of a mandatory invariant.
- **Evidence gap:** a required measurement/test is absent or could not run.
- **Measured regression:** linked comparable results show a budget/regression failure.
- **Optional experiment:** possible benefit needing a benchmark; not a merge blocker
  merely because a construct appears in the code.

State coverage and remaining uncertainty when no findings are demonstrated. Do not
claim production readiness from static review. Review-only requests report changes
needed; authorized implementation requests fix definite in-scope violations and run
relevant checks. Do not silently redesign interfaces, change budgets, or rewrite
contract requirements to make a review pass. Missing hardware/harnesses remain gaps.
