---
name: project-flow
description: Use this skill whenever work needs to be aligned with the MCPMate GitHub Project, including roadmap slicing, draft-item planning, PR scoping, worktree setup, or task-status updates. Trigger on requests about planning, roadmap, project board, task center, worktree discipline, or how to split work into reviewable slices.
---

# Project Flow

Use this skill to keep implementation work anchored to the repository's GitHub Project workflow instead of letting planning drift into chat history.

## Goals

- Keep the GitHub Project `MCPMate` as the canonical task center.
- Keep one PR focused on one Project item or one clearly named sub-slice.
- Make planning portable across Mac mini, MacBook, mobile sessions, and later agent runs.

## When this skill should drive the work

Use it when the user asks for any of the following:

- Create, refine, or split roadmap tasks.
- Decide whether work should become a draft item, issue, or PR.
- Map a coding task to a Project item before implementation.
- Set up or audit worktree naming and ownership.
- Update task metadata, validation evidence, or PR links after progress.

## Workflow

1. Identify the active GitHub Project item before non-trivial work starts.
2. If no item exists, create or request a small draft item with a concrete scope.
3. Keep the slice narrow: one PR should usually cover one item, or at most one item plus one tightly related follow-up.
4. When a worktree is needed, align it to the Project item or sub-slice and use `.worktrees/<semantic-task-name>/`.
5. Link the PR, worktree branch, and validation evidence back to the same Project item.
6. After validation, leave the PR in a reviewable state for Loocor/Copilot review unless Loocor explicitly asks you to merge it.
7. Before closing the task, update Project fields and attach the final validation summary.

## Merge boundary

- Do not merge PRs, enable auto-merge, delete review branches, or mark Project items done unless Loocor explicitly requests that action in the current session.
- Treat "tests passed", "PR is ready", or "task is implemented" as readiness for human/Copilot review, not permission to merge.

## When to read more

Read `references/project-governance.md` when the task needs:

- full Project field expectations
- worktree discipline
- workflow run hygiene
- completion and reporting rules

## Reporting expectations

In the final report, include:

- Which Project item this work belongs to.
- What was updated in the Project record.
- What validation ran.
- Any remaining gap between the implementation and the Project or design contract.
