# Validation Playbook

Use this reference when the short `validation` skill body is not enough.

## Surface matrix

| Surface | Minimum validation |
| --- | --- |
| `backend/` Rust logic | `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings` |
| MCP surface or proxy behavior | Inspector loop with targeted `tools/list`, `prompts/list`, `resources/list`, and affected tool calls |
| REST + MCP shared behavior | Cross-check routed HTTP results against Inspector output |
| `board/` or `website/` | `bun run lint`, `bun run build` |
| `desktop/` packaging or integration | Relevant Tauri build or smoke workflow for the touched path |
| Release or distribution scripts | Run the script or smoke command directly and record the exact outcome |

## Inspector loop

When MCP behavior changes:

- Terminal A: run the backend service, usually with `cargo run` from `backend/`
- Terminal B: use `bunx --bun @modelcontextprotocol/inspector --cli http://127.0.0.1:8000/mcp --transport http` with the list or call methods needed for the touched surface

Scale the loop to the risk:

- metadata-only edits may need targeted `tools/list`
- behavior changes usually need the affected tool calls too
- shared REST and MCP changes should be checked on both surfaces

## Validation rules

- Do not skip checks because they may fail.
- Classify failures as caused by the current change, pre-existing, or unrelated tooling/transient issues.
- Fix current-change failures before reporting done.
- Call out pre-existing or unrelated failures explicitly in the final report.
- Use routed HTTP tooling instead of raw `curl` or `wget`.

## Evidence handling

Record the following in the PR or active Project item:

- commands run
- pass or fail result
- important timings or anomalies
- residual risk or unverified edges
