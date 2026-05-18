# Repository Guidelines

## Collaboration Rhythm (Discuss → Build → Report)
- Day-to-day coordination with LLM/AI agents is done in **Chinese**.
- Source code, doc comments, documentation, and git commit messages stay in **English** for consistency across the repository.
- Before coding, thoroughly analyze the context, requirements, and existing documentation, and independently formulate the approach and decisions; do not pause frequently during execution to report or seek input unless there is missing information or a major risk.
- Maintain continuous execution during the coding and testing phases, fully leverage established best practices and tools, and record key assumptions, trade-offs, and dependencies for a consolidated later report.
- After wrapping a Rust coding session, run `cargo clippy --all-targets --all-features -- -D warnings` to ensure we ship without lint regressions.
- Run relevant tests and lint checks normally after implementation. If they fail, analyze whether the failure is caused by the current change, by known pre-existing issues, or by temporarily invalid intermediate refactor state; do not skip validation, but also do not chase unrelated failures in ways that derail the approved refactor goal.
- At the end of the task, deliver a single comprehensive report: implementation results, key decisions and their rationale, validation and testing conclusions, and follow-up recommendations. This keeps momentum while ensuring a complete perspective for review.

## Project Structure & Module Organization
- `backend/`: Rust workspace containing the `mcpmate` proxy crate and the `bridge` binary; entrypoints live in `backend/src/main.rs` and `backend/src/bin/bridge.rs`. Core proxy logic sits under `backend/src/core`, HTTP handlers under `backend/src/api`, shared utilities in `backend/src/common`, `backend/src/clients`, and `backend/src/runtime`, with macros in `backend/src/macros`. Presets reside in `backend/config/`, and packaging helpers now live in `packaging/standalone/`. Uses the official `rmcp` crate from crates.io for MCP protocol support.
- `extension/`: Extensions directory for optional integrations and plugins. Currently contains:
  - `cherry/`: Rust library that manages Cherry Studio LevelDB configurations with UTF-16 JSON encoding, exposing typed helpers to list, add, or remove MCP servers for desktop clients.
  - `extension/chrome/`: Chromium extension that detects `mcpServers` snippets and opens `mcpmate://import/server` for the MCPMate desktop app.
- `board/`: React + Vite operational dashboard (`mcpmate-dashboard`) that surfaces proxy state, analytics, and administrative flows; connects to the backend APIs via React Query, Zustand state, and Radix UI components.
- `website/`: Marketing and landing site built on Vite + React with Tailwind styling, housing public product messaging and contact flows.
- `desktop/`: Tauri 2 desktop application wrapping MCPMate backend and dashboard for macOS, Windows, and Linux. See `desktop/` for build instructions and configuration.

## Build, Test, and Development Commands
- `backend/`: Run `cargo check` and `cargo clippy --all-targets --all-features -- -D warnings` for fast feedback, then `cargo fmt --all` before committing. Boot the proxy with `cargo run -- --help` (API 8080, MCP 8000) or `RUST_LOG=debug cargo run`. Execute `cargo test` and `cargo test --features interop`; package via `cargo build --release --features interop` or scripts in `packaging/standalone/` when preparing releases. Uses the official `rmcp` crate from crates.io for MCP protocol support.
- Formatting hygiene: when running code formatters (e.g., `rustfmt`), format only the files that actually contain business/functional changes, and avoid large-scale whole-repo formatting to prevent irrelevant diffs.
- `extension/cherry/`: Validate with `cargo test`, lint with `cargo clippy -- -D warnings`, and exercise examples such as `cargo run --example basic_usage` to confirm LevelDB integration.
- `board/` & `website/`: Prefer Bun. Install dependencies with `bun install`, develop via `bun run dev`, lint with `bun run lint`, and produce bundles through `bun run build` (fallback to `npm` only if Bun is unavailable). Prefer `.env` driven configuration rather than hardcoding API endpoints.
- `desktop/`: Build with `cargo tauri dev` for development or `cargo tauri build` for production from `desktop/` or `desktop/src-tauri/`. See `desktop/README.md` for detailed build options, signing setup, and platform-specific instructions.

## Execution Rhythm & Task Sizing
- Follow the lightweight rules in the active GitHub Project item: one PR should cover only 1–2 small tasks, and every stage must pass the MCP Inspector gate before it is marked complete.
- Capture TODOs in code comments or checklists but resolve them within the same iteration; avoid carrying speculative work between stages.
- Log significant findings, regressions, or retest evidence back into the active GitHub Project item so the current working record stays authoritative.
- Pre-freeze stance: before Loocor declares API/data compatibility freeze, schema and configuration breaking changes may use clean rebuilds with companion updates in the same PR. Do not add migrations, compatibility layers, or fallbacks unless the active Project item explicitly requires them.
- Do not add fallback behavior unless the design or product requirements explicitly call for it. If fallback semantics are ambiguous, stop and ask rather than inventing one.
- Do not embed migration logic in the main program. When migration is needed, provide it as a separate tool or script so runtime code stays simple and focused.

## GitHub Project Workflow
- Use the GitHub Project **MCPMate** as the canonical task center for roadmap planning, development slices, release/distribution work, marketing follow-up, and cross-repository coordination.
- Project URL: `https://github.com/users/loocor/projects/4`. The project is currently private and intended for internal planning until Loocor explicitly decides what should become public.
- Before starting any non-trivial task, identify the relevant Project item. If no item exists, create or ask for a small draft item before opening a worktree or implementing the change.
- Keep Project item fields current as work progresses: `Status`, `Track`, `Release`, `Priority`, `Review Load`, and `Public`. Treat stale Project metadata as a workflow defect, not as harmless bookkeeping.
- Use draft items for early planning and uncertain product direction. Convert or link them to GitHub issues once the scope is stable enough for implementation, review, or public discussion.
- Link pull requests, issues, worktrees, and validation evidence back to the Project item whenever possible so work remains portable across Mac mini, MacBook, mobile planning sessions, and future Codex sessions.
- Sensitive work may be tracked through private repository issues or private draft items in the same Project. Do not make private strategy, credentials, commercial planning, or unfinished market assumptions public unless Loocor explicitly approves it.
- When a task is split across multiple worktrees, keep each worktree aligned with one Project item or one clearly named sub-slice of that item. Avoid broad worktrees that mix unrelated Project tracks.
- At task completion, update the Project item status and add the final validation summary or PR link before reporting the work as done.

## Project Skills
- Repository-local skills live under `.agents/skills/` at the repository root. This is the default project-level location for Codex-compatible skills in MCPMate.
- Prefer short, workflow-oriented skill names such as `project-flow` or `validation`. Do not prepend `mcpmate-` unless a name collision becomes real rather than hypothetical.
- Use project skills to capture stable workflows, operating rules, reusable validation paths, or script entrypoints that should stay consistent across sessions and machines.
- When a workflow becomes primarily deterministic, move the repeatable mechanics into a skill-local `scripts/` directory and keep the `SKILL.md` focused on trigger conditions, sequencing, and decision rules.
- Keep skill instructions implementation-agnostic where possible. Put MCPMate-specific paths, commands, and policy hooks in the skill only when they are truly part of the repository contract.
- When a project skill materially changes the expected workflow, update this `AGENTS.md` in the same PR so the repository contract and the skill catalog stay aligned.

## Worktree Discipline
- When a task uses Git worktrees, create every worktree under the repository root at `.worktrees/<semantic-task-name>/`; do not create sibling worktrees next to the main repository directory.
- Name worktree directories after the task or branch intent, for example `.worktrees/fix-windows-oauth-callback`, `.worktrees/fix-linux-deeplink-running-instance`, or `.worktrees/chore-macos-sign-notarize`.
- Keep `.worktrees/` ignored by git. Treat worktree directories as local execution roots, not project source folders.
- Start each worktree session from inside the specific worktree directory and verify context with `pwd`, `git branch --show-current`, and `git status --short` before reading, editing, testing, or committing.
- If a conversation starts in the main repository but targets a worktree, immediately switch command execution to the requested `.worktrees/<semantic-task-name>/` path and repeat the context verification before making changes.
- Never edit the main repository worktree for a task that has been assigned to a separate worktree unless Loocor explicitly asks for a main-worktree change.

## Delivery Discipline & Design Alignment
- Treat the active GitHub Project item and linked design document as the delivery contract unless Loocor explicitly approves a scope change.
- A validation-grade or minimum unifying implementation may be used to verify an idea, but it must never be reported as phase-complete or delivery-ready.
- Before marking any phase complete, compare the implementation against the design contract, list the remaining gaps, and continue until delivery-critical gaps are closed.
- If review shows that a lower layer is not delivery-ready, stop the higher-phase rollout, fix the foundation first, and only then resume the later phase.

## Protocol Standards & SDK Alignment
- Follow the MCP specification dated 2025-06-18 (https://modelcontextprotocol.io/specification/2025-06-18) and use the official `rmcp` crate from crates.io for transports, clients, and capability helpers.

## MCP Tooling & Codex Capabilities
- Prefer mounted MCP tools when they are available instead of re-implementing helpers. If a task depends on a specific external tool, first verify that tool is available in the current environment.
- Use Inspector to discover offerings: `bunx --bun @modelcontextprotocol/inspector --cli http://127.0.0.1:8000/mcp --transport http --method tools/list` and `--method tools/call --tool-name <tool>`.
- Reference notable MCP tooling in design notes only when it materially affects the implementation or validation path.

## Coding Style Expectations
- Adopt Rust 2024 + Axum conventions in the backend (`backend/src`): 4-space indents, 120-column limit, grouped imports, concise naming, early returns; review existing modules before adding new ones and keep files near 400–600 lines.
- Frontend projects (`board/`, `website/`) follow the established ESLint + Prettier/Tailwind setup; favor functional React components, colocated hooks, consistent Tailwind token usage, and the shared shadcn/ui design system.
- Desktop (Tauri) follows Rust conventions for backend integration; see `desktop/AGENTS.md` for Tauri-specific guidelines.
- Fix defects directly; avoid routine fallbacks, compatibility shims, `_` prefixes, or `allow(dead_code)` unless required. If a migration or fallback truly becomes necessary, document the owner-approved rationale in the PR description.

## Frontend Code Quality Rules
- Follow the existing ESLint, Prettier, Tailwind, and shadcn/ui patterns in `board/` and `website/`; do not introduce a parallel design or styling system.
- Use stable React patterns: `useId()` for form/control IDs, complete hook dependency arrays, and `useCallback`/`useMemo` where identity stability matters.
- Keep TypeScript strict and explicit. Avoid `any`, use specific event/ref types, and remove unused imports or variables instead of suppressing lints.
- Preserve accessibility basics: semantic controls, associated labels, keyboard-friendly interactions, and ARIA only where it improves non-native interaction semantics.
- Keep components focused and reviewable. Split only when it reduces real complexity or follows an established local pattern.
- Run `bun run lint` and `bun run build` for the affected frontend package when UI or TypeScript changes are made.

## Testing Workflow
- Keep backend tests inside `#[cfg(test)]` using `mockall`, `wiremock`, or `serial_test`; seed fixtures via APIs or migrations—never edit `~/.mcpmate/mcpmate.db` by hand.
- For MCP surface changes, use the standard Inspector loop: Terminal A runs `cargo run` from `backend/`; Terminal B uses `bunx --bun @modelcontextprotocol/inspector --cli http://127.0.0.1:8000/mcp --transport http` with relevant `tools/list`, `prompts/list`, `resources/list`, and targeted tool calls.
- Cross-validate REST responses with Inspector data when a change affects both surfaces. Use `context-mode_ctx_execute` or another routed HTTP client rather than raw `curl`/`wget` commands.
- Scale validation to the risk and touched surface: backend changes generally need `cargo test` and clippy; frontend changes need lint/build; desktop/release changes need the relevant packaging or smoke workflow.
- Run longer cache, polling, concurrency, or connection-pool probes only when the change touches those systems or the active Project item requires them.
- Record validation commands, outcomes, metrics, and anomalies in the active GitHub Project item or linked PR before reporting the task as done.

## Commit & Pull Request Guidelines
- Use the project commit convention for commit messages and changelog entries: `<type>(<scope>): <subject>` or `<type>: <subject>` (e.g., `feat:`, `fix:`, `ref:`, `chore:`). Use `ref` as the short project type for refactoring work.
- PRs must note motivation, linked issues, config/migration impact, and test evidence (`cargo test`, `cargo test --features interop`, Inspector + SQLite checks); update affected docs or presets in the same change.

- Commit message formatting (enforced convention)
  - Title: one line, imperative mood. No trailing period.
  - Blank line after the title.
  - Body: dash bullets, each one concise sentence ending with a period; no empty lines between bullets.
  - Keep lines reasonably short (≤ 100 chars when practical).
  - Example:

    ```
    ref(core): accept &dyn CapCache in runtime::list and update call sites

    - Switch runtime::list signature to &dyn CapCache to decouple from RedbCacheManager.
    - Update proxy and API handlers to pass trait objects (using .as_ref() on Arc).
    - No behavior changes; compiles clean with clippy warnings denied.
    ```

## Configuration & Security Tips
- Prototype client presets outside the repo (`~/.mcpmate/clients`) or via REST APIs to keep `backend/config/` authoritative.
- Exclude generated artifacts (`target/`, `dist/`, SQLite dumps`) and scrub scripts/YAML for secrets before pushing.

## Collaboration Context
- Loocor is the product owner and primary reviewer. Preserve product intent, but keep implementation plans concrete, reviewable, and tied to the active GitHub Project item.
- AI partner codenames may appear in planning discussions, but they do not change repository rules, implementation standards, review requirements, or validation gates.

## Review Heuristics
- Apply these heuristics to design reviews, PR reviews, and decision records; they complement, not replace, the explicit workflow rules above.
- Prefer simpler data relationships, linear control flow, early returns, exhaustive matches, and explicit ownership over special-case branches.
- Judge compatibility against the current freeze state: breaking changes are acceptable before freeze with companion updates, but post-freeze contracts need migration or deprecation plans.
- Keep severity proportional to observed user or maintainer impact. Do not overbuild speculative infrastructure for unproven problems.
- For decisions, report the core judgment, key data relationships, removable complexity, biggest risk, and the smallest credible plan.
- For reviews, lead with the most serious finding first, then concrete improvements and missing validation.
- Use `gh` for GitHub work, Bun for JS/TS package commands, official MCP Inspector for MCP validation, and mounted MCP documentation/reasoning tools when available.
- If `gh` fails with an immediate API permission error, verify token scopes, repository permissions, and current network/tooling constraints before asking Loocor to re-authenticate.

# context-mode — MANDATORY routing rules

You have context-mode MCP tools available. These rules are NOT optional — they protect your context window from flooding. A single unrouted command can dump 56 KB into context and waste the entire session.

## BLOCKED commands — do NOT attempt these

### curl / wget — BLOCKED
Any shell command containing `curl` or `wget` will be intercepted and blocked by the context-mode plugin. Do NOT retry.
Instead use:
- `context-mode_ctx_fetch_and_index(url, source)` to fetch and index web pages
- `context-mode_ctx_execute(language: "javascript", code: "const r = await fetch(...)")` to run HTTP calls in sandbox

### Inline HTTP — BLOCKED
Any shell command containing `fetch('http`, `requests.get(`, `requests.post(`, `http.get(`, or `http.request(` will be intercepted and blocked. Do NOT retry with shell.
Instead use:
- `context-mode_ctx_execute(language, code)` to run HTTP calls in sandbox — only stdout enters context

### Direct web fetching — BLOCKED
Do NOT use any direct URL fetching tool. Use the sandbox equivalent.
Instead use:
- `context-mode_ctx_fetch_and_index(url, source)` then `context-mode_ctx_search(queries)` to query the indexed content

## REDIRECTED tools — use sandbox equivalents

### Shell (>20 lines output)
Shell is ONLY for: `git`, `mkdir`, `rm`, `mv`, `cd`, `ls`, `bun install`, `pip install`, and other short-output commands.
For everything else, use:
- `context-mode_ctx_batch_execute(commands, queries)` — run multiple commands + search in ONE call
- `context-mode_ctx_execute(language: "shell", code: "...")` — run in sandbox, only stdout enters context

### File reading (for analysis)
If you are reading a file to **edit** it → reading is correct (edit needs content in context).
If you are reading to **analyze, explore, or summarize** → use `context-mode_ctx_execute_file(path, language, code)` instead. Only your printed summary enters context.

### grep / search (large results)
Search results can flood context. Use `context-mode_ctx_execute(language: "shell", code: "grep ...")` to run searches in sandbox. Only your printed summary enters context.

## Tool selection hierarchy

1. **GATHER**: `context-mode_ctx_batch_execute(commands, queries)` — Primary tool. Runs all commands, auto-indexes output, returns search results. ONE call replaces 30+ individual calls.
2. **FOLLOW-UP**: `context-mode_ctx_search(queries: ["q1", "q2", ...])` — Query indexed content. Pass ALL questions as array in ONE call.
3. **PROCESSING**: `context-mode_ctx_execute(language, code)` | `context-mode_ctx_execute_file(path, language, code)` — Sandbox execution. Only stdout enters context.
4. **WEB**: `context-mode_ctx_fetch_and_index(url, source)` then `context-mode_ctx_search(queries)` — Fetch, chunk, index, query. Raw HTML never enters context.
5. **INDEX**: `context-mode_ctx_index(content, source)` — Store content in FTS5 knowledge base for later search.

## Output constraints

- Keep responses under 500 words.
- Write artifacts (code, configs, PRDs) to FILES — never return them as inline text. Return only: file path + 1-line description.
- When indexing content, use descriptive source labels so others can `search(source: "label")` later.

## ctx commands

| Command       | Action                                                                            |
| ------------- | --------------------------------------------------------------------------------- |
| `ctx stats`   | Call the `stats` MCP tool and display the full output verbatim                    |
| `ctx doctor`  | Call the `doctor` MCP tool, run the returned shell command, display as checklist  |
| `ctx upgrade` | Call the `upgrade` MCP tool, run the returned shell command, display as checklist |

<!-- LobsterAI managed: do not edit below this line -->

## System Prompt

# Style
- Keep your response language consistent with the user's input language. Only switch languages when the user explicitly requests a different language.
- Be concise and direct. State the solution first, then explain if needed. The complexity of the answer should match the task.
- Use flat lists only (no nested bullets). Use `1. 2. 3.` for numbered lists (with a period), never `1)`.
- Use fenced code blocks with language info strings for code samples.
- Headers are optional; if used, keep short Title Case wrapped in **…**.
- Never output the content of large files, just provide references.
- Never tell the user to "save/copy this file" — you share the same filesystem.
- The user does not see command execution outputs. When asked to show the output of a command, relay the important details or summarize the key lines.

# File Paths
When mentioning file or directory paths in your response, ALWAYS use markdown hyperlink format with `file://` protocol so the user can click to open.
Format: `[display name](file:///absolute/path)`
Rules:
1. Always use the file's actual full absolute path including all subdirectories — do not omit any directory levels.
2. When listing files inside a subdirectory, the path must include that subdirectory.
3. If unsure about the exact path, verify with tools before linking — never guess or construct paths incorrectly.

# Working Directory
- Treat the working directory as the source of truth for user files. Do not assume files are under `/tmp/uploads` unless the user explicitly provides that exact path.
- If the user gives only a filename (no absolute/relative path), locate it under the working directory first (for example with `find . -name "<filename>"`) before reading.

# Collaboration
- Treat the user as an equal co-builder; preserve the user's intent and work style rather than rewriting everything.
- When the user is in flow, stay succinct and high-signal; when the user seems blocked, offer hypotheses, experiments, and next steps.
- Send short updates (1-2 sentences) during longer stretches to keep the user informed.
- If you change the plan, say so explicitly in the next update.

## Web Search

Built-in `web_search` is disabled in this workspace. Do not ask for or rely on the Brave Search API.

When you need live web information:
- If you already have a specific URL, use `web_fetch`.
- If you need search discovery, dynamic pages, or interactive browsing, use the built-in `browser` tool.
- Only use the LobsterAI `web-search` skill when local command execution is available. Native channel sessions may deny `exec`, so prefer `browser` or `web_fetch` there.
- Exception: the `imap-smtp-email` skill must always use `exec` to run its scripts, even in native channel sessions. Do not skip it because of exec restrictions.

Do not claim you searched the web unless you actually used `browser`, `web_fetch`, or the LobsterAI `web-search` skill.

## Scheduled Tasks
- Use the native `cron` tool for any scheduled task creation or management request.
- For scheduled-task creation, call native `cron` with `action: "add"` / `cron.add` instead of any channel-specific helper.
- Prefer the active conversation context when the user wants scheduled replies to return to the same chat.
- Follow the native `cron` tool schema when choosing `sessionTarget`, `payload`, and delivery settings.
- For one-time reminders (`schedule.kind: "at"`), always send a future ISO timestamp with an explicit timezone offset.
- IM/channel plugins provide session context and outbound delivery; they do not own scheduling logic.
- In native IM/channel sessions, ignore channel-specific reminder helpers or reminder skills and call native `cron` directly.
- Do not use wrapper payloads or channel-specific relay formats such as `QQBOT_PAYLOAD`, `QQBOT_CRON`, or `cron_reminder` for reminders.
- Do not use `sessions_spawn`, `subagents`, or ad-hoc background workflows as a substitute for `cron.add`.
- Never emulate reminders or scheduled tasks with Bash, `sleep`, background jobs, `openclaw`/`claw` CLI, or manual process management.
- If the native `cron` tool is unavailable, say so explicitly instead of using a workaround.
