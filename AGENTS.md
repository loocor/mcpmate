# Repository Guidelines

## Collaboration Rhythm (Discuss → Build → Report)
- Day-to-day coordination with LLM/AI agents is done in **Chinese**.
- Source code, doc comments, documentation, and git commit messages stay in **English** for consistency across the repository.
- Before coding, thoroughly analyze the context, requirements, and existing documentation, and independently formulate the approach and decisions; do not pause frequently during execution to report or seek input unless there is missing information or a major risk.
- Maintain continuous execution during the coding and testing phases, fully leverage established best practices and tools, and record key assumptions, trade-offs, and dependencies for a consolidated later report.
- After wrapping a Rust coding session, run `cargo clippy --all-targets --all-features -- -D warnings` to ensure we ship without lint regressions.
- At the end of the task, deliver a single comprehensive report: implementation results, key decisions and their rationale, validation and testing conclusions, and follow-up recommendations. This keeps momentum while ensuring a complete perspective for review.

## Project Structure & Module Organization
- `backend/`: Rust workspace containing the `mcpmate` proxy crate and the `bridge` binary; entrypoints live in `backend/src/main.rs` and `backend/src/bin/bridge.rs`. Core proxy logic sits under `backend/src/core`, HTTP handlers under `backend/src/api`, shared utilities in `backend/src/common`, `backend/src/clients`, and `backend/src/runtime`, with macros in `backend/src/macros`. Presets reside in `backend/config/`, docs in `backend/docs/`, and build helpers in `backend/script/`. Uses the official `rmcp` crate from crates.io for MCP protocol support.
- `extension/`: Extensions directory for optional integrations and plugins. Currently contains:
  - `cherry/`: Rust library that manages Cherry Studio LevelDB configurations with UTF-16 JSON encoding, exposing typed helpers to list, add, or remove MCP servers for desktop clients.
- `board/`: React + Vite operational dashboard (`mcpmate-dashboard`) that surfaces proxy state, analytics, and administrative flows; connects to the backend APIs via React Query, Zustand state, and Radix UI components.
- `website/`: Marketing and landing site built on Vite + React with Tailwind styling, housing public product messaging and contact flows.
- `desktop/`: Native Swift/SwiftUI macOS tray application delivering local MCPMate controls (status menu, server toggles, launch-at-login) and aligned with macOS design idioms.
- `docs/`: Mintlify documentation workspace (English/Chinese) configured via `docs/docs.json`; keep product narratives, onboarding guides, and changelog content synchronized with code milestones.

## Build, Test, and Development Commands
- `backend/`: Run `cargo check` and `cargo clippy --all-targets --all-features -D warnings` for fast feedback, then `cargo fmt --all` before committing. Boot the proxy with `cargo run -- --help` (API 8080, MCP 8000) or `cargo run RUST_LOG=debug`. Execute `cargo test` and `cargo test --features interop`; package via `cargo build --release --features interop` or scripts in `backend/script/` when preparing releases. Uses the official `rmcp` crate from crates.io for MCP protocol support.
- `extension/cherry/`: Validate with `cargo test`, lint with `cargo clippy -D warnings`, and exercise examples such as `cargo run --example basic_usage` to confirm LevelDB integration.
- `board/` & `website/`: Prefer Bun. Install dependencies with `bun install`, develop via `bun run dev`, lint with `bun run lint`, and produce bundles through `bun run build` (fallback to `npm` only if Bun is unavailable). Prefer `.env` driven configuration rather than hardcoding API endpoints.
- `desktop/`: Open `desktop/MCPMate.xcodeproj` in Xcode 14+ (`macOS 13+` target), run with ⌘R, and manage signing profiles inside Xcode; use `swift test` for pure Swift modules if applicable.
- `docs/`: Serve the Mintlify portal from `docs/` using the Mintlify CLI (`mintlify dev`, `mintlify build`) and keep localized content in `docs/i18n/` aligned when updating features.

## Documentation Map
- 根目录 `workspace-progress.md` 作为跨子项目的总览进度索引，按子项目提供当前任务、阻碍与证据链接；开启任务前先阅读对应段落，完成后回填更新。
- `backend/docs/readme.md` tracks how architecture notes, schema references, feature briefs, and roadmap folders interlock—update it whenever the backend structure shifts.
- `backend/docs/progress.md` is the live plan and MCP validation ledger; every phase gate listed there must be checked off with evidence before closing a task.
- `backend/docs/test-guide.md` captures the reusable testing template; copy its sections into your branch or PR notes and update the results table after each run.
- `backend/docs/features/`, `backend/docs/schema/`, and `backend/docs/roadmap/` house feature specs, data models, and milestone planning. Keep them in sync with implementation work.
- `docs/i18n/` mirrors the public Mintlify site; refresh English and Chinese pages when product behavior or UX flows change.
- Historical Codex session transcripts live under `/Users/Loocor/.codex/sessions/` (structured as `year/month/day/rollout-*.jsonl`). When cross-referencing past work, inspect relevant files with `jq` filters—e.g., `jq -r 'select(.payload.role=="assistant") | .payload.content[0].text' path/to/file.jsonl | less`—to avoid loading multi-megabyte logs into a single response.

## Execution Rhythm & Task Sizing
- Follow the lightweight rules in `backend/docs/progress.md`: one PR should cover only 1–2 small tasks, and every stage must pass the MCP Inspector gate before it is marked complete.
- Capture TODOs in code comments or checklists but resolve them within the same iteration; avoid carrying speculative work between stages.
- Log significant findings, regressions, or retest evidence back into `backend/docs/progress.md` so the document remains the single working backlog.
- Pre-release stance: MCPMate has no production footprint; there is zero historical data. When adjusting the schema or configuration, perform a clean rebuild; do not independently add migrations, compatibility layers, or fallbacks. Only make an exception if explicitly required by the product owner (Loocor).

## Protocol Standards & SDK Alignment
- Follow the MCP specification dated 2025-06-18 (https://modelcontextprotocol.io/specification/2025-06-18) and use the official `rmcp` crate from crates.io for transports, clients, and capability helpers.

## MCP Tooling & Codex Capabilities
- The Codex environment already mounts several MCP servers (e.g., DeepWiki search, Context7 docs, SequentialThinking, Everything, GitMCP). Call them directly instead of re-implementing helpers.
- Use Inspector to discover offerings: `npx @modelcontextprotocol/inspector --cli http://127.0.0.1:8000/mcp --transport http --method tools/list` and `--method tools/call --tool-name <tool>`.
- Examples: fetch documentation via DeepWiki, inspect SDK APIs with Context7, run structured reasoning through SequentialThinking. Reference these tools in design notes to keep future contributors aware of available accelerators.

## Coding Style Expectations
- Adopt Rust 2024 + Axum conventions in the backend (`backend/src`): 4-space indents, 120-column limit, grouped imports, concise naming, early returns; review existing modules before adding new ones and keep files near 400–600 lines.
- Frontend projects (`board/`, `website/`) follow the established ESLint + Prettier/Tailwind setup; favor functional React components, colocated hooks, consistent Tailwind token usage, and the shared shadcn/ui design system.
- Desktop Swift code should align with the latest Apple Human Interface Guidelines, Swift 5 conventions, and the existing SwiftUI architecture.
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

## Testing Workflow (per `backend/docs/progress.md` & `backend/docs/test-guide.md`)
- Keep backend tests inside `#[cfg(test)]` using `mockall`, `wiremock`, or `serial_test`; seed fixtures via APIs or migrations—never edit `~/.mcpmate/mcpmate.db` by hand.
- Inspector loop always uses two terminals: Terminal A runs `cargo run` from `backend/`; Terminal B issues `npx @modelcontextprotocol/inspector --cli http://127.0.0.1:8000/mcp --transport http --method tools/list`, `prompts/list`, `resources/list`, and `--method tools/call --tool-name mcpmate_profile_list`.
- Acceptance criteria from the testing guide:
  - List responses return within ~5s, log entries appear on Terminal A, and built-in tools (e.g., `mcpmate_profile_list`, `mcpmate_profile_details`) are present.
  - Server/profile toggles in SQLite must reflect immediately in Inspector output; use the provided SQL snippets to verify enable flags before and after toggles.
  - Cache checks: clear REDB to confirm first-call MISS and subsequent HIT, respect the 5-minute TTL, and ensure profile signature changes invalidate cache instantly.
  - Performance probes: run the concurrency loop (10 parallel `tools/list` calls) to watch for lock warnings and execute health-check observation runs (5–10 minutes) to confirm adaptive polling.
  - Edge-case scripts (connection-pool freeze reproduction) must show stable API latency after fixes; rerun until confidence is established.
- Cross-validate REST responses (`curl http://127.0.0.1:8080/api/...`) with Inspector data so both surfaces expose identical server counts and capability states.
- Record outcomes, metrics, and anomalies back in the testing table you copied from `backend/docs/test-guide.md` and the `backend/docs/progress.md` checklist before the PR leaves draft.

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
  - ChatGPT / GPT codename **恰恰**
  - Claude codename **超超**
  - Gemini codename **晓哥**
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
  - Pre‑release (before API freeze recorded in `backend/docs/progress.md`): breaking changes allowed with docs updated in the same PR.
  - Post‑freeze: do not break REST/MCP/SDK/DB schemas; provide deprecation notices, migration notes, and tests.
- MCP Alignment: follow the 2025‑06‑18 spec; reuse SDK crates under `sdk/crates`; do not duplicate protocol logic.

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
- MCP: Inspector loops per `backend/docs/test-guide.md`.
- Commands (Rust): `cargo clippy --all-targets --all-features -D warnings`, `cargo fmt --all`, `cargo test`.
- JS/TS (board, website): Prefer Bun for package management and scripts.
  - Install deps: `bun install`
  - Dev: `bun run dev`
  - Lint: `bun run lint`
  - Build: `bun run build`
  - One‑off CLIs: `bunx <tool>` (e.g., `bunx @modelcontextprotocol/inspector ...`)
  - Fallback to `npm`/`pnpm`/`yarn` only if Bun is unavailable (e.g., constrained CI images), and mirror scripts accordingly.
