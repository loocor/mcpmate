# RMCP Upgrade Inspector Validation Plan

Scope: validate the `rmcp` protocol SDK upgrade without changing MCPMate profile, unify, hosted, or API semantics.

## Preconditions

- Build from the `codex/rmcp-upgrade` worktree.
- Start backend from `backend/` with the same config/home used for the smoke run.
- Keep REST/API requests routed through approved tooling; do not use raw `curl` or `wget`.
- Use MCP Inspector CLI through Bun:

```sh
bunx --bun @modelcontextprotocol/inspector --cli http://127.0.0.1:8000/mcp --transport http --method <method>
```

## Inspector Protocol Checks

1. `initialize`
   - Confirm the server accepts the current MCP protocol version.
   - Confirm server implementation name/version and capability sections are present.
2. `tools/list`
   - Confirm built-in MCPMate tools are present.
   - In `unify` mode, confirm unify guide/action tools are visible and non-unify-only tools remain hidden.
   - In `hosted` mode, confirm hosted profile discovery/selection tools match the configured capability source.
3. `prompts/list`
   - Confirm expected MCPMate prompts are visible for the active mode.
   - Confirm profile/source switching emits visible list changes where supported by the client.
4. `resources/list` and `resources/templates/list`
   - Confirm hosted/profile resources match API profile capability settings.
   - Confirm disabled profile resources are absent from enabled-only surfaces.
5. `tools/call`
   - Call `mcpmate_profile_list`.
   - Call `mcpmate_profile_preview` or the active hosted profile selector with a known shared profile.
   - In `unify` mode, call the unify next-action tool and confirm it returns actionable JSON/text without direct unmanaged tool leakage.

## API Cross-Checks

1. Client management
   - Create or select a test client.
   - Switch `config_mode` across `transparent`, `hosted`, and `unify`.
   - Confirm Inspector `tools/list`, `prompts/list`, and `resources/list` reflect each mode.
2. Server lifecycle
   - Add a test server using the management API.
   - Confirm it appears in API list/detail responses.
   - Remove the server and confirm it disappears from API and Inspector surfaces.
3. Server capability toggles
   - Toggle a server capability off.
   - Confirm API state persists the disabled capability.
   - Confirm Inspector no longer exposes that capability in enabled-only mode.
   - Toggle it back on and confirm both API and Inspector surfaces recover.
4. Profile capability switches
   - Switch a profile server capability source for hosted mode.
   - Confirm API profile capability state and Inspector visible surfaces agree.
5. Origin/Host boundary
   - Confirm existing MCPMate Origin handling remains unchanged in this PR.
   - Record a follow-up if `/mcp` Origin/Host validation should move to `rmcp` transport config.

## Completion Evidence

Record:

- Backend start command and commit SHA.
- Inspector commands and pass/fail result.
- API requests exercised and pass/fail result.
- Any mismatch between API state and Inspector surfaces.
- Any `rmcp` protocol behavior that needs a separate follow-up PR.
