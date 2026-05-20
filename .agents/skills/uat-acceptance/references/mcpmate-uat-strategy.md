# MCPMate UAT Strategy

Use this strategy for acceptance planning and release-readiness review across MCPMate subprojects. It applies to Board, Tauri-loaded Board UI, Admin Console, Extension flows, website operational surfaces, and backend/API changes that affect user-facing workflows.

## UAT Principles

MCPMate UAT checks whether a real operator can complete the workflow safely, understand state, recover from visible errors, and leave an auditable trace. It is not only a browser smoke test.

Acceptance should answer:

- Can the intended actor complete the path without hidden assumptions?
- Is the UI consistent with MCPMate's compact operational style?
- Are state, risk, and ownership visible at the point of action?
- Does the system avoid silent fallback behavior unless explicitly required?
- Can a future reviewer find evidence in tests, screenshots, logs, audit events, or GitHub Project notes?

## Test Roles

Use the minimum set that covers the touched workflow.

- Product Owner: judges product intent, wording, flow order, risk framing, and whether the behavior matches the Project item.
- Operations Admin: manages accounts, clients, servers, profiles, registry/portal settings, publish/rollback, and audit review.
- Runtime Consumer: uses MCPMate through a client or agent and validates that UI changes do not break runtime expectations.
- Developer/Reviewer: validates implementation, test coverage, i18n, accessibility, and failure surfaces.
- External User: only when public discovery, marketing, website, or public endpoint behavior is in scope.

## Test Environments

Choose the smallest realistic environment.

- Local Web Board: Vite Board connected to local backend at `http://localhost:8080`.
- Desktop Shell: Tauri app loading Board when shell events, deep links, desktop readiness, or runtime source discovery matters.
- Extension: Chrome extension import/discovery flow when browser-based server import is involved.
- Admin Console: future/admin-like control surface for account, portal, client, server, publish, rollback, and audit tasks.
- Backend/API: direct API/Inspector checks when UI depends on endpoints, status, or event streams.

Record exact commands and versions. Use Bun for frontend package commands.

## Seed And Reset Preconditions

Define seed/reset before executing UAT. Do not rely on unknown local state.

Minimum seed dimensions:

- Account/session: unauthenticated, local admin, active admin, expired/invalid session.
- Clients: no clients, detected unmanaged client, allowed client, pending client, denied/suspended client, writable and non-writable config.
- Servers: no servers, stdio server, streamable HTTP server, OAuth server, disabled server, connected server, failed server, server with discovered capabilities.
- Profiles: empty profile, profile with server associations, profile with tools/resources/prompts/templates enabled.
- Registry/portal/discovery: no results, published result, draft/import preview, failed discovery, public endpoint response.
- Audit: no events, success event, failed event, rollback event, publish event, account/admin event.

Reset expectations:

- State-changing UAT must declare how the local database/config is reset or which fixture namespace is used.
- Avoid editing user home config directly unless the workflow under test is specifically about client config management.
- Prefer deterministic scripts or API setup when repeated UAT is expected.
- If reset cannot be made deterministic yet, mark the run `Blocked` or `Partially conforming` and create a Project follow-up.

## User Path Format

Write UAT paths in this format:

```markdown
### Path: [name]

Actor:
Environment:
Seed:
Entry point:

Steps:
1. ...
2. ...

Expected:
- UI:
- Data/API:
- Audit/log:
- Error/rollback:

Evidence:
- Screenshot:
- Test/log:
- Project item note:

Result: Pass | Partial | Fail | Blocked
Severity if failed: P0 | P1 | P2 | P3
```

## Acceptance Dimensions

### Workflow Completion

- Entry point is discoverable.
- Required data is present at the point of action.
- The user can finish the path without leaving the interface unexpectedly.
- Success, failure, pending, and retry states are visible.

### UI/UX Conformance

- Layout, density, controls, color, typography, spacing, and icon usage follow `mcpmate-ui-ux-reference.md`.
- Data surfaces have search/filter/sort/pagination when needed.
- Long values truncate or expand predictably.
- Actions stay near their object of control.

### Accessibility And Internationalization

- Interactive controls have labels or accessible names.
- Keyboard path works for toolbar, table expansion, dialog/drawer, and destructive confirmation.
- Board-like UI uses `t()` with `defaultValue` and page translations.
- Language changes do not leave memoized labels stale.

### Data And State Integrity

- UI state matches backend/API state after refresh.
- Mutations invalidate/refetch relevant queries or receive event updates.
- No hidden fallback creates misleading data.
- Optimistic or pending states are clear and reversible only when the underlying operation is reversible.

### Security, Risk, And Audit

- Destructive operations require explicit confirmation.
- Publish/rollback/account/admin actions are auditable.
- Sensitive values are redacted where appropriate.
- Permission/session boundaries are visible.

### Performance And Stability

- Initial route load, refresh, and pagination are responsive enough for the local environment.
- Large lists do not freeze the shell.
- Loading skeletons preserve layout and do not create jarring shifts.
- WebSocket/live streams degrade visibly when disconnected.

## Severity

- P0: Blocks the core workflow, corrupts or exposes sensitive data, bypasses authorization, or prevents safe rollback/publish/account control.
- P1: Major workflow cannot be completed by the target actor, critical state is misleading, destructive action is ambiguous, or audit evidence is missing for a required action.
- P2: Workflow completes but has significant UI/UX, accessibility, i18n, consistency, seed/reset, or evidence gaps that should be fixed before broad use.
- P3: Minor polish, copy, spacing, or documentation issue that does not block acceptance but should be tracked.

Use the highest severity that reflects user impact. Do not downgrade because a workaround exists unless the workaround is the documented intended path.

## Automation Boundary

Automate when the assertion is objective and repeatable:

- Route loads and renders expected controls.
- Search/filter/sort/pagination changes visible row/card sets.
- Dialog/drawer opens and closes.
- Form validation blocks invalid submit and shows error.
- Publish/rollback/API call creates expected response and audit event.
- Public discovery endpoint returns expected status/body/schema.
- Accessibility labels exist for icon-only controls.
- No console error occurs during a scripted path.

Use human or agent product judgment when the assertion is qualitative:

- Visual density feels aligned with Board.
- Information hierarchy is clear enough for an operations admin.
- Copy sets the right risk expectation.
- Component choice matches MCPMate taste.
- A new flow belongs in dialog, drawer, tab, table, card, or toolbar.
- A workflow should be accepted despite a lower-level limitation.

Hybrid checks should combine screenshot/video evidence with written judgment.

## GitHub Project Feedback Loop

The GitHub Project item is the canonical acceptance record.

For each UAT pass or review:

1. Link the relevant PR or branch.
2. Record environment, seed/reset, commands, screenshots, and path results.
3. Add findings with severity and owner.
4. Keep unresolved P0/P1 issues out of "done" status.
5. Convert P2/P3 items into follow-up Project items only when they are not required by the current acceptance contract.
6. Attach Playwright, lint/build, backend/API, Inspector, or manual evidence before reporting ready for Loocor/Copilot review.

## Recommended Report

```markdown
## UAT Decision
[Conforming | Partially conforming | Non-conforming | Blocked]

## Scope
- Project item:
- Surface:
- Actor:
- Environment:
- Seed/reset:

## Results
- Passed:
- Failed:
- Blocked:

## Findings
- [P?] ...

## Automation
- Ran:
- Missing:

## Project Feedback
- Updated:
- Still needed:
```

## Stop Conditions

Stop and ask for clarification when:

- The active Project item and requested acceptance scope conflict.
- The requested flow requires a migration, fallback, or compatibility layer not approved by the Project item.
- Seed/reset would alter user data or client config outside the declared test namespace.
- The UI path does not exist and the task is to judge implementation readiness rather than draft a playbook.
