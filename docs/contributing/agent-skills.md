# Shared Agent Skills

Core skills use the open [Agent Skills format](https://agentskills.io/specification):
`SKILL.md` with `name` and `description` YAML frontmatter followed by Markdown
instructions. Runtime discovery paths and optional UI metadata are separate.

## Layout

```text
AGENTS.md                          # Shared repository instructions
CLAUDE.md                          # Imports AGENTS.md for Claude Code
skills/
  strait-contract-review/SKILL.md   # Canonical portable instructions
  strait-performance-check/SKILL.md
  strait-security-review/SKILL.md
.agents/skills/<name>/
  SKILL.md                         # Relative symlink to canonical SKILL.md
  agents/openai.yaml               # Optional Codex UI/invocation metadata
.claude/skills/<name>               # Relative symlink to canonical skill directory
```

Edit rules only in `skills/` and their canonical `docs/` references. Codex metadata
contains display names, a sample invocation, and Codex invocation policy; it does
not define engineering requirements or configure Claude. The shared SKILL.md files
contain no vendor-specific tool restrictions, substitutions, or invocation syntax.

[Claude Code](https://code.claude.com/docs/en/skills) supports project skill-directory
symlinks. `CLAUDE.md` uses the documented
[AGENTS.md import](https://code.claude.com/docs/en/memory) to avoid copied rules.
Existing Codex entrypoints remain in place, so prior explicit calls still resolve.
CI reads canonical paths directly from its trusted policy checkout.

## Invocation

| Workflow | Codex | Claude Code |
| --- | --- | --- |
| Contract | `$strait-contract-review` | `/strait-contract-review` |
| Performance | `$strait-performance-check` | `/strait-performance-check` |
| Security | `$strait-security-review` | `/strait-security-review` |

Both discovery descriptions support task-relevant automatic selection. AGENTS.md
specifies required review routing independently of implicit selection. If discovery
is unavailable, instruct the agent to read `skills/<name>/SKILL.md` directly. These
are task workflows, not save hooks or a guarantee of model compliance. Current AI
CI runs Codex; adding Claude compatibility does not change its provider or billing.

## Checkout and Validation

Commit the canonical files and relative symlinks together. Check that links resolve
inside this repository after cloning or creating a worktree. On Windows, Git symlink
checkout requires appropriate symlink support; a checkout that materializes links
as text files does not provide automatic discovery. Such environments can read the
canonical files directly or use properly provisioned symlink support. Do not edit
or copy divergent rule files into the adapter directories.

`python3 -B .github/scripts/check-skill-layout.py` checks canonical frontmatter,
matching names, contained link targets, both discovery entrypoints, and the shared
CLAUDE.md import. CI runs it with the report-gate checks. Skill format validation is
also performed when authoring. Local validation establishes file layout, not that
a particular installed Codex/Claude version has loaded the skills; verify discovery
in a new session. No Claude API invocation is needed to share the instructions.
