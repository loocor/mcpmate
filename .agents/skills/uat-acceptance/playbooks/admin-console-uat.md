# Admin Console UAT Playbook

This is the first acceptance playbook for MCPMate Admin Console or admin-like management surfaces. Use Board as the UI/UX taste reference, but verify actual routes, API names, and seed scripts from the active implementation before running the playbook.

## Scope

The playbook covers:

- Local admin login
- Account maintenance
- Portal, client, and server editor
- Publish
- Public discovery endpoint
- Rollback
- Audit review

## Preconditions

Actor:

- Operations Admin with local administrative access.

Environment:

- Local backend/API reachable.
- Admin Console or admin-like UI reachable.
- Board/Tauri reference available if UI conformance is in scope.
- Browser automation or Playwright available for repeatable path checks.

Seed:

- One admin account or local admin session.
- One non-admin or invalid session.
- One portal/registry/discovery configuration.
- One client record in each relevant governance state: allowed, pending, denied/suspended.
- One server draft/import candidate.
- One published server or portal record.
- One rollback target.
- Audit table initially empty or with a known fixture set.

Reset:

- Declare whether the run uses a fresh local database, fixture namespace, test account, or reversible API setup.
- Do not run publish/rollback against production or public systems unless the Project item explicitly approves that environment.

## Path 1: Local Admin Login

Actor: Operations Admin

Entry point:

- Admin Console login route or account/session dialog.

Steps:

1. Open the Admin Console.
2. Attempt access without an active session.
3. Log in with local admin credentials or complete local admin bootstrap.
4. Reload the page.
5. Log out or expire the session if the implementation supports it.

Expected:

- Unauthenticated access is blocked or redirected with clear state.
- Login form has labels, visible error state, and no placeholder-only required fields.
- Successful login lands on the intended operational surface.
- Header/sidebar/session affordance clearly shows account/admin state.
- Failed login shows the backend error directly enough to diagnose.
- Audit records login success/failure when the system owns that event.

Conformance checks:

- Dialog is acceptable for compact login; drawer is acceptable only if login includes multi-section setup.
- No marketing-style landing page is inserted before the operational surface.
- No silent fallback to guest/admin mode.

Severity:

- P0 if unauthorized users can access admin functionality.
- P1 if admin cannot log in or session state is misleading.
- P2 if the path works but lacks audit, labels, or clear errors.

## Path 2: Account Maintenance

Actor: Operations Admin

Steps:

1. Open account/user management.
2. View admin profile/session details.
3. Create, edit, suspend, or delete a test account as supported.
4. Try an invalid edit.
5. Confirm audit records.

Expected:

- Account list is searchable/filterable when it can contain many records.
- Status uses badges, not only text.
- Destructive actions use AlertDialog/ConfirmDialog with destructive styling.
- Editing a complex account uses a drawer; short focused edits may use a dialog.
- Suspended/deleted accounts cannot continue privileged actions after refresh.
- Audit captures actor, target, action, status, and timestamp.

Non-conforming:

- Account actions hidden in unlabelled icon clusters.
- Delete/suspend without confirmation.
- Permission changes that appear successful before backend confirmation without pending state.

## Path 3: Portal Editor

Actor: Operations Admin

Steps:

1. Open portal/registry/discovery settings.
2. Create or edit a portal record.
3. Validate URL or registry metadata fields.
4. Disable and re-enable the portal.
5. Confirm list/detail state and audit records.

Expected:

- Portal records use list/table/card depending on density:
  - table for many comparable records,
  - card/list for a small number with descriptive metadata.
- Editor groups identity, endpoint, credentials/secrets, and publication/discovery settings.
- Sensitive fields are redacted after save.
- Validation failures are inline and do not silently rewrite fields.
- Toggle enablement uses Switch only for immediate reversible state.

Public endpoint relation:

- Portal changes that affect public discovery must be reflected in the public discovery endpoint path below.

## Path 4: Client Editor

Actor: Operations Admin

Steps:

1. Open client management.
2. Review detected, allowed, pending, and denied/suspended clients.
3. Approve or suspend a client.
4. Edit client metadata or configuration when supported.
5. Refresh and verify stable ordering.

Expected:

- Client ordering is stable by display name then identifier unless the UI clearly exposes another sort.
- Governance state is visible as badge/status and switch state.
- Pending clients cannot be treated as allowed without explicit approval.
- Writable/non-writable config is visible.
- Refresh does not reorder unchanged records unexpectedly.
- Audit records governance changes.

Conformance checks:

- Uses Board-like client list/card affordances.
- Switch is acceptable for allowed/suspended if operation is immediate and reversible.
- Confirmation is required if the change can break a client or remove configuration.

## Path 5: Server Editor

Actor: Operations Admin

Steps:

1. Open server management.
2. Create a stdio or HTTP server draft.
3. Edit server identity, transport, auth, default headers, and capability-related fields.
4. Trigger preview/discovery if supported.
5. Save and refresh.

Expected:

- Long server editor uses drawer/full-height side panel, not cramped modal.
- Transport-specific fields appear only when relevant and preserve valid existing values.
- OAuth/headers/secrets are clearly labelled and redacted where appropriate.
- Capability discovery has loading, success, failure, and empty states.
- Saved server appears with status badge and correct enabled/runtime state.
- Audit records create/update/discovery events.

Non-conforming:

- Hidden default values that publish or enable a server without explicit choice.
- No visible distinction between draft, saved, enabled, connected, failed, and discovered states.

## Path 6: Publish

Actor: Operations Admin

Steps:

1. Open a draft portal/server/public listing candidate.
2. Review all publish-critical fields.
3. Attempt publish with missing required fields.
4. Publish a valid candidate.
5. Refresh list/detail and query public discovery endpoint.
6. Review audit.

Expected:

- Publish is a primary action only in the review context.
- Missing fields block publish with specific errors.
- Valid publish has pending/loading state and clear success/failure notification.
- Published state is visible in badge/metadata.
- Public discovery endpoint returns the published item after refresh.
- Audit records actor, target, published state, and status.

Severity:

- P0 if publish exposes wrong data or bypasses authorization.
- P1 if publish succeeds but public discovery/audit state is wrong.
- P2 if publish works but UI hierarchy or error copy is weak.

## Path 7: Public Discovery Endpoint

Actor: External User or Operations Admin

Steps:

1. Identify the public discovery endpoint from the active implementation.
2. Query empty/no-record state.
3. Publish a record and query again.
4. Query invalid parameters or unsupported filters.
5. Disable/rollback the record and query again.

Expected:

- Response shape is documented or inspectable.
- Empty state is explicit and not confused with server error.
- Published item appears with correct metadata and no secret leakage.
- Invalid request returns an appropriate error response.
- Disabled/rolled-back item no longer appears or is clearly marked according to the contract.

Automation:

- This path should be scriptable with API/client tooling once the endpoint is implemented.

## Path 8: Rollback

Actor: Operations Admin

Steps:

1. Open a published record with previous version/history.
2. Inspect current and rollback target.
3. Trigger rollback.
4. Confirm destructive/risky action.
5. Verify UI, API/public endpoint, and audit state.

Expected:

- Rollback target is explicit.
- Confirmation copy names the target and consequence.
- Rollback has pending, success, and failure states.
- UI and endpoint reflect the rolled-back version after refresh.
- Audit records rollback actor, source version, target version, and result.

Non-conforming:

- Rollback button beside unrelated row action without confirmation.
- Rollback that succeeds in UI but leaves public endpoint stale.
- Rollback that cannot be tied to an audit event.

## Path 9: Audit Review

Actor: Operations Admin or Reviewer

Steps:

1. Open audit/logs.
2. Filter by category, status, target, account, client, server, or publish/rollback action.
3. Expand a row.
4. Open raw details.
5. Refresh and paginate.
6. Verify live/disconnected state if WebSocket/live stream exists.

Expected:

- Audit table follows Board density: sticky header, fixed widths, truncation, row expansion, pagination.
- Empty/no-match states are distinct.
- Status uses semantic badges.
- Raw details open in drawer or clear detail panel.
- Long target/route/session values do not break layout.
- Refresh and pagination preserve user context where reasonable.

Conformance checks:

- Audit is not a decorative timeline when table density is needed.
- Raw detail view redacts sensitive values.
- Failed events are not hidden by default without a visible filter state.

## Final Acceptance Gate

Admin Console is not ready for acceptance when any of these remain:

- Unauthenticated admin access or privilege confusion.
- Publish/rollback/account changes without audit evidence.
- Missing seed/reset plan for state-changing tests.
- Public discovery exposes secrets or stale/wrong records.
- Major workflows use a design system that diverges from Board.
- P0 or P1 findings remain unresolved.

Admin Console may be marked partially conforming when:

- Core paths complete, but Playwright coverage, screenshot evidence, or Project feedback is incomplete.
- Visual hierarchy is close to Board but has P2 density/control issues.
- Manual product judgment is needed for copy, risk framing, or component choice.
