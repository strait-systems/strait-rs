# Security and Resilience Contract

Public market data is untrusted input. Production safety includes preventing false
books, silent stale output, CPU/memory exhaustion, and compromised builds. Apply this
contract alongside [production engineering](engineering.md) and [testing](testing.md).
The skeleton has no implemented parser or transport to certify yet.

## Input and Transport

- Validate required fields, symbol/product identity, numeric scale/range, sequence
  identifiers, and message kind before applying state. Unsupported precision and
  overflow fail explicitly; malformed required input invalidates the affected cycle.
- Enforce message/frame/snapshot and level-count limits before unbounded retention.
  Account for fragmented frames, decompressed size if enabled, nesting, oversized
  numeric strings, parse work, and repeated malformed input. Never truncate updates.
- External input and network errors must not reach unchecked indexing, unchecked
  arithmetic, panic/unwrap paths, or partial mutation that leaves a published live
  book. Defined process-fatal internal invariant failures need explicit supervision
  and must not leave consumers believing a dead writer is healthy.
- Preserve TLS certificate/hostname verification. Validate configured endpoints and
  allowed schemes; redirects, proxy behavior, and credentials must have explicit
  policies. Do not silently downgrade security to reduce latency.
- Bound diagnostics and redact secrets. Arbitrary input must not become commands,
  file paths, metric labels, or a reason to bypass validation.

## Supervision, Cancellation, and Recovery

Define ownership and supervision for readers, snapshot workers, writers, and timers.
Unexpected exit must reach a supervisor and invalidate affected output independently
of the data queue. Cancellation and shutdown have a defined order: publish unusable
state, stop producers, cancel/join remaining work within bounded time, and release
resources. Reject late results by cycle identity; no abandoned task may keep writing.

Bound retry count/rate and concurrent recovery; document backoff/jitter, cooldown,
request deadlines, and venue rate-limit handling. A retryable error, malformed
protocol input, and an unsupported configuration are distinct outcomes. Reconnect
storms must not starve control tasks, exhaust sockets, or disrupt unrelated live books.
Avoid infinite immediate retry loops. Test cancellation during snapshot fetch,
synchronization, queued input, publication, and reconnect delays.

## Supply Chain and Repository Controls

`deny.toml` defines dependency advisory, license, source, and wildcard checks.
`.github/workflows/security.yml` runs them on PRs, pushes, a weekly schedule, and
manual dispatch. Known vulnerability/source/license errors block that check;
duplicate versions are warnings requiring review. Unmaintained dependencies are
triaged, not silently ignored. Exceptions must be specific, justified, owned, and
reviewed with an expiry/revisit date; do not add blanket advisory ignores to pass CI.

Review build scripts, procedural macros, git dependencies, enabled features, and
transitive changes when introducing or updating dependencies. Passing advisory checks
only establishes no detected issue in their scope, not that a dependency is safe.
Use pinned action commits; update pins through reviewed changes. CI credentials must
have minimum permissions and not be exposed to untrusted builds or fork code.

AI review checks out baseline policies and inspects candidate code through Git objects;
it does not load candidate skills or execute candidate code. Policy changes are
reviewed against the old rules. An initial bootstrap must merge policies into the
trusted base before enabling AI review. Manual runs trust the selected workflow ref.
Workflow changes can still alter the job itself: configure branch/ruleset protection
and mandatory human review for `AGENTS.md`, `CLAUDE.md`, `skills/`, `.agents/`, `.claude/`, `.github/`, security policies,
and contract docs. These remote protections require repository-admin setup and are
not created by local files. Do not treat same-repository authorship as a sandbox.

Follow [branch protection setup](maintainers/branch-protection.md) to require PR updates for
`dev` and `main` and enforce same-repository `dev` as the only PR source for `main`.
The metadata-only source check uses trusted base workflow logic without checkout,
secrets, or token permissions; its required-check enforcement is configured remotely.

## Verification Gates

- First parser/numeric-decoder implementation includes fuzz targets for arbitrary
  bytes, malformed numbers, boundaries, and oversized-input handling. First sync
  implementation includes generated event sequences with independently specified
  acceptance/invalidation behavior. Retain seeds/corpus and minimized regressions.
- Fuzz smoke runs accompany affected PRs once targets exist; longer campaigns are
  scheduled. No placeholder fuzz target counts as coverage. Bound work and assert
  no input-driven panic, unsafe access, silent corruption, or unbounded retention.
- Unsafe/memory-reuse changes document invariants and use applicable dynamic checks.
  Custom atomic publication/reclamation changes need interleaving tests or model
  checking suitable to their memory model; single writer does not prove reader safety.
- Fault and soak tests verify failure detection, shutdown, recovery isolation, task/
  connection counts, memory plateau, and latency drift across repeated cycles.
  Record run duration and load; thresholds follow reference baselines and budgets.

## Review Workflow

`$strait-security-review` applies to network/parser changes, dependencies, endpoints,
configuration, unsafe code, and CI/credential handling. It distinguishes demonstrated
risks, missing evidence, and optional hardening. Relevant changes also retain the
contract/performance reviews; security never justifies silent data drops or stale
output, and speed never justifies removing validation.

AI review findings are advisory evidence. Its CI disposition gate rejects required
changes, missing required evidence, inconclusive or malformed reports; a successful
AI review is not a proof of safety. Deterministic tests and dependency checks remain
separate. Current fuzz/fault/soak harnesses are pending implementation, not passed.

## Connection Failures and Resource Isolation

Apply [connection lifecycle and capacity](connections.md) for blackholes, heartbeat
failure, retry storms, late generations, global admission/rate limits, and noisy
neighbors. A shared socket's blast radius and failure-to-invalidation delay must be
explicit; a successful reconnect alone cannot mark dependent books usable.
