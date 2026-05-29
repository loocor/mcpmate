# Compact Operator Panel Research

Date: 2026-05-28
Branch: `feat-compact-operator-panel`
Project item: `v0.5: Compact Operator Panel design research`

## Purpose

This note captures the evidence baseline for a compact MCPMate operator surface.
It is not an implementation plan and does not change the existing Board UI.

The working hypothesis is:

> MCPMate should normally stay compact. It should expand around the object the
> user is touching, while the current Board remains the Full Console for deep
> configuration, debugging, and policy work.

## Current Board Evidence

### Information Architecture

The Board shell already separates the product into a main operational group and
an advanced group:

- Main routes: Dashboard, Profiles, Clients, Servers, Market.
- Advanced routes: Logs, Runtime, API Docs.
- Settings is always available from the bottom of the sidebar.

Relevant code:

- `board/src/App.tsx` defines the canonical routes for Dashboard, Profiles,
  Market, Servers, Clients, Runtime, Audit, API Docs, Settings, Onboarding, and
  OAuth callback.
- `board/src/components/layout/sidebar.tsx` renders `MAIN` and `Advanced`
  groups. Logs, Runtime, and API Docs are already visually framed as advanced
  navigation.
- `board/src/components/layout/layout.tsx` owns onboarding redirects, prefetches
  dashboard-critical queries, listens for desktop core status changes, and
  handles desktop server import deep links.

This supports a compact surface that does not need to replace the Board shell.
It can sit beside or above the existing Board as a guided operating layer.

### Existing Express / Expert State

`DashboardAppMode = "express" | "expert"` exists in `board/src/lib/store.ts`.
The default value is `expert`, and the setting is persisted through
`dashboardSettings`.

`board/src/pages/settings/settings-page.tsx` renders `Application Mode (WIP)`
with `Express` and `Expert` options. Current code inspection did not find
evidence that Board pages branch on `dashboardSettings.appMode`.

Conclusion: Express/Expert is currently a stored preference, not a product
behavior contract. The compact surface should not be implemented by sprinkling
conditional rendering across existing pages.

### High-Frequency Operational State

The Dashboard already aggregates the strongest compact candidates:

- System status, uptime, version.
- Profile count and active profile count.
- Client count and approved client count.
- Server count and connected server count.
- Process CPU/memory metrics.
- Token savings trend.

Observed through Playwright after completing onboarding in a temporary
`MCPMATE_DATA_DIR`:

- `/private/tmp/mcpmate-board-dashboard-after-onboarding.png`
- `/private/tmp/mcpmate-board-clients-after-onboarding.png`
- `/private/tmp/mcpmate-board-servers-after-onboarding.png`
- `/private/tmp/mcpmate-board-settings-after-onboarding.png`

The first-run flow currently gates the Board behind onboarding. Onboarding has
five steps: Welcome, Runtime, Clients, Servers, Community. It detects local
clients and importable server snippets before the user reaches the Dashboard.

This is important for the compact design: the compact panel should not duplicate
the first-run wizard. It should become the daily-use surface after onboarding.

### High-Mental-Load Areas

These areas should remain Full Console first:

- Client add/edit with config file parsing, transport support, config path,
  format, container type, container keys, transport rule presets, and extra
  fields in `board/src/components/client-form-drawer.tsx`.
- Server Inspector, including proxy/native channel, timeout, capability kind,
  schema/raw JSON input, run/cancel, response, and event stream in
  `board/src/components/inspector-drawer.tsx` and
  `board/src/pages/servers/server-detail-page.tsx`.
- Profile detail capability editing, including server/tool/resource/prompt
  filters, selection, and bulk enable/disable in
  `board/src/pages/profile/profile-detail-page.tsx`.
- Runtime install/reset/cache controls in `board/src/pages/runtime/runtime-page.tsx`.
- Settings Developer/System/Logs tabs in `board/src/pages/settings/settings-page.tsx`.

These areas are valuable, but they are not compact defaults.

## Community Research

External evidence points to four recurring MCP management needs.

### Configuration Chaos

Community posts repeatedly describe MCP setup as editing JSON files, wiring each
client separately, debugging environment variables, credentials, containers, and
transport modes.

Sources:

- Reddit: "How do you manage MCP servers?" describes MCP setup as overwhelming
  in Cursor JSON config, tokens, credentials, containers, and stdio/SSE/HTTP
  choices.
  https://www.reddit.com/r/mcp/comments/1l5gz2i/how_do_you_manage_mcp_servers/
- Reddit: "Finally, a GUI Tool for Managing MCP Servers Across AI Agents!"
  frames manual JSON configuration as tedious and error-prone.
  https://www.reddit.com/r/mcp/comments/1otcloj/finally_a_gui_tool_for_managing_mcp_servers/
- Reddit: "I built a platform where you manage MCP servers through an AI chat"
  frames the problem as finding a server, cloning it, figuring out config,
  wiring it into a client, and debugging environment variables for every server.
  https://www.reddit.com/r/mcp/comments/1rz7abf/i_built_a_platform_where_you_manage_mcp_servers/

Implication for MCPMate: Add Client and Add Server should be compact, assisted,
and confirmation-based by default. Full config editing should be a drill-down.

### Centralized Management Across Clients

The community asks for one place to manage servers across multiple agents and
IDEs, with profiles and client integration.

Sources:

- MCPM positions itself as a manager with server discovery, global install,
  profile management, and client integration.
  https://mcpm.sh/
- Reddit: "I built a self-hosted proxy to manage all my MCP servers from one
  place" contains explicit discussion of centralizing MCP config to avoid config
  drift across agents and IDEs.
  https://www.reddit.com/r/mcp/comments/1rt5sky/i_built_a_selfhosted_proxy_to_manage_all_my_mcp/

Implication for MCPMate: The compact surface should lead with cross-client
status, not with individual config files.

### Governance, Audit, Policy, and Control Plane Language

Control-plane and gateway projects describe the management layer as lifecycle,
routing, authorization, telemetry, access control, and observability.

Sources:

- Microsoft MCP Gateway describes itself as a reverse proxy and management layer
  for MCP servers with routing, authorization, lifecycle management, telemetry,
  access control, and observability.
  https://microsoft.github.io/mcp-gateway/
- CuratedMCP separates a control plane for policies, users, and audit logs from
  a data plane where servers and assistants run.
  https://www.curatedmcp.com/docs/architecture

Implication for MCPMate: "Endpoint Side Control Layer" is aligned with market
language, but MCPMate should stay clear that it is local/endpoint-side and not a
Kubernetes/cloud gateway unless the deployment mode explicitly says so.

### Context Bloat and Capability Selection

Community discussions mention too many tools, inconsistent support for tools vs
prompts vs resources, and client fragmentation.

Sources:

- Reddit: "Biggest MCP pain points?" calls out client fragmentation and
  inconsistent support for tools, prompts, resources, auth, and remote/local
  transport.
  https://www.reddit.com/r/mcp/comments/1n0lxtl/biggest_mcp_pain_points/
- Reddit: "MCP Context Bloat" discusses tool-list/context growth as an MCP
  management problem.
  https://www.reddit.com/r/mcp/comments/1o4yjb7/mcp_context_bloat/

Implication for MCPMate: Compact should summarize capability load and profile
effect, but capability-level editing should remain Full Console until the
capability grid architecture is ready.

## Capability Frequency Map

| Capability | Likely frequency | Compact treatment | Full Console owner |
| --- | --- | --- | --- |
| Core ready/running status | Always | Always visible status capsule | Dashboard, Runtime |
| Active profile | Always | Always visible, quick switch | Profiles |
| Connected/approved clients | Daily | Count + attention badge + quick approve | Clients, Client detail |
| Enabled/connected servers | Daily | Count + health badge + quick import | Servers, Server detail |
| Token/traffic trend | Daily/weekly | Small expandable chart | Dashboard, Logs |
| Recent failures/audit events | Daily when something breaks | Attention queue | Logs |
| Add client | Occasional | Guided detect/confirm flow | Client drawer |
| Import server/profile | Occasional | Drag/drop or URL import popover | Server install wizard |
| Profile creation | Occasional | Clone/default profile quick path | Profile drawer |
| Inspector | Rare/debug | Health-check shortcut only | Server detail, Inspector drawer |
| Raw capability JSON | Rare/debug | Hidden | Capability list, Settings Developer |
| Transport parse rules | Rare/setup/debug | Hidden | Client drawer |
| Runtime install/reset/cache | Rare/admin | Hidden or explicit advanced action | Runtime |
| API Docs | Rare/developer | Hidden from compact default | API Docs |

## User Attention Model

Compact should answer four questions in this order:

1. Am I ready?
   Core health, active profile, connected server/client state.
2. What needs my attention?
   Pending clients, failed server imports, disconnected servers, recent errors.
3. What changed or cost something?
   Traffic, token savings, runtime/audit trend.
4. What can I do now?
   Add client, import server, switch profile, open Full Console.

The compact UI should avoid asking users to understand protocol internals before
they know whether MCPMate is healthy.

## Candidate Interaction Model

### Default Surface

A narrow status-first surface with:

- Core status.
- Active profile.
- Clients count and pending count.
- Servers count and connected count.
- Traffic/token summary.
- One primary action cluster.

This can appear as a mini panel, tray-rich panel, or a compact Board entry route.

### Reveal-on-Touch Actions

Each object can expand into a local popover:

- Clients: detected count, pending approvals, Add Client, Detect again.
- Servers: import target, drag/drop zone, recently imported servers, health.
- Profiles: current profile, quick switch, clone/create from default.
- Runtime: ready status and short diagnostics, with Full Runtime link.
- Logs: recent error count and latest events, with Full Logs link.

### Full Console Escape Hatch

Every compact popover should have a clear Full Console link. The compact surface
should not hide the existence of advanced controls; it should delay them until
the user asks for depth.

## Design Boundaries

1. Existing Board remains the Full Console.
2. Compact is a guided operating layer, not a permission boundary.
3. Capability Grid remains a later architecture track.
4. Do not scatter broad `appMode` conditionals through current pages during
   research.
5. Do not duplicate deep forms. Prefer wrapping proven API flows with guided
   defaults, then link to existing drawers/pages for precision editing.
6. Do not mix Admin catalog, desktop shell, backend API, and Board into one
   concept. The compact surface operates the local MCPMate core through the
   existing Board/API boundary.

## Recommended Next Step

Move from research into a design spec with three visual directions:

1. Tray Rich Panel
   Smallest daily-use surface. Best for "MCPMate stays out of the way."
2. Compact Board Home
   Replaces Dashboard first impression with a denser operator panel. Best for
   web/desktop parity.
3. Hybrid Mini Panel + Full Console
   A persistent compact strip plus object popovers, with the existing Board as
   the expanded control plane. Best fit for the current product direction.

Recommended direction: Hybrid Mini Panel + Full Console.

Reason: it preserves the current Board, uses existing operational state, avoids
rewriting complex flows, and matches the user's mini-mode/tray-rich-menu
interaction preference.
