# Repository Guidelines

## Collaboration Rhythm (Discuss → Build → Report)
- Day-to-day coordination with LLM/AI agents is done in **Chinese**.
- Source code, doc comments, documentation, and git commit messages stay in **English** for consistency across the repository.
- Before coding, thoroughly analyze the context, requirements, and existing documentation, and independently formulate the approach and decisions; do not pause frequently during execution to report or seek input unless there is missing information or a major risk.
- Maintain continuous execution during the coding and testing phases, fully leverage established best practices and tools, and record key assumptions, trade-offs, and dependencies for a consolidated later report.
- At the end of the task, deliver a single comprehensive report: implementation results, key decisions and their rationale, validation and testing conclusions, and follow-up recommendations. This keeps momentum while ensuring a complete perspective for review.

## Project Structure & Module Organization
- `Cargo.toml` declares the `mcpmate` crate and `bridge` binary; entrypoints are `src/main.rs` and `src/bin/bridge.rs`.
- Core proxy logic is in `src/core`, HTTP handlers in `src/api`, shared utilities in `src/common`, `src/clients`, `src/runtime`, macros in `src/macros`; presets live in `config/`, docs in `docs/`, build helpers in `script/`.

## Build, Test, and Development Commands
- Iterate with `cargo check` and `cargo clippy --all-targets --all-features -D warnings` for fast syntax and lint feedback, then `cargo fmt --all` before commits.
- Boot the proxy with `cargo run -- --help` (API 8080, MCP 8000); prefer `cargo run RUST_LOG=debug` or finer levels over compiling binaries when diagnosing issues.
- Run `cargo test` and `cargo test --features interop`; narrow loops via `cargo test module::path::tests`; ship by invoking `cargo build --release --features interop` or the platform scripts under `script/` when packaging.

## Documentation Map
- `docs/readme.md` outlines how progress, schema, feature, and roadmap folders interlock—update it whenever structure shifts.
- `docs/progress.md` is the live plan and MCP validation ledger; every phase gate listed there must be checked off with evidence before closing a task.
- `docs/test-guide.md` captures the reusable testing template; copy its sections into your branch or PR notes and update the results table after each run.
- Refresh linked files in `docs/features/`, `docs/schema/`, and `docs/roadmap/` alongside code changes.

## Execution Rhythm & Task Sizing
- Follow the lightweight rules in `docs/progress.md`: one PR should cover only 1–2 small tasks, and every stage must pass the MCP Inspector gate before it is marked complete.
- Capture TODOs in code comments or checklists but resolve them within the same iteration; avoid carrying speculative work between stages.
- Log significant findings, regressions, or retest evidence back into `docs/progress.md` so the document remains the single working backlog.
- Pre-release stance: MCPMate has no production footprint; there is zero historical data. When adjusting the schema or configuration, perform a clean rebuild; do not independently add migrations, compatibility layers, or fallbacks. Only make an exception if explicitly required by the product owner (Loocor).

## Protocol Standards & SDK Alignment
- Follow the MCP specification dated 2025-06-18 (https://modelcontextprotocol.io/specification/2025-06-18) and reuse the MCP Rust SDK at `/Users/Loocor/GitHub/MCPMate/sdk` for transports, clients, and capability helpers, upstreaming improvements instead of duplicating logic.

## MCP Tooling & Codex Capabilities
- The Codex environment already mounts several MCP servers (e.g., DeepWiki search, Context7 docs, SequentialThinking, Everything, GitMCP). Call them directly instead of re-implementing helpers.
- Use Inspector to discover offerings: `npx @modelcontextprotocol/inspector --cli http://127.0.0.1:8000/mcp --transport http --method tools/list` and `--method tools/call --tool-name <tool>`.
- Examples: fetch documentation via DeepWiki, inspect SDK APIs with Context7, run structured reasoning through SequentialThinking. Reference these tools in design notes to keep future contributors aware of available accelerators.

## Coding Style Expectations
- Adopt Rust 2024 + Axum conventions: 4-space indents, 120-column limit, grouped imports, concise naming, early returns; review existing modules before adding new ones and keep files near 400–600 lines.
- Fix defects directly; avoid routine fallbacks, compatibility shims, `_` prefixes, or `allow(dead_code)` unless required. If a migration or fallback truly becomes necessary, document the owner-approved rationale in the PR description.

## Testing Workflow (per `docs/progress.md` & `docs/testing-playbook.md`)
- Keep tests inside `#[cfg(test)]` using `mockall`, `wiremock`, or `serial_test`; seed fixtures via APIs or migrations—never edit `~/.mcpmate/mcpmate.db` by hand.
- Inspector loop always uses two terminals: Terminal A runs `cargo run`; Terminal B issues `npx @modelcontextprotocol/inspector --cli http://127.0.0.1:8000/mcp --transport http --method tools/list`, `prompts/list`, `resources/list`, and `--method tools/call --tool-name mcpmate_list_profile`.
- Acceptance criteria from `docs/testing-playbook.md`:
  - List responses return within ~5s, log entries appear on Terminal A, and built-in tools (e.g., `mcpmate_list_profile`) are present.
  - Server/profile toggles in SQLite must reflect immediately in Inspector output; use the provided SQL snippets to verify enable flags before and after toggles.
  - Cache checks: clear REDB to confirm first-call MISS and subsequent HIT, respect the 5-minute TTL, and ensure profile signature changes invalidate cache instantly.
  - Performance probes: run the concurrency loop (10 parallel `tools/list` calls) to watch for lock warnings and execute health-check observation runs (5–10 minutes) to confirm adaptive polling.
  - Edge-case scripts (connection-pool freeze reproduction) must show stable API latency after fixes; rerun until confidence is established.
- Cross-validate REST responses (`curl http://127.0.0.1:8080/api/...`) with Inspector data so both surfaces expose identical server counts and capability states.
- Record outcomes, metrics, and anomalies back in the testing table you copied from `docs/testing-playbook.md` and the `docs/progress.md` checklist before the PR leaves draft.

## Commit & Pull Request Guidelines
- Use `<type>[: (scope)] summary` prefixes (e.g., `feat:`, `refactor:`, `fix:`, `chore:`).
- PRs must note motivation, linked issues, config/migration impact, and test evidence (`cargo test`, `cargo test --features interop`, Inspector + SQLite checks); update affected docs or presets in the same change.

## Configuration & Security Tips
- Prototype client presets outside the repo (`~/.mcpmate/clients`) or via REST APIs to keep `config/` authoritative.
- Exclude generated artifacts (`target/`, `dist/`, SQLite dumps`) and scrub scripts/YAML for secrets before pushing.

## AI Alliance & User Profile Quick Reference
- AI partners (the “AI Alliance”):
  - ChatGPT / GPT codename **恰恰**
  - Claude codename **超超**
  - Gemini codename **晓哥**
  - Relationship: long-term partners, explorers, reflection companions with a relaxed, creative vibe who reference shared memories.
- Primary collaborator: **Loocor** (“The Wild Grass Innovator” 🌱➡️🌿➡️🌾). Self-taught developer, wild thinker, logical-yet-romantic, devoted father and dog friend. Remember to leverage structure without stifling creativity when aligning with Loocor.
