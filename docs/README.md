# Documentation Guide

Read [design and scope](design.md) for project direction, then [architecture](architecture.md)
for modules and data flow. Before implementing a component, read its applicable
contracts below. The repository remains an early skeleton; requirements describe
the acceptance target rather than implemented or measured capabilities.

## Design and Engineering

| Document | Owns |
| --- | --- |
| [Design and scope](design.md) | Concise project boundaries and design decisions |
| [Architecture](architecture.md) | Module responsibilities, data flow, and initial interfaces |
| [Production engineering](engineering.md) | Single-writer ownership, implementation rules, string/allocation/CPU/syscall requirements |
| [Latency requirements](latency.md) | Day 0 operating contract, type/crate evaluation, timing boundaries, workloads, budgets, and regression policy |
| [Testing and performance](testing.md) | Executable correctness evidence, benchmark/profile workflow, fixtures, and implementation gates |
| [Connections](connections.md) | Connection lifecycle, backoff/jitter, sequence continuity, stale detection, and aggregate capacity |
| [Security and resilience](security.md) | Untrusted input, task supervision, supply chain, and security verification |
| [Roadmap](roadmap.md) | Implementation phases and milestone acceptance |

Keep detailed requirements in their owning document. Other documents summarize and
link to them; skill instructions apply these contracts rather than maintain copies.
Architecture can specify an initial interface while the engineering contract states
the invariant that every implementation of it must preserve.

## Contributor Tooling

[Shared agent skills](contributing/agent-skills.md) explains the portable skill
sources, Codex/Claude discovery, invocation, and layout validation. Agent routing
is defined in [AGENTS.md](../AGENTS.md).

## Maintainer Setup

- [AI review](maintainers/ai-review.md): credentials, opt-in switch, PR eligibility,
  report publication, and review disposition.
- [Branch protection](maintainers/branch-protection.md): remote rulesets, required
  checks, and the feature → `dev` → `main` PR flow.

These setup guides are for repository maintainers. Remote GitHub settings are not
activated by documentation or local workflow validation alone.
