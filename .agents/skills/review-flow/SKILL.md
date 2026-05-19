---
name: review-flow
description: Use this skill whenever MCPMate work needs code review, PR readiness checks, severity calibration, or final findings ordering. Trigger on review requests, merge-readiness checks, PR cleanup, risk discussion, or any request to assess whether a change is ready to land.
---

# Review Flow

Use this skill to keep reviews concise, severity-driven, and aligned with repository rules.

## Goals

- Lead with the most serious problem first.
- Keep findings proportional to user or maintainer impact.
- Separate repository contract issues from style preferences.

## Workflow

1. Identify the touched contract surface: API, schema, MCP behavior, UI, release flow, or workflow rules.
2. Check whether validation depth matches the blast radius.
3. Report findings in severity order, or say clearly when no findings remain.
4. Keep merge-readiness notes narrow: unresolved risk, missing validation, or stale workflow state.
5. Treat "merge-ready" as a recommendation for Loocor/Copilot review unless Loocor explicitly asks you to merge the PR in the current session.

## When to read more

- Read `references/review-rubric.md` when the review needs the full decision and reporting rubric.
- Cross-reference the `validation` skill when a finding depends on missing or weak validation evidence.
