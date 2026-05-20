---
name: uat-acceptance
description: Use this skill for MCPMate UI/UX review, UAT planning, Playwright acceptance, Admin Console acceptance, Board acceptance, Extension delivery acceptance, or any request to judge whether a page, flow, component, PR, or deliverable conforms to MCPMate product taste and acceptance standards. Use it even when the user does not say "UAT" if the task involves visual quality, interaction quality, acceptance criteria, release readiness, or product-facing workflow validation.
---

# UAT Acceptance

Use this skill to judge MCPMate deliverables against the product's UI/UX reference and acceptance workflow. The goal is not generic design advice; it is a conforming/non-conforming decision backed by evidence from the current implementation, screenshots, tests, and the active GitHub Project item.

## Start Here

1. Confirm repository context before changing files or branch state:
   - `git rev-parse --show-toplevel`
   - `git remote get-url origin`
   - `git status --short --branch`
2. Identify the active GitHub Project item. If the user already names one, treat it as the delivery contract.
3. Identify the touched surface:
   - Board or Tauri-loaded Board UI
   - Admin Console or admin-like workflow
   - Extension UI or import flow
   - Website/marketing surface
   - Backend/API behavior that materially changes UI state or UAT paths
4. Read only the bundled resource needed for the task:
   - `references/mcpmate-ui-ux-reference.md` for UI/UX conformance, layout, component selection, visual taste, and interaction rules.
   - `references/mcpmate-uat-strategy.md` for UAT roles, environments, seed/reset, severity, automation boundaries, and GitHub Project feedback.
   - `playbooks/admin-console-uat.md` for Admin Console acceptance paths such as login, account maintenance, portal/client/server editor, publish, discovery, rollback, and audit review.

## Evidence Collection

Use real product evidence before judging a change.

- Read the current code for the affected surface and nearby established components.
- For Board-like UI, compare against these reference paths first:
  - `board/src/index.css`
  - `board/src/components/layout/`
  - `board/src/components/ui/`
  - `board/src/components/page-layout.tsx`
  - `board/src/components/ui/page-toolbar.tsx`
  - `board/src/components/entity-card.tsx`
  - `board/src/components/status-badge.tsx`
  - `board/src/pages/dashboard/`
  - `board/src/pages/servers/`
  - `board/src/pages/clients/`
  - `board/src/pages/audit/`
- For new UI, inspect the page in a browser or Playwright screenshot when the app can run locally.
- For UAT, record exact setup, seed data, actor, path, expected result, observed result, severity, and follow-up owner.

## Acceptance Output

When reviewing or validating, report in this shape:

```markdown
## Decision
Conforming | Partially conforming | Non-conforming | Blocked

## Evidence
- Surface:
- Project item:
- Code paths:
- Screenshots/tests:

## Findings
- [P0/P1/P2/P3] Finding title
  Evidence:
  Expected MCPMate standard:
  Required change:

## UAT Coverage
- Automated:
- Manual/agent judgment:
- Not covered:

## Project Feedback
- GitHub Project item update needed:
- PR/readiness note:
```

## Decision Rules

- Mark `Conforming` only when the implementation matches the relevant reference, the primary UAT path passes, and no P0/P1 issue remains.
- Mark `Partially conforming` when the workflow works but visual hierarchy, interaction model, accessibility, or feedback loop still has P2 gaps.
- Mark `Non-conforming` when the implementation invents a parallel design system, hides critical state, breaks the expected workflow, skips auditability, or makes destructive operations ambiguous.
- Mark `Blocked` when the environment, seed data, missing route, missing API, or missing Project contract prevents a defensible judgment.

## MCPMate-Specific Priorities

- Prefer dense operational layouts over marketing-style hero sections.
- Prefer reusable Board primitives and shadcn/Radix-compatible controls over custom one-off widgets.
- Prefer explicit operational state: status badge, audit trace, refresh/loading state, empty state, and destructive confirmation.
- Keep primary actions near the page or section toolbar. Keep row/card actions local to the entity they affect.
- Treat i18n, keyboard access, stable layout dimensions, and auditability as acceptance requirements, not polish.
- Do not add fallback behavior, silent substitution, compatibility shims, or hidden recovery paths unless the active Project item explicitly requires them.

## Skill Maintenance

When this skill is updated, keep `SKILL.md` as the trigger and workflow entrypoint. Move long examples, checklists, playbooks, and evidence matrices into bundled resources so future agents load only what they need.
