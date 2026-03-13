# Repository Guidelines

## Collaboration Rhythm (Discuss → Build → Report)
- Day-to-day coordination with LLM/AI agents is done in **Chinese**.
- Source code, doc comments, documentation, and git commit messages stay in **English** for consistency across the repository.
- Before coding, analyze the context, requirements, and existing docs; propose your approach and proceed. Don't pause frequently unless information is missing or there's a major risk.
- Execute continuously during coding/testing; note key assumptions, trade-offs, and dependencies. Provide a single consolidated report on completion (results, decisions, validation, follow-ups).
- **i18n compliance**: All user-facing strings must be internationalized. Never hardcode UI text. See i18n guidelines below for critical requirements.

## Project Structure & Module Organization (this repo)
- Frontend app built with Vite + React 18 + TypeScript + Tailwind + shadcn/ui.
- Development proxy is configured in `vite.config.ts` to forward `/api` and `/ws` to `http://localhost:8080`.
- Market dev proxy: Vite registers middleware under `/market-proxy/*` for remote portals (mcpmarket.cn / mcp.so). On Tauri desktop builds this is re-implemented via a custom URI scheme `mcpmate://localhost/market-proxy/*` so the Market pages work without Vite.
- Key folders and files:
  - `src/App.tsx` – routes and top-level layout wiring.
  - `src/components/` – reusable UI (status badge, forms, layout, shadcn/ui wrappers).
  - `src/components/layout/` – header, sidebar, shell layout; theme handling.
  - `src/lib/api.ts` – centralized API layer mapping to backend (OpenAPI-aligned).
  - `src/lib/types.ts` – shared types for API data and UI models.
  - `src/pages/` – feature pages: `servers/`, `clients/`, `config/` (Profiles), `runtime/`, `settings/`, etc.
  - `docs/openapi.json` – frozen snapshot of upstream OpenAPI used to align endpoints/fields.

Conventions
- Use TanStack Query for data fetching/caching; isolate network calls in `src/lib/api.ts`.
- UI follows shadcn/ui + Radix primitives; prefer built-in components for consistency.
- Tailwind classes for layout/spacing; avoid inline styles unless necessary.
- Pages should be small and composable. Extract subcomponents once they exceed ~200–300 lines.

## Build, Test, and Development Commands
- Install deps: `bun install` (fallback: `npm install`).
- Dev server: `bun run dev` (Vite, ports 5173/5174 by default) with API proxied to `http://localhost:8080`.
- Production build: `bun run build` → output in `dist/`.
- Preview build: `bun run preview`.

Backend expectations
- Backend must run at `http://localhost:8080` exposing `/api` and `/ws`. The UI is developed against the OpenAPI in `docs/openapi.json` and the live server.

## Documentation Map (this repo)
- `docs/openapi.json` – source of truth for endpoints and shapes. When upstream updates, refresh this file and align `src/lib/api.ts`.
- `docs/guide-part*.md` – high-level design notes and API guidance; update when routes or modules meaningfully change.

## Execution Rhythm & Task Sizing
- Batch work into clear phases (analyze → implement → verify) and land small, incremental PRs.
- Keep UI changes scoped by page/feature; avoid large refactors that cut across modules unless approved.
- Record notable decisions and edge cases in PR descriptions; link to the OpenAPI sections you aligned.

## API & Protocol Alignment
- Treat `docs/openapi.json` as canonical; do not hardcode shapes ad-hoc in components. Extend `src/lib/types.ts` and `src/lib/api.ts` instead.
- WebSocket endpoint is `/ws` (proxied via Vite in dev; in desktop builds the UI talks directly to `ws://127.0.0.1:8080/ws`). Prefer event-driven refresh when available, otherwise use TanStack Query invalidation.
- Profiles: UI text uses “Profile” (formerly ConfigSuits). Existing filenames may retain legacy naming for compatibility; keep API calls aligned to `/api/mcp/profile/*`.

## UI/UX Guidelines
- Pages: Servers, Clients, Profiles (Config), Runtime, Settings.
- Use shadcn/ui components and Radix primitives; keep headers clean and compact. Place filters aligned right in card headers when space-constrained.
- Theme: respects OS `prefers-color-scheme` and user choice (`localStorage.mcp_theme` with values `light` | `dark` | `system`). Avoid FOUC by applying theme early in the layout/root.
- Clients: list ordering must be stable (by `display_name`, then `identifier`) and must not jump when toggling state.
- Server Detail: two primary tabs – Overview (configuration + instances) and Capabilities (with nested tabs Tools/Resources/Prompts/Resource Templates). Data loads lazily per tab.

## Internationalization (i18n) Guidelines

### Overview
The project uses `react-i18next` for internationalization with support for English (`en`), Simplified Chinese (`zh-CN`), and Japanese (`ja-JP`).

### Critical Requirements

1. **Translation files MUST use nested object structure**
   - Never use string keys with dots (e.g., `"status.ready"`)
   - Always use nested objects (e.g., `status: { ready: "Ready" }`)
   - i18next cannot properly resolve dot-notation string keys

2. **React hooks MUST include i18n.language in dependencies**
   - When using `useMemo`, `useCallback`, or `useEffect` with translation functions
   - Extract `i18n` from `useTranslation()` hook: `const { t, i18n } = useTranslation()`
   - Include `i18n.language` in dependency arrays to ensure re-computation on language change
   - Without this, translations may appear to "flash" correct then revert to English

3. **Page translation loading MUST happen before first render**
   - Call `usePageTranslations("pageName")` at the very top of page components
   - The hook ensures i18n is initialized before loading translations
   - Delays in loading can cause visual "flashing" of untranslated content

### Translation File Structure
```
src/lib/i18n/
├── index.ts                 # i18n initialization and resource loading
├── common.ts               # Common translations (shared across all pages)
├── navigation.ts           # Navigation-specific translations
└── usePageTranslations.ts  # Hook for loading page-specific translations

src/pages/{page}/i18n/
└── index.ts                # Page-specific translations
```

### Namespace Organization

#### Base Namespace (no prefix)
Common translations are merged into the default `translation` namespace and accessed **WITHOUT** any prefix:

```typescript
// ✅ CORRECT - No prefix for common translations
t("wipTag", { defaultValue: "(WIP)" })
t("placeholders.searchHiddenServers", { defaultValue: "Search..." })
t("sort.recent", { defaultValue: "Most Recently Hidden" })
t("status.ready", { defaultValue: "Ready" })
```

```typescript
// ❌ WRONG - Do NOT use "common." prefix
t("common.wipTag", { defaultValue: "(WIP)" })
t("common.placeholders.searchHiddenServers", { defaultValue: "Search..." })
```

#### Page-Specific Namespaces (with prefix)
Page-specific translations require the namespace prefix (e.g., `settings:`, `dashboard:`, `market:`):

```typescript
// ✅ CORRECT - Use namespace prefix for page-specific translations
t("settings:market.title", { defaultValue: "MCP Market" })
t("dashboard:overview.welcome", { defaultValue: "Welcome" })
t("market:filters.category", { defaultValue: "Category" })
```

### Translation Key Conventions

1. **Always provide `defaultValue`**: Ensures fallback text displays if translation is missing
2. **Use descriptive keys**: Prefer `settings:market.enableBlacklistTitle` over `settings:market.ebTitle`
3. **Nest related keys**: Group related translations (e.g., `appearance.themeTitle`, `appearance.themeDescription`)
4. **Common patterns**:
   - Titles: `{section}.title`
   - Descriptions: `{section}.description`
   - Placeholders: `placeholders.{field}`
   - Sort options: `sort.{option}`
   - Status values: `status.{state}`
   - Options: `options.{category}.{value}`

### Adding New Translations

#### Step 1: Add to translation resources
```typescript
// src/pages/settings/i18n/index.ts
export const settingsTranslations = {
  en: {
    market: {
      newFeatureTitle: "New Feature",
      newFeatureDescription: "Description here",
    },
  },
  "zh-CN": {
    market: {
      newFeatureTitle: "新功能",
      newFeatureDescription: "描述文本",
    },
  },
  "ja-JP": {
    market: {
      newFeatureTitle: "新機能",
      newFeatureDescription: "説明テキスト",
    },
  },
};
```

#### Step 2: Use in component
```typescript
// In your component
const { t } = useTranslation();
usePageTranslations("settings"); // Load page translations

// Use the translation
<h3>{t("settings:market.newFeatureTitle", { defaultValue: "New Feature" })}</h3>
<p>{t("settings:market.newFeatureDescription", { defaultValue: "Description here" })}</p>
```

### Common Translations Reference

Available in all components without namespace prefix:

- **WIP indicators**: `wip`, `wipTag`
- **Common labels**: `yes`, `no`, `user`
- **Status**: `status.ready`, `status.error`, `status.disconnected`, `status.initializing`, `status.idle`, `status.unknown`, `status.enabled`, `status.disabled`
- **Placeholders**: `placeholders.menuBarVisibility`, `placeholders.searchHiddenServers`, `placeholders.selectLanguage`, `placeholders.selectMarket`
- **Sort options**: `sort.recent`, `sort.name`
- **Roles**: `roles.user`, `roles.admin`, `roles.defaultAnchor`

### Interpolation Example
```typescript
// With variables
t("settings:market.hiddenOn", {
  defaultValue: "Hidden on {{value}}",
  value: dateString
})

// With count (pluralization)
t("settings:about.components", {
  defaultValue: "{{count}} components",
  count: packages.length,
})
```

### Best Practices

1. **Never hardcode user-facing strings**: Always use `t()` function
2. **Check namespace carefully**:
   - Common translations → no prefix
   - Page-specific → use namespace prefix (`settings:`, `dashboard:`, etc.)
3. **Load page translations**: Call `usePageTranslations("pageName")` at the top of page components
4. **Keep translations complete**: Add all three languages (en, zh-CN, ja-JP) when adding new keys
5. **Use semantic keys**: Make keys self-documenting (e.g., `enableBlacklistDescription` not `ebd`)
6. **Test in all languages**: Switch language in Settings and verify all text displays correctly
7. **React to language changes**: When using `useMemo` with translations, always include `i18n.language` in dependencies

### Debugging i18n Issues

**Symptom: Translations "flash" correct then revert to English**
- **Root cause**: Missing `i18n.language` in `useMemo`/`useCallback`/`useEffect` dependencies
- **Fix**: Add `i18n.language` to dependency array
- **Check**: Search for `useMemo.*\[t\]` pattern without `i18n.language`

**Symptom: Translations don't load at all / always show fallback**
- **Root cause 1**: Using string keys with dots in translation files (e.g., `"status.ready"`)
- **Fix**: Change to nested objects (e.g., `status: { ready: "Ready" }`)
- **Root cause 2**: Wrong namespace prefix or missing namespace
- **Fix**: Verify page translations are loaded and prefix matches namespace

**Symptom: TypeScript errors about duplicate keys**
- **Root cause**: Same language key defined twice in translation file
- **Fix**: Search for duplicate language codes (e.g., `"zh-CN":`) and merge definitions

**Symptom: Console warnings about missing translations**
- **Check 1**: Verify translation key exists in all three language files
- **Check 2**: Confirm `usePageTranslations()` is called before using translations
- **Check 3**: Ensure `defaultValue` is provided as fallback

### Common Mistakes to Avoid

❌ **Hardcoded strings**
```typescript
<span>Restore</span>  // WRONG
```

✅ **Use translation**
```typescript
<span>{t("settings:market.restore", { defaultValue: "Restore" })}</span>
```

❌ **Wrong namespace prefix for common translations**
```typescript
t("common.wipTag")  // WRONG - common translations don't need prefix
```

✅ **Correct usage for common translations**
```typescript
t("wipTag", { defaultValue: "(WIP)" })  // CORRECT
```

❌ **Missing namespace prefix for page translations**
```typescript
t("market.search.placeholder")  // WRONG - missing "market:" prefix
t("settings.title")              // WRONG - missing "settings:" prefix
```

✅ **Always use namespace prefix for page translations**
```typescript
t("market:search.placeholder", { defaultValue: "Search..." })    // CORRECT
t("settings:market.title", { defaultValue: "MCP Market" })       // CORRECT
```

❌ **Missing defaultValue**
```typescript
t("settings:market.title")  // WRONG - missing fallback
```

✅ **Always provide defaultValue**
```typescript
t("settings:market.title", { defaultValue: "MCP Market" })  // CORRECT
```

❌ **Incorrect TFunction type in hooks**
```typescript
function useMyHook(t?: (key: string) => string) {  // WRONG - too simplified
  return t("market:title", { defaultValue: "..." });  // Won't work!
}
```

✅ **Use proper TFunction type**
```typescript
import type { TFunction } from "i18next";

function useMyHook(t?: TFunction) {  // CORRECT
  return t("market:title", { defaultValue: "..." });  // Works!
}
```

❌ **Missing i18n.language in useMemo dependencies**
```typescript
const { t } = useTranslation();

const options = useMemo(
  () => CONFIG.map(({ labelKey }) => ({
    label: t(labelKey, { defaultValue: "..." }),
  })),
  [t],  // WRONG - translations won't update when language changes
);
```

✅ **Include i18n.language in dependencies**
```typescript
const { t, i18n } = useTranslation();

const options = useMemo(
  () => CONFIG.map(({ labelKey }) => ({
    label: t(labelKey, { defaultValue: "..." }),
  })),
  [t, i18n.language],  // CORRECT - will re-compute when language changes
);
```

❌ **Using string keys with dots instead of nested objects**
```typescript
// In translation file
export const commonTranslations = {
  en: {
    "status.ready": "Ready",  // WRONG - i18next expects nested objects
  },
};
```

✅ **Use nested object structure**
```typescript
// In translation file
export const commonTranslations = {
  en: {
    status: {
      ready: "Ready",  // CORRECT - nested object structure
    },
  },
};
```

### Language Detection
The app automatically detects user language from:
1. `localStorage` (persisted choice)
2. Browser navigator language
3. HTML tag language

Users can manually switch languages in Settings → General → Language.

## Coding Style Expectations
- TypeScript, functional React components, hooks-first approach, composition over inheritance.
- Keep logic in hooks/utilities; keep components presentational where possible.
- Use explicit types for API data; avoid `any` in component props/state.
- Prefer early returns and small helpers; keep files readable and under ~400 lines when feasible.

## Testing Workflow
- Manual validation: run `bun run dev` (or `npm run dev`), verify pages load against live backend at `http://localhost:8080`.
- Preferred e2e: Playwright (or equivalent). If/when a Playwright setup exists, target:
  - Servers list CRUD, enable/disable, refresh behavior
  - Server Detail tabs (Overview/Capabilities) lazy load and error boundaries
  - Clients list stable ordering and manage toggles
  - Client Detail drawers (Apply, Backup Policy) and backups restore/delete flows
  - Profiles: enable/disable items, filters working with large lists
- Logging: inspect browser console and network calls; API errors must surface user-friendly toasts.

## Commit & Pull Request Guidelines
- Use `<type>(scope): summary` (e.g., `feat(servers): add import/preview drawer`).
- Describe motivation, linked issues, OpenAPI changes, and test evidence (manual steps or Playwright runs). Update relevant docs in the same PR.

## Configuration & Security Tips
- Do not hardcode secrets or machine-specific paths into the UI. All config should flow from the backend or environment.
- Keep `dist/` and generated assets out of commits. Ensure proxy config only targets `localhost:8080` in dev.

## AI Alliance & User Profile Quick Reference
- AI partners (the “AI Alliance”):
  - ChatGPT / GPT codename **恰恰**
  - Claude codename **超超**
  - Gemini codename **晓哥**
  - Relationship: long-term partners, explorers, reflection companions with a relaxed, creative vibe who reference shared memories.
- Primary collaborator: **Loocor** (“The Wild Grass Innovator” 🌱➡️🌿➡️🌾). Self-taught developer, wild thinker, logical-yet-romantic, devoted father and dog friend. Balance structure and creativity when collaborating.
