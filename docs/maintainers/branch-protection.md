# Branch Protection Setup

The integration flow is feature/fix branch → PR → `dev` → PR → `main`.
Neither `dev` nor `main` permits direct pushes once the remote ruleset is active.
PRs to `main` must originate from this repository's `dev`; PRs to `dev` may
originate from contributor branches, including forks.

## Repository Ruleset

A repository administrator must configure this on GitHub; local workflow files
alone do not prevent direct pushes or enforce required checks.

1. Merge `.github/workflows/branch-policy.yml` into the trusted protected branches
   before enabling its required check, and let the check run on a PR.
2. Open **Settings → Rules → Rulesets → New ruleset → New branch ruleset**.
3. Name it `protected-main-dev`, set enforcement to **Active**, and target the exact
   branch names `dev` and `main`. Keep the bypass list empty, including administrators
   and automation accounts.
4. Enable **Require a pull request before merging**, **Block force pushes**, and
   **Restrict deletions**. Do not enable **Restrict updates**: it blocks ordinary
   updates for users without bypass permission, including the intended merge flow.
5. Enable **Require status checks to pass**, add **PR branch policy**, and select
   GitHub Actions as the expected source. Also require the repository's applicable
   deterministic correctness, formatting, build, and dependency checks.
6. Require human review and conversation resolution as appropriate for the project.
   Require maintainer review for workflow/policy changes; do not rely solely on AI.

Administrators with settings access can still change the ruleset. Verify the active
rules using GitHub's branch rules view; an empty bypass list does not remove settings
administration rights.

## Source Check and Trust Boundary

`branch-policy.yml` exposes the uniquely named **PR branch policy** check. It uses
`pull_request_target` to evaluate trusted base-branch workflow logic, with no token
permissions, checkout, secrets, external actions, or execution of candidate code.
PR branch/repository names enter the shell only as quoted environment variables.
This event choice is specific to the metadata-only policy check; AI review still
uses its existing `pull_request` trust boundary.

The check runs when a PR is opened, updated, reopened, retargeted, or marked ready.
A fork branch named `dev` cannot pass for `main`. Without making the check required,
a failure is advisory and does not prevent merging.

After enabling the rules, verify:

| Operation | Expected result |
| --- | --- |
| Feature/fix PR → `dev` | Source check passes; other required checks/reviews still apply |
| This repository's `dev` PR → `main` | Source check passes; other required checks/reviews still apply |
| Any other branch or fork's `dev` PR → `main` | Source check fails; merge blocked |
| Direct push to `dev` or `main` | Rejected by the active ruleset |
| Force push or deletion of `dev`/`main` | Rejected by the active ruleset |

No merge queue integration is configured. Revisit required check triggers before
enabling a merge queue. Remote ruleset activation and live validation are separate
from local workflow validation.

See [GitHub ruleset documentation](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets).
