# AI Review Setup

The feature → `dev` → `main` PR flow and required remote ruleset settings are
documented in [branch protection setup](branch-protection.md).

`.github/workflows/skill-review.yml` invokes the applicable review workflows through the
[official Codex GitHub Action](https://learn.chatgpt.com/docs/github-action).
It explicitly names the skills and their repository paths; automatic discovery is
not required. Performance review applies to measured-path changes; other changes
are marked not applicable. The existing Rust CI remains separate.

To enable it after merging the workflow and skills:

1. Add an OpenAI API key as repository Actions secret `OPENAI_API_KEY`.
2. Set repository Actions variable `STRAIT_AI_REVIEW_ENABLED` to `true`.
3. Open/update a non-draft, same-repository PR targeting `dev` or `main` from an
   owner/member/collaborator, or run **Strait AI contract review** manually with a
   comparison base revision.

Automatic AI review runs before merging into `dev` or `main`. Pushes alone and PRs
targeting other branches do not trigger it; merging does not trigger a second run.
Keep work-in-progress PRs in draft to avoid API usage until ready for review. New
commits to an eligible ready PR trigger another review. Manual runs remain available
on a selected trusted workflow revision regardless of its branch.

The API-backed review incurs API usage. No credentials are stored in this repository.
When disabled or for fork PRs, the job is skipped; a skip does not satisfy review
requirements. Manual dispatch defaults to comparing against `HEAD^`; select the
appropriate base and a trusted workflow revision. Configuration errors and action
failures fail the job explicitly. Disabling the variable stops future invocations.

Reports are retained as workflow artifacts for 14 days. Baseline policies are checked
out, and candidate code is inspected through Git objects; merge these policy files
into the trusted base before enabling the workflow. The workflow has read-only
repository permissions and does not run repository code under the AI step. A separate
job downloads this run's report and creates or updates one bot comment on the PR,
including reviewed commit and workflow/artifact link. Only that job has PR write
permission; it has no OpenAI key and executes the trusted baseline publisher.
Reports with findings are published even when the disposition gate fails. Oversized
reports are truncated in the comment; artifacts retain the complete report. Stale,
closed, draft, or retargeted PRs are skipped before publication. Manual runs retain
artifacts without posting a PR comment. Neither job changes code.
Same-repository eligibility is not proof that
content is safe; restrict repository write access and review changes to workflow,
skills, and contract documents before enabling them. No `pull_request_target` run
checks out fork code with secrets.

This is static contract/performance-evidence review. It does not run benchmark or
profiling measurements on a stable reference machine, and the disposition gate fails for required changes, required evidence gaps,
inconclusive or malformed reports. Review findings before merging; do not use the AI job alone as a numerical performance or production gate.
Existing tests/benchmark builds remain executable CI checks. Live Actions execution
requires the configured repository secret/variable and is not validated locally.

Security, supervision, fuzzing, and soak requirements are defined in
[security and resilience](../security.md). Repository-admin branch/ruleset protection
and policy-file review remain required remote setup steps.

