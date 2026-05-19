# Project Governance

Use this reference when the task needs more than the short `project-flow` skill body.

## Canonical record

- GitHub Project `MCPMate` is the canonical task center.
- Use the active Project item to hold scope, validation evidence, linked PRs, and notable follow-up context.
- If no item exists for non-trivial work, create or request a small draft item before implementation.

## Project fields

Keep these fields current whenever they exist:

- `Status`
- `Track`
- `Release`
- `Priority`
- `Review Load`
- `Public`

Treat stale metadata as a workflow defect.

## Scoping and slicing

- One PR should usually map to one Project item or one clearly named sub-slice.
- Use draft items for uncertain work and convert or link them to issues once scope is stable.
- Keep sensitive strategy, unfinished market assumptions, or commercial planning private unless Loocor explicitly approves disclosure.
- Avoid broad branches or worktrees that mix unrelated tracks.

## PR lifecycle boundary

- Agents may create and update PRs for approved implementation tasks, but PR merge remains a product-owner action by default.
- Do not run `gh pr merge`, enable auto-merge, delete the review branch, or mark the linked Project item `Done` unless Loocor explicitly asks for that exact action in the current session.
- Validation success means the PR is ready for Loocor/Copilot review; it does not by itself authorize merge.
- Keep Project status at the most accurate pre-merge state and attach validation evidence so Loocor can request Copilot review manually.

## Worktree discipline

- Create task worktrees under `.worktrees/<semantic-task-name>/`.
- Keep each worktree aligned with one Project item or one clearly named sub-slice.
- Start worktree sessions by verifying `pwd`, `git branch --show-current`, and `git status --short`.
- Do not edit the main repository worktree for a task that already has a dedicated worktree unless Loocor explicitly asks for it.

## Workflow run hygiene

Treat GitHub Actions runs as short-lived operational evidence rather than permanent archive material.

- Do not delete runs that are still active, queued, or part of the current debugging loop.
- Keep a short lag window before cleanup so validation-in-progress is not disrupted.
- Prefer cleaning redundant or obsolete runs only after their signal has been captured in the PR or Project item.
- Preserve the newest meaningful success and failure pair while they still explain the branch state.
- Keep runs longer when they are the only evidence for release, packaging, signing, or flaky CI diagnosis.

## Completion record

Before reporting a task done:

- update the Project item status
- attach the final validation summary or PR link
- note any remaining delivery gap between implementation and the linked design contract
- if the PR has not been explicitly merged by Loocor, report it as ready for review rather than complete
