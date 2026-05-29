# Tray Operator Panel Design Spec

Date: 2026-05-29
Project item: `v0.5: Compact Operator Panel design research`
Branch: `feat/compact-operator-panel`

## Status

This is the final design direction for the compact operator workstream after
product, interaction, and desktop feasibility review.

The selected model is:

> MCPMate runs quietly in the system tray. The tray opens a compact operator
> panel for everyday status, attention, and high-frequency actions. The existing
> Board remains the Full Board for complete configuration, management,
> inspection, and troubleshooting.

This replaces the earlier `Express / Expert` and `Full / Compact` mode framing.
Those labels are not the user-facing product model for this feature.

## Product Model

### Full Board

Full Board is the existing Board application. Its Dashboard, Sidebar, routes,
and page-level information architecture remain unchanged.

Full Board owns:

- Client configuration parsing, config path/container fields, transport rules,
  and advanced client editing.
- Server Inspector, raw capability JSON, request/response/event streams, and
  schema-level debugging.
- Profile capability editing, server/tool/resource/prompt filters, and bulk
  enable/disable.
- Runtime install, reset, cache, diagnostics, API Docs, developer settings,
  complete Logs, complete Audit, and deep troubleshooting.

### Tray Operator Panel

Tray Operator Panel is a system-tray utility surface, not a Dashboard variant
and not a second full application.

It answers four questions:

1. `Am I ready?`
   Core status, uptime, active profile, and readiness.
2. `What needs attention?`
   Pending clients, unhealthy servers, recent failures, and attention count.
3. `What changed or cost something?`
   Traffic, token/runtime indicators, and recent audit summaries.
4. `What can I do now?`
   Quick approve, quick profile switch, server import entry, and Open Full
   Board.

The panel must not expose deep configuration by default. Deep actions route to
Full Board.

## First-Run Behavior

Onboarding remains the first-run setup path. The panel does not duplicate the
onboarding wizard.

Required behavior:

- Before onboarding is complete, tray activation opens or focuses the setup path
  in Full Board.
- After onboarding completes for the first time, MCPMate opens Tray Operator
  Panel once to establish the tray-first mental model.
- Later launches stay quiet in the system tray until the user opens the panel.
- The panel always includes an obvious `Open Full Board` escape hatch.

Application routing priority remains:

1. Backend/service loading or startup gate.
2. Onboarding/setup gate.
3. Full Board or Tray Operator Panel surface.

This prevents Full Board from flashing before onboarding or service readiness.

## Panel Anatomy

The panel uses the vertical Direction A structure, adapted for desktop tray
utility use.

### Header

Header height should be approximately `40-44px`.

Header contents:

- MCPMate glyph or compact brand mark.
- Core readiness dot.
- Short title, such as `MCPMate`.
- Icon buttons for `Pin`, `Detach/Attach`, `Open Full Board`, and `Close`.

The empty header area is the drag zone. Header controls must not trigger drag.
All icon controls require accessible labels, keyboard focus, pressed/selected
state where applicable, and tooltip text.

### Readiness Band

The readiness band is a compact status row, not a hero and not a card.

It summarizes:

- Core state: ready, degraded, starting, stopped, or unreachable.
- Active profile.
- Attention count.

The first glance should answer whether MCPMate is ready.

### Operator Rows

The core rows are:

- Core
- Profiles
- Clients
- Servers
- Traffic
- Attention

Each row uses this structure:

```text
[icon cell] [title + status] [summary/meta] [local affordance]
```

Guidelines:

- Icon cell: `32-36px`.
- Row height: `56-64px`.
- Row height must not shrink below readable title and summary.
- Status appears as dot, badge, or short text, not a large color block.
- Each row can expose one primary local action.
- Deep operations route to Full Board.

### Detail Behavior

Fixed-width tray panels must not use a right-side detail region.

Allowed detail patterns:

- Inline row expansion below the selected row.
- Anchored popover from the row action.
- Small bounded detail block for recent state and next action.

Detail content may include:

- Pending client count and a quick review action.
- Unhealthy server count and a server owner link.
- A small traffic sparkline or current token/runtime metric.
- Latest audit/error snippet.

Detail content must not include raw JSON, Inspector forms, transport parsing,
bulk capability editing, runtime reset, or API Docs.

### Footer

`Open Full Board` must remain reachable.

At normal height, the footer can contain a text button. At minimum height, the
footer may collapse while the header icon remains available.

## Window Behavior

Tray Operator Panel should be implemented as a dedicated Tauri `operator`
`WebviewWindow` that renders a dedicated Board `/operator` surface outside the
normal Board layout.

The panel must not resize or mutate the `main` Full Board window.

Recommended window constraints:

- Fixed width: default around `400px`; acceptable design range `380-420px`.
- Resizable height.
- Minimum height around `420px`; the exact value must preserve the required
  header, readiness, and operator row minimums.
- Maximum height should be bounded by the current monitor work area.
- Horizontal resizing disabled by setting min and max width to the same value.

### Attached State

Attached state is opened from the tray and anchored near the tray area where
the platform permits it.

Behavior:

- Tray activation toggles the panel on supported platforms.
- External click, `Esc`, or tray activation closes the panel when it is not
  pinned.
- The panel does not remember arbitrary detached coordinates while attached.

### Detached State

Detached state turns the panel into a small utility window.

Behavior:

- Width remains fixed.
- Height remains resizable.
- The user can move the window by the header drag zone.
- `Attach` returns it to tray behavior.

Detach is useful but not required to block the first implementation if the
dedicated tray panel and pin path are already shippable.

### Pin State

Pin means `keep open / always on top`.

Behavior:

- Pin has an explicit pressed state.
- Pinned panels do not close on outside click.
- Detached pinned panels use always-on-top behavior.
- Attached pinned panels still provide an explicit close control.

Pin is not a favorite/bookmark concept.

## Tray Integration

Native tray menus cannot host the rich React operator UI. The tray menu remains
for command fallback and platform compatibility.

Recommended desktop behavior:

- Left-click tray icon: toggle Tray Operator Panel on platforms that support
  tray click events.
- Right-click tray icon: show native tray menu.
- Native menu includes `Open Full Board`, `Open Operator Panel`, service
  actions, settings/about where applicable, and quit.
- Linux may need to rely on the native menu path because tray icon mouse events
  are not reliable across environments.

## Visual System

The panel should feel like a desktop tray utility, not a stretched phone screen.

Required visual qualities:

- Dense, calm, operational layout.
- Hairline borders, hover/focus states, compact icon buttons, and short labels.
- Existing Board tokens and component taste.
- Semantic colors for status: ready/success, warning, critical, muted.
- Lucide-compatible icon style where it matches the Board system.

Avoid:

- Large rounded mobile cards.
- Bottom navigation.
- Swipe-only gestures.
- Full-screen modal language.
- Marketing copy.
- Large decorative backgrounds or one-note color themes.

## Content Priority and Compression

Vertical resize changes secondary visibility, not the core control structure.

Priority order:

1. Header.
2. Readiness band.
3. Core, Profiles, Clients, Servers, Traffic, Attention row minimums.
4. Selected row detail.
5. Recent events, sparkline, hints, and secondary copy.

Compression rules:

- Header, readiness, and row minimums remain stable.
- Details, charts, recent event lists, and helper text collapse first.
- Rows may scroll as a group when height is tight.
- Text must stay readable; do not reduce core UI text below usable desktop
  control sizes.

## Implementation Boundaries

MUST:

- Preserve existing Full Board and Dashboard information architecture.
- Use a dedicated `/operator` surface for the tray panel.
- Keep the panel outside the normal `Layout` Sidebar/Header shell.
- Keep all user-facing text i18n-ready.
- Provide loading, starting, degraded, offline, empty, and error states.
- Provide a clear `Open Full Board` path.

MUST NOT:

- Implement this as `Express / Expert` page branching.
- Embed the panel into Dashboard.
- Use a right-side detail panel in the fixed-width tray surface.
- Expose raw JSON, Inspector forms, transport parsing, runtime reset, API Docs,
  or capability bulk editing inside the tray panel.
- Add fallback behavior that silently replaces unavailable desktop capability.

SHOULD:

- Reuse data hooks and API boundaries from the existing Board where practical.
- Keep one primary action per operator row.
- Route deep actions to the owning Full Board page.
- Prefer narrow desktop commands for window behavior over broad JS window
  permission exposure.

## Technical Architecture Recommendation

Board:

- Add a dedicated `/operator` route.
- Render the operator surface outside the normal Board layout.
- Reuse the compact status/data composition where useful, but remove Dashboard
  `appMode` ownership.
- Update tests to cover `/operator` as its own surface.

Desktop:

- Add an `operator` window label.
- Lazy-create the operator `WebviewWindow` on first tray activation.
- Reuse show/hide/focus lifecycle patterns parallel to the existing `main`
  window handling.
- Add a narrow command for `Open Full Board`.
- Add a narrow command for pinning the operator window if JS window permissions
  are too broad.
- Add Tauri capability entries for the `operator` window.

Platform caveats:

- macOS and Windows can target tray click toggling.
- Linux acceptance should include the native menu path for opening the panel.
- Fixed-width vertical resize must be validated per platform/window manager.
- A Tauri `WebviewWindow` is not the same as a native macOS `NSPanel`; do not
  promise non-activating panel behavior without a later native extension.

## Acceptance Criteria

Product:

- Users no longer see `Express / Expert` as the model for this feature.
- Full Board remains unchanged as the complete console.
- Tray Operator Panel provides immediate status, attention, and high-frequency
  action access.
- First-run behavior introduces the tray panel once after onboarding.

Interaction:

- Tray activation opens a fixed-width panel.
- The panel can be resized vertically without breaking row readability.
- `Open Full Board` opens/focuses the main Board window instead of navigating
  the operator window into Full Board.
- Pin state is visible and changes close/always-on-top behavior.
- Keyboard access works for header controls and primary row actions.

Visual:

- Core rows remain visible and scan-friendly at the minimum accepted height.
- Secondary content collapses or scrolls before primary rows shrink.
- The panel reads as a desktop utility, not a mobile app embedded on desktop.
- Long localized strings do not overflow critical controls.

Technical:

- Board lint and build pass for the `/operator` route work.
- Desktop `cargo check` passes after adding the operator window.
- macOS and Windows tray click behavior is verified.
- Linux native menu fallback path is verified or explicitly scoped.

## Phasing

### Phase 1: Spec-Correct Foundation

- Remove Dashboard `appMode` ownership from the operator prototype.
- Add `/operator` as a dedicated Board surface.
- Add desktop `operator` window shell and tray toggle.
- Implement six operator rows, readiness band, Open Full Board, and basic
  loading/error/empty states.
- Validate Board build/lint and desktop build/check.

### Phase 2: Utility Behavior

- Add pin behavior.
- Add vertical resize constraints and platform validation.
- Add first-run post-onboarding panel reveal.
- Add focused Playwright and Tauri manual acceptance paths.

### Phase 3: Advanced Panel Refinement

- Add detach/attach behavior if platform behavior is reliable.
- Add row-level popovers or inline expansions.
- Add direct deep links from row actions to owning Full Board pages.
- Evaluate guided media or help content after the panel's core behavior is
  stable.

## Open Questions

- Exact default width should be selected after the first visual pass; start from
  `400px`.
- Whether detach ships in Phase 1 or Phase 3 depends on Tauri behavior during
  real desktop validation.
- Whether first-run auto-open should happen immediately after onboarding or on
  the next tray-ready tick should be decided during implementation, based on
  lifecycle reliability.
