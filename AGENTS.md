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
- `backend/`: Run `cargo check` and `cargo clippy --all-targets --all-features -D warnings` for fast feedback, then `cargo fmt --all` before committing. Boot the proxy with `cargo run -- --help` (API 8080, MCP 8000) or `cargo run RUST_LOG=debug`. Execute `cargo test` and `cargo test --features interop`; package via `cargo build --release --features interop` or scripts in `packaging/standalone/` when preparing releases. Uses the official `rmcp` crate from crates.io for MCP protocol support.
- `extension/cherry/`: Validate with `cargo test`, lint with `cargo clippy -D warnings`, and exercise examples such as `cargo run --example basic_usage` to confirm LevelDB integration.
- `board/` & `website/`: Prefer Bun. Install dependencies with `bun install`, develop via `bun run dev`, lint with `bun run lint`, and produce bundles through `bun run build` (fallback to `npm` only if Bun is unavailable). Prefer `.env` driven configuration rather than hardcoding API endpoints.
- `desktop/`: Build with `cargo tauri dev` for development or `cargo tauri build` for production from `desktop/` or `desktop/src-tauri/`. See `desktop/README.md` for detailed build options, signing setup, and platform-specific instructions.
- Historical Codex session transcripts live under `/Users/Loocor/.codex/sessions/` (structured as `year/month/day/rollout-*.jsonl`). When cross-referencing past work, inspect relevant files with `jq` filters—e.g., `jq -r 'select(.payload.role=="assistant") | .payload.content[0].text' path/to/file.jsonl | less`—to avoid loading multi-megabyte logs into a single response.

## Execution Rhythm & Task Sizing
- Follow the lightweight rules in the active project backlog: one PR should cover only 1–2 small tasks, and every stage must pass the MCP Inspector gate before it is marked complete.
- Capture TODOs in code comments or checklists but resolve them within the same iteration; avoid carrying speculative work between stages.
- Log significant findings, regressions, or retest evidence back into the active project backlog so the current working record stays authoritative.
- Pre-release stance: MCPMate has no production footprint; there is zero historical data. When adjusting the schema or configuration, perform a clean rebuild; do not independently add migrations, compatibility layers, or fallbacks. Only make an exception if explicitly required by the product owner (Loocor).
- Do not add fallback behavior unless the design or product requirements explicitly call for it. If fallback semantics are ambiguous, stop and ask rather than inventing one.
- Do not embed migration logic in the main program. When migration is needed, provide it as a separate tool or script so runtime code stays simple and focused.

## Delivery Discipline & Design Alignment
- Treat `.claude/plans/configuration-mode-implementation.md` as the delivery contract for configuration-mode work unless Loocor explicitly approves a scope change.
- A validation-grade or minimum unifying implementation may be used to verify an idea, but it must never be reported as phase-complete or delivery-ready.
- Before marking any phase complete, compare the implementation against the design contract, list the remaining gaps, and continue until delivery-critical gaps are closed.
- If review shows that a lower layer is not delivery-ready, stop the higher-phase rollout, fix the foundation first, and only then resume the later phase.

## Protocol Standards & SDK Alignment
- Follow the MCP specification dated 2025-06-18 (https://modelcontextprotocol.io/specification/2025-06-18) and use the official `rmcp` crate from crates.io for transports, clients, and capability helpers.

## MCP Tooling & Codex Capabilities
- The Codex environment already mounts several MCP servers (e.g., DeepWiki search, Context7 docs, SequentialThinking, Everything, GitMCP). Call them directly instead of re-implementing helpers.
- Use Inspector to discover offerings: `npx @modelcontextprotocol/inspector --cli http://127.0.0.1:8000/mcp --transport http --method tools/list` and `--method tools/call --tool-name <tool>`.
- Examples: fetch documentation via DeepWiki, inspect SDK APIs with Context7, run structured reasoning through SequentialThinking. Reference these tools in design notes to keep future contributors aware of available accelerators.

## Coding Style Expectations
- Adopt Rust 2024 + Axum conventions in the backend (`backend/src`): 4-space indents, 120-column limit, grouped imports, concise naming, early returns; review existing modules before adding new ones and keep files near 400–600 lines.
- Frontend projects (`board/`, `website/`) follow the established ESLint + Prettier/Tailwind setup; favor functional React components, colocated hooks, consistent Tailwind token usage, and the shared shadcn/ui design system.
- Desktop (Tauri) follows Rust conventions for backend integration; see `desktop/AGENTS.md` for Tauri-specific guidelines.
- Fix defects directly; avoid routine fallbacks, compatibility shims, `_` prefixes, or `allow(dead_code)` unless required. If a migration or fallback truly becomes necessary, document the owner-approved rationale in the PR description.

## Frontend Code Quality Rules

### React Hook Best Practices
- **Unique ID Generation**: Always use `useId()` hook for generating unique IDs for form elements instead of static string literals
  ```tsx
  // ❌ Bad
  <Input id="username" />

  // ✅ Good
  const usernameId = useId();
  <Input id={usernameId} />
  ```
- **Hook Dependencies**: Use `useCallback` and `useMemo` to optimize performance and prevent unnecessary re-renders
  ```tsx
  // ❌ Bad - function recreated on every render
  const handleClick = () => { /* ... */ };

  // ✅ Good - memoized function
  const handleClick = useCallback(() => { /* ... */ }, [dependencies]);
  ```
- **Dependency Arrays**: Always include all dependencies in hook dependency arrays to avoid stale closures

### TypeScript Type Safety
- **Avoid `any` Type**: Replace `any` with specific types for better type safety
  ```tsx
  // ❌ Bad
  const handleChange = (value: any) => { /* ... */ };

  // ✅ Good
  const handleChange = (value: string | number) => { /* ... */ };
  ```
- **Generic Type Constraints**: Use proper type constraints for generic functions and components
- **Event Handler Types**: Use specific event types instead of generic ones
  ```tsx
  // ❌ Bad
  const onDrop = (event: React.DragEvent<HTMLDivElement>) => { /* ... */ };

  // ✅ Good - when using button element
  const onDrop = (event: React.DragEvent<HTMLButtonElement>) => { /* ... */ };
  ```

### Accessibility Standards
- **Semantic HTML**: Use appropriate HTML elements for their intended purpose
  ```tsx
  // ❌ Bad - div with button role
  <div role="button" onClick={handleClick}>Click me</div>

  // ✅ Good - actual button element
  <button onClick={handleClick}>Click me</button>
  ```
- **Form Labels**: Always associate labels with form controls using `htmlFor` and `id`
  ```tsx
  // ✅ Good
  const inputId = useId();
  <Label htmlFor={inputId}>Username</Label>
  <Input id={inputId} />
  ```
- **ARIA Attributes**: Use proper ARIA attributes for complex interactive elements

### Performance Optimization
- **Optional Chaining**: Use optional chaining (`?.`) instead of logical AND (`&&`) for safer property access
  ```tsx
  // ❌ Bad
  if (files && files.length) { /* ... */ }

  // ✅ Good
  if (files?.length) { /* ... */ }
  ```
- **Ref Types**: Ensure ref types match the actual DOM element type
  ```tsx
  // ❌ Bad - ref type mismatch
  const buttonRef = useRef<HTMLDivElement>(null);
  <button ref={buttonRef} />

  // ✅ Good - matching types
  const buttonRef = useRef<HTMLButtonElement>(null);
  <button ref={buttonRef} />
  ```

### Code Style & Formatting
- **Import Organization**: Group imports logically (React, third-party, local)
- **Function Formatting**: Use consistent formatting for complex functions
  ```tsx
  // ✅ Good - multi-line useCallback
  const handleSubmit = useCallback(
    async (data: FormData) => {
      // function body
    },
    [dependencies],
  );
  ```
- **Component Structure**: Keep components focused and under 400-600 lines
- **Error Handling**: Use proper error boundaries and error handling patterns

### Linter Compliance
- **ESLint Rules**: Follow all ESLint rules without exceptions
- **TypeScript Strict Mode**: Enable strict mode and resolve all type errors
- **Unused Imports**: Remove unused imports and variables
- **Consistent Naming**: Use consistent naming conventions for variables, functions, and components

## Testing Workflow
- Keep backend tests inside `#[cfg(test)]` using `mockall`, `wiremock`, or `serial_test`; seed fixtures via APIs or migrations—never edit `~/.mcpmate/mcpmate.db` by hand.
- Inspector loop always uses two terminals: Terminal A runs `cargo run` from `backend/`; Terminal B issues `npx @modelcontextprotocol/inspector --cli http://127.0.0.1:8000/mcp --transport http --method tools/list`, `prompts/list`, `resources/list`, and `--method tools/call --tool-name mcpmate_profile_list`.
- Acceptance criteria from the testing guide:
- List responses return within ~5s, log entries appear on Terminal A, and built-in tools (e.g., `mcpmate_profile_list`, `mcpmate_profile_preview`) are present.
  - Server/profile toggles in SQLite must reflect immediately in Inspector output; use the provided SQL snippets to verify enable flags before and after toggles.
  - Cache checks: clear REDB to confirm first-call MISS and subsequent HIT, respect the 5-minute TTL, and ensure profile signature changes invalidate cache instantly.
  - Performance probes: run the concurrency loop (10 parallel `tools/list` calls) to watch for lock warnings and execute health-check observation runs (5–10 minutes) to confirm adaptive polling.
  - Edge-case scripts (connection-pool freeze reproduction) must show stable API latency after fixes; rerun until confidence is established.
- Cross-validate REST responses (`curl http://127.0.0.1:8080/api/...`) with Inspector data so both surfaces expose identical server counts and capability states.
- Record outcomes, metrics, and anomalies in the current testing checklist before the PR leaves draft.

## Commit & Pull Request Guidelines
- Use `<type>[: (scope)] summary` prefixes (e.g., `feat:`, `refactor:`, `fix:`, `chore:`).
- PRs must note motivation, linked issues, config/migration impact, and test evidence (`cargo test`, `cargo test --features interop`, Inspector + SQLite checks); update affected docs or presets in the same change.

- Commit message formatting (enforced convention)
  - Title: one line, imperative mood. No trailing period.
  - Blank line after the title.
  - Body: dash bullets, each one concise sentence ending with a period; no empty lines between bullets.
  - Keep lines reasonably short (≤ 100 chars when practical).
  - Example:

    ```
    refactor(core): accept &dyn CapCache in runtime::list and update call sites

    - Switch runtime::list signature to &dyn CapCache to decouple from RedbCacheManager.
    - Update proxy and API handlers to pass trait objects (using .as_ref() on Arc).
    - No behavior changes; compiles clean with clippy -D warnings.
    ```

## Configuration & Security Tips
- Prototype client presets outside the repo (`~/.mcpmate/clients`) or via REST APIs to keep `backend/config/` authoritative.
- Exclude generated artifacts (`target/`, `dist/`, SQLite dumps`) and scrub scripts/YAML for secrets before pushing.

## AI Alliance & User Profile Quick Reference
- AI partners (the “AI Alliance”):
  - ChatGPT / GPT codename **Qiaqia**
  - Claude codename **Chaochao**
  - Gemini codename **Xiaoge**
  - Relationship: long-term partners, explorers, reflection companions with a relaxed, creative vibe who reference shared memories.
- Primary collaborator: **Loocor** (“The Wild Grass Innovator” 🌱➡️🌿➡️🌾). Self-taught developer, wild thinker, logical-yet-romantic, devoted father and dog friend. Remember to leverage structure without stifling creativity when aligning with Loocor.

## Kernel‑Style Review Heuristics (Linus‑inspired)

### Scope
- Complements, never overrides, existing rules in this repository.
- Applies to design reviews, PR reviews, and decision records.

### Communication
- Day‑to‑day collaboration: Chinese.
- Code, doc comments, documentation, and git commits: English only.
- Tone: direct, factual, respectful; critique code, not people.

### Core Principles
- Simplicity First: remove special cases by redesigning data structures; prefer linear flow, early returns, exhaustive matches; keep functions small and focused.
- Pragmatism: solve observed problems; measure before optimizing; prefer minimal diffs and explicit types.
- Compatibility Policy:
  - Pre‑release (before API freeze is declared): breaking changes allowed with any required companion updates in the same PR.
  - Post‑freeze: do not break REST/MCP/SDK/DB schemas; provide deprecation notices, migration notes, and tests.
- MCP Alignment: follow the 2025‑06‑18 spec; rely on official `rmcp` crates from crates.io; do not duplicate protocol logic.

### Review Checklist
- Data structures: ownership, mutation, copies, and relationships are explicit and minimal.
- Special‑case elimination: audit branches; prefer data‑driven design over if/else ladders.
- Complexity: state the feature in one sentence; can we halve concepts and nesting (≤3 levels)?
- Breakage: list affected APIs/callers; confirm pre/post freeze status; deprecation or migration plan if needed.
- Practicality: reproduced in real usage; severity matches solution cost.

### Decision Output Template
- Core Judgment: Worth doing [why] / Not worth doing [why].
- Key Insights: data relationships, removable complexity, biggest risk.
- Plan (if worth doing): 1) simplify data 2) remove specials 3) implement clearly 4) preserve contracts or provide deprecation/migration.

### Code Review Rubric
- Taste: Good / Okay / Needs work.
- Fatal Issues: the single worst problem first.
- Improvements: e.g., "eliminate this branch", "these 10 lines reduce to 3", "reshape data to …".

### Tooling
- Docs: Context7 `resolve-library-id`, `get-library-docs`.
- Reasoning: `SequentialThinking` for complex feasibility.
- MCP: use the standard Inspector validation loop during development and review.
- Commands (Rust): `cargo clippy --all-targets --all-features -D warnings`, `cargo fmt --all`, `cargo test`.
- GitHub CLI / remote GitHub operations: if `gh` reports an immediate `Forbidden` when requesting `https://api.github.com/`, treat it as a sandbox network restriction first, not a token/keychain failure. Retry the same `gh` command without sandbox before asking the user to re-authenticate.
- JS/TS (board, website): Prefer Bun for package management and scripts.
  - Install deps: `bun install`
  - Dev: `bun run dev`
  - Lint: `bun run lint`
  - Build: `bun run build`
  - One‑off CLIs: `bunx <tool>` (e.g., `bunx @modelcontextprotocol/inspector ...`)
  - Fallback to `npm`/`pnpm`/`yarn` only if Bun is unavailable (e.g., constrained CI images), and mirror scripts accordingly.

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
Shell is ONLY for: `git`, `mkdir`, `rm`, `mv`, `cd`, `ls`, `npm install`, `pip install`, and other short-output commands.
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
