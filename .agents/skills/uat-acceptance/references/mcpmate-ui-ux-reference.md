# MCPMate UI/UX Reference

This reference is derived from the current Board implementation, which is the web management interface and the UI loaded by the Tauri shell. Use it as the taste baseline for MCPMate operational surfaces.

## Evidence Sample

The first version was sampled from:

- `board/src/index.css`
- `board/tailwind.config.js`
- `board/src/App.tsx`
- `board/src/components/layout/layout.tsx`
- `board/src/components/layout/sidebar.tsx`
- `board/src/components/layout/header.tsx`
- `board/src/components/page-layout.tsx`
- `board/src/components/ui/button.tsx`
- `board/src/components/ui/card.tsx`
- `board/src/components/ui/badge.tsx`
- `board/src/components/ui/tabs.tsx`
- `board/src/components/ui/segment.tsx`
- `board/src/components/ui/select.tsx`
- `board/src/components/ui/dialog.tsx`
- `board/src/components/ui/alert-dialog.tsx`
- `board/src/components/ui/drawer.tsx`
- `board/src/components/entity-card.tsx`
- `board/src/components/status-badge.tsx`
- `board/src/components/audit-logs-panel.tsx`
- `board/src/pages/dashboard/dashboard-page.tsx`
- `board/src/pages/servers/server-list-page.tsx`
- `board/src/pages/servers/server-detail-page.tsx`
- `board/src/pages/clients/clients-page.tsx`
- `board/src/pages/audit/audit-page.tsx`

## Product Taste

MCPMate should feel like a compact control plane for repeated operational work. The interface should be quiet, structured, and information-dense. It should not feel like a landing page, marketing site, showcase dashboard, or decorative SaaS mockup.

Conforming:

- Uses a persistent left sidebar plus fixed top header for navigation and context.
- Keeps page content inside a viewport-height shell with internal scrolling.
- Uses compact descriptions, tables, cards, badges, and toolbars to support scanning.
- Makes state and ownership explicit: enabled/disabled, connected/disconnected, pending/allowed/denied, live/disconnected, success/failed/cancelled.
- Uses icon buttons for compact tools and text buttons for clear commands.
- Keeps data surfaces stable under refresh, toggles, filters, pagination, and language changes.

Non-conforming:

- Large hero sections, oversized marketing copy, decorative illustrations, or empty whitespace as the primary screen.
- One-off palettes, gradients, oversized shadows, novelty controls, or unframed interaction patterns that do not match Board.
- Hidden primary state, ambiguous destructive actions, missing loading/error/empty states, or workflow completion without audit visibility.
- UI text that is hardcoded in Board-like React surfaces instead of using page translations and `defaultValue`.

## Layout Density

Board layout is dense but not cramped.

- Shell: fixed sidebar (`w-64` expanded, `w-16` collapsed), fixed header (`h-16`), page padding `p-4`.
- Page body: `space-y-4`, `gap-4`, cards and tables in a single scrollable content column.
- Page header: compact one-line description on the left, toolbar/actions on the right.
- Cards: generally `p-4`, `pb-2`, `pt-0`, or `pt-2`; card grids use `gap-4`.
- Tables: `text-sm`, `py-2` header cells, `py-3` body cells, sticky headers for scrollable audit tables.
- Toolbars: controls generally `h-9`; icon actions generally `h-9 w-9`.

Conforming:

- Use compact toolbar rows with search, filters, sort, refresh, and primary add/publish action.
- Use internal scroll areas for long tables and detail tabs rather than growing the whole shell.
- Use `min-w-0`, `truncate`, `whitespace-nowrap`, fixed colgroups, or stable widths where data can overflow.

Non-conforming:

- Page-level panels that float inside decorative wrappers.
- Cards nested inside cards for layout decoration.
- Controls that jump in size when loading, filtering, expanding rows, switching language, or changing route.
- Dense data rendered as a sparse card gallery when a table or list is the clearer operational form.

## Color

Board uses shadcn-style HSL tokens with slate neutrals and a small set of semantic accents.

- Light background: white / slate-50 style surfaces.
- Dark background: slate-like dark tokens, with subtle borders close to card luminance.
- Primary: near-slate/foreground token, not a brand-gradient.
- Semantic accents:
  - emerald for success/ready/allowed
  - amber for warning/pending
  - red/destructive for failed/denied/delete
  - sky/info sparingly
- Borders are subtle hairlines. Dark borders should not read as bright rails.

Conforming:

- Use theme tokens (`background`, `foreground`, `card`, `muted`, `border`, `primary`, `destructive`) where possible.
- Use semantic badge colors to encode state.
- Keep hover states restrained: accent background, subtle border, or small shadow.

Non-conforming:

- Purple/blue gradient dominance, one-note accent palettes, bright saturated backgrounds, or decorative color fields.
- Destructive actions styled like normal primary actions.
- State colors that conflict with Board semantics.

## Typography

Board typography is utilitarian and compact.

- Header route titles: about `text-xl font-semibold`.
- Page description: `text-base text-muted-foreground`, often truncated.
- Card titles: default `text-2xl` in primitive, often overridden to `text-lg` or `text-sm`.
- Entity titles: `text-lg font-semibold leading-tight`.
- Body, controls, and table text: `text-sm`.
- Dense metadata: `text-xs`, uppercase labels, mono text for protocol/version/config values.
- Tiny repeated stats can use `text-[9px] uppercase tracking-wide`.

Conforming:

- Reserve large headings for route/header context. Keep card, drawer, and table headings tighter.
- Use tabular numbers or mono text for durations, versions, protocols, IDs, paths, and code-like values.
- Clamp or truncate long entity names, targets, routes, descriptions, and file paths.

Non-conforming:

- Hero-scale typography inside cards, drawers, dialogs, toolbars, or tables.
- Text that overlaps controls or relies on viewport-width font scaling.
- Uncontrolled wrapping that pushes critical actions offscreen.

## Radius, Borders, Shadow

The system uses moderate radius and clear but quiet boundaries.

- Base radius token: `0.5rem`.
- Buttons, selects, inputs, tabs, and toolbar controls: `rounded-md` or `rounded-sm`.
- Cards currently use `rounded-xl` with `border border-border bg-card shadow-sm`.
- Entity cards may add hover shadow and slight upward motion.
- Dialogs and drawers use borders plus shadow, not heavy decoration.

Conforming:

- Use borders and small shadows to separate operational surfaces.
- Use hover shadow on clickable entity cards only when it reinforces actionability.
- Use dashed borders for empty/loading placeholders.

Non-conforming:

- Large pill cards, excessive shadows, glass effects, bokeh/orbs, or decorative wrappers.
- Borderless controls in dense forms where affordance becomes unclear.

## Icons

Board uses `lucide-react`.

Conforming:

- Use lucide icons for navigation, toolbar actions, status affordances, and compact commands.
- Use `h-4 w-4` for toolbar/action icons and about `20px` icons in sidebar links.
- Add tooltips or accessible labels when the icon-only action is not self-explanatory.
- Use icon-only buttons for compact tools such as refresh, inspect, theme, docs, feedback, expand row, and grid/list toggle.

Non-conforming:

- Custom SVG icons when a suitable lucide icon exists.
- Icon-only destructive actions without clear tooltip, confirmation, or local context.
- Decorative icons that do not clarify state or action.

## Information Hierarchy

Board gives each page a short route title in the fixed header, a compact page description, and then operational content.

Conforming:

- Place page-level actions in the right side of the page header or toolbar.
- Place entity-specific actions on the entity card/list row/detail header.
- Show operational summary cards only when they improve scanning or monitoring.
- Use badges near the entity title or relevant metadata, not as decoration.
- Use detail tabs for large peer sections and nested tabs only when the content belongs to a stable detail context.

Non-conforming:

- Repeating page titles in multiple places when the fixed header already establishes route context.
- Moving primary actions into hidden menus without space pressure or risk rationale.
- Mixing global actions and row actions in one unlabelled cluster.

## Component Selection Rules

### Tabs

Use tabs for stable peer sections inside a detail page or dense workflow context. Board uses tabs for server capability sections and nested capability types.

Conforming:

- Use tabs when content can be lazy-loaded and users need to switch among persistent sections.
- Keep tab labels short. Pair with URL state when deep-linking matters.

Non-conforming:

- Tabs used as a replacement for a binary toggle, one-off filter, or wizard progress.

### Segment

Use segment controls for small mutually exclusive modes that affect the same local surface.

Conforming:

- Mode selection such as view/transport/channel/state when the options are few and visible comparison helps.
- Tooltip segment options when labels alone are ambiguous.

Non-conforming:

- Segment controls for long option lists, sparse settings, or destructive decisions.

### Select

Use select for compact option sets where only the selected value matters.

Conforming:

- Sort field, category filter, status filter, page size, server type, language, or profile selection.
- Keep trigger height consistent with toolbar/form sizing (`h-9` in toolbar, `h-10` in forms).

Non-conforming:

- Select for two obvious binary states where switch/checkbox is clearer.
- Select for large searchable data sets; use combobox/search instead.

### Radio

Use radio groups for visible exclusive choices when users must compare labels/descriptions before choosing.

Conforming:

- Deployment mode, exposure policy, or plan choice where each option needs explanation.

Non-conforming:

- Radio groups in cramped table rows or toolbars.

### Checkbox

Use checkbox for multi-select or independent inclusion choices.

Conforming:

- Selecting multiple clients, servers, capabilities, or rows.
- Show indeterminate state for select-all when only some items are selected.

Non-conforming:

- Checkbox for global enablement of a single entity when a switch communicates state better.

### Switch

Use switch for immediate binary enablement that changes live state or governance state.

Conforming:

- Server enabled/disabled, client allowed/suspended, feature toggles.
- Disable the switch or show pending state when an operation is in progress or cannot be changed.

Non-conforming:

- Switch for one-time destructive actions, irreversible choices, or settings that require multiple related fields.

### Dialog

Use dialog for short, focused forms or decisions.

Conforming:

- Small add/edit forms, account/session state, focused confirmation, or compact setup.
- Title, description, cancel, submit, loading, and error state are visible.

Non-conforming:

- Long editors, multi-section workflows, or tables inside a small centered dialog.

### Drawer / Sheet

Board uses right-side Vaul drawers for larger editors and inspectors. Prefer this pattern for MCPMate operational editing.

Conforming:

- Server/client/profile editors, import review, inspector output, audit raw event details.
- Right-side width around `sm:w-[560px]` to `md:w-[720px]`, full-height, scrollable, with clear header/footer.

Non-conforming:

- Center modal for a large editor that forces cramped scrolling.
- Drawer without a clear close path, dirty-state handling, or submit/cancel affordance.

### Table

Use tables for audit, logs, registry rows, matrix-like capabilities, and dense comparable data.

Conforming:

- Fixed column widths, sticky header for scrollable regions, row expansion for details, pagination for long lists.
- Truncate long target/path fields and provide detail drawer or expansion.

Non-conforming:

- Cards for log streams or dense event history.
- Table columns that resize unpredictably when row expansion toggles.

### Card

Use cards for entity summaries, stats, and bounded repeated items.

Conforming:

- EntityCard pattern: avatar/title/description, top-right badge, small stats, bottom tags/actions.
- Stats cards with title, value, short description.
- Empty/loading states in a single card or dashed placeholder.

Non-conforming:

- Cards as generic page section wrappers around other cards.
- Decorative cards that do not represent an entity, metric, or bounded task.

## Forms

Conforming:

- Use React Hook Form + Zod in Board-style surfaces when validation is needed.
- Use labels, descriptions, error messages, and stable IDs.
- Group related fields with `space-y-4`, `gap-4`, and compact two-column layouts when space supports it.
- Use drawers for long editors and dialogs only for short forms.
- Surface backend/API errors directly instead of substituting hidden fallback behavior.
- Keep submit/cancel actions in the footer and show loading spinners for pending submit.

Non-conforming:

- Unlabelled inputs, placeholder-only forms, hidden validation failures, or submit buttons far from the edited data.
- Hardcoded user-facing text in Board-like React surfaces.
- Silent defaults that create or publish data the user did not explicitly select.

## Toolbars And Main Actions

Conforming:

- Search left/flexible, filters/sort/view mode/actions right.
- Refresh and inspect are icon buttons with tooltips/titles.
- Primary add/publish action is the rightmost or most visually prominent toolbar action.
- Persist search/view/sort in URL when the page is list-heavy and deep-linking is useful.

Non-conforming:

- Multiple primary buttons with equal visual weight.
- A toolbar that wraps into unreadable controls on narrower screens.
- Primary action buried in a dropdown when there is enough space.

## Status, Empty State, Danger, Audit

Conforming:

- Status badges use semantic variants and small dots when status should be scanned quickly.
- Empty state explains whether there are no records or no matches after filters.
- Dangerous actions use AlertDialog or ConfirmDialog, destructive styling, and clear irreversible copy.
- Audit-sensitive flows expose reviewable event history, raw details, or a direct route to audit review.
- Loading uses skeletons or restrained placeholders that preserve layout.

Non-conforming:

- Delete/publish/rollback without confirmation or audit trace.
- Empty state that implies setup is complete when data is merely filtered out.
- Error state that only logs to console or hides the backend message.

## Page Structure

Conforming Board-like page:

1. Fixed shell provides route title, docs/feedback/theme/notifications, sidebar navigation.
2. Page content starts with compact description plus toolbar/actions.
3. Optional stats cards summarize operational state.
4. Main content is list/grid/table/detail surface with loading, empty, error, and pagination as needed.
5. Editors and raw details open in drawer/dialog without losing the list context.
6. Significant changes produce notifications and update audit/event surfaces.

Non-conforming page:

1. Hero or marketing header dominates the first screen.
2. Main action is separated from the relevant entity or hidden behind unrelated controls.
3. Long data has no search/filter/sort/pagination.
4. State changes have no visible feedback, no audit link, or no refresh/invalidation path.
