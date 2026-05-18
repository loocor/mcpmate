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

## Validation matrix

Choose the rows that match the changed surface.

| Surface | Minimum validation |
| --- | --- |
| `backend/` Rust logic | `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings` |
| MCP surface or proxy behavior | Inspector loop with targeted `tools/list`, `prompts/list`, `resources/list`, and relevant tool calls |
| REST + MCP shared behavior | Cross-check routed HTTP results against Inspector output |
| `board/` or `website/` | `bun run lint`, `bun run build` |
| `desktop/` packaging or integration | Relevant Tauri build or smoke workflow for the touched path |
| Release or distribution scripts | Run the script or smoke command directly and record exact outcome |

## Rules

1. Do not skip validation because it might fail.
2. If a check fails, classify it as one of:
   - caused by the current change
   - pre-existing
   - unrelated transient/tooling issue
3. Fix current-change failures before reporting done.
4. Call out pre-existing or unrelated failures explicitly in the final report.
5. Use routed HTTP tooling instead of raw `curl` or `wget`.

## Inspector loop

When MCP behavior changes, use the standard two-terminal mental model:

- Terminal A: run the backend service.
- Terminal B: use `bunx --bun @modelcontextprotocol/inspector --cli http://127.0.0.1:8000/mcp --transport http` with the relevant list or call methods.

Scale the loop to the change. Simple metadata edits may need only targeted `tools/list`; behavior changes need the affected tool calls too.

## Evidence handling

Record the following in the PR or active Project item:

- Commands run.
- Pass or fail result.
- Any important timings or anomalies.
- Remaining risk or unverified edges.

## Final report shape

Report validation in three parts:

1. What you ran.
2. What passed or failed.
3. What residual risk remains, if any.
