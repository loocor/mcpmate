---
name: validation
description: Use this skill whenever MCPMate work needs test, lint, build, Inspector, or release-slice verification. Trigger on implementation, review, bug fixes, PR preparation, regression checks, or any request to validate backend, frontend, desktop, or MCP behavior before reporting completion.
---

# Validation

Use this skill to choose the smallest complete validation set for the touched surface, run it, and report the result clearly.

## Goals

- Match validation depth to the actual blast radius.
- Keep backend, frontend, desktop, and MCP verification consistent.
- Distinguish new failures from pre-existing failures instead of skipping checks.

## Rules

1. Do not skip validation because it might fail.
2. If a check fails, classify it as one of:
   - caused by the current change
   - pre-existing
   - unrelated transient/tooling issue
3. Fix current-change failures before reporting done.
4. Call out pre-existing or unrelated failures explicitly in the final report.
5. Use routed HTTP tooling instead of raw `curl` or `wget`.

## When to read more

Read `references/validation-playbook.md` when the task needs:

- the full surface-to-command matrix
- Inspector loop details
- evidence handling and reporting expectations

## Final report shape

Report validation in three parts:

1. What you ran.
2. What passed or failed.
3. What residual risk remains, if any.
