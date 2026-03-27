# Website Development Guidelines

## Collaboration Rhythm (Discuss → Build → Report)
- Day-to-day coordination with LLM/AI agents is done in **Chinese**.
- Source code, doc comments, documentation, and git commit messages stay in **English** for consistency across the repository.

## Project Structure

```
website/
├── src/
│   ├── i18n/                    # Marketing page translations
│   │   ├── en.ts                # English (primary)
│   │   ├── zh.ts                # Chinese (primary)
│   │   └── ja.ts                # Japanese (secondary)
│   ├── docs/
│   │   ├── pages/
│   │   │   ├── en/              # English documentation pages
│   │   │   ├── zh/              # Chinese documentation pages
│   │   │   └── ja/              # Japanese documentation pages
│   │   ├── changelog/
│   │   │   ├── en.json
│   │   │   ├── zh.json
│   │   │   └── ja.json
│   │   ├── nav.ts               # Navigation configuration (all locales)
│   │   └── DocRoutes.tsx        # Route generation
│   └── components/
│       └── LanguageProvider.tsx # Language detection & switching
├── AGENTS.md                    # This file
└── README.md
```

## Internationalization (i18n) Conventions

### Language Priority

| Priority | Language | Code | Role |
|----------|----------|------|------|
| **Primary** | English | `en` | Source of truth for content |
| **Primary** | Chinese | `zh` | Co-equal with English, updated in parallel |
| **Secondary** | Japanese | `ja` | Derived from primary languages, synced after updates |

### Translation Workflow

1. **Primary Language Updates**:
   - When adding/modifying content, update `en.ts` and `zh.ts` **together**
   - Both primary languages should be updated in the same PR
   - Ensure feature parity between English and Chinese

2. **Secondary Language Sync**:
   - Japanese translations are updated **after** primary languages stabilize
   - Use Board's i18n terminology as the authoritative source
   - Reference `board/src/lib/i18n/common.ts` and page-specific i18n files

3. **New Feature Checklist**:
   - [ ] Add keys to `src/i18n/en.ts`
   - [ ] Add translations to `src/i18n/zh.ts`
   - [ ] Create doc page in `src/docs/pages/en/`
   - [ ] Create doc page in `src/docs/pages/zh/`
   - [ ] Update `src/docs/nav.ts` for both locales
   - [ ] Sync Japanese when ready (`ja.ts`, `pages/ja/`)

### Terminology Consistency

All Website and Document terminology **MUST** align with Board (`board/src/lib/i18n/`):

| English | Chinese | Japanese | Context |
|---------|---------|----------|---------|
| Dashboard | 仪表盘 | ダッシュボード | Main console |
| Profiles | 配置集 | プロファイル | Capability sets |
| Servers | 服务器 | サーバー | MCP servers |
| Clients | 客户端 | クライアント | AI clients |
| Market | 服务源 | マーケット | MCP registry |
| Runtime | 运行时 | ランタイム | Execution environment |
| Audit | 审计 | 監査 | Audit logs |
| Tools | 工具 | ツール | MCP tools |
| Prompts | 提示 | プロンプト | MCP prompts |
| Resources | 资源 | リソース | MCP resources |
| Templates | 模板 | テンプレート | Resource templates |
| Inspector | 检视器 | 検査 | Inspector tool |
| Profile | 配置集 | プロファイル | Single profile |
| Status | 状态 | 状態 | Status indicators |
| Enabled | 已启用 | 有効 | Enabled state |
| Disabled | 已禁用 | 無効 | Disabled state |
| Connected | 已连接 | 接続済み | Connection state |
| Overview | 概览 | 概要 | Tab/section |
| Settings | 设置 | 設定 | Configuration |

### File Naming Conventions

- Marketing translations: `src/i18n/{locale}.ts`
- Doc pages: `src/docs/pages/{locale}/{PascalCase}.tsx`
- Changelog: `src/docs/changelog/{locale}.json`

### Route Structure

- `/docs/en/{page-id}` - English documentation
- `/docs/zh/{page-id}` - Chinese documentation
- `/docs/ja/{page-id}` - Japanese documentation
- `?lang=ja` - URL parameter for marketing pages

## Build Commands

```bash
# Development
bun run dev

# Build
bun run build

# Preview
bun run preview
```

## Content Guidelines

### Documentation Pages

- Use `DocLayout` wrapper for consistent layout
- Import shared components from `src/docs/components/`
- Follow existing heading hierarchy (`H2` > `H3`)
- Include meta description for SEO

### Changelog Format

```json
{
  "versions": [
    {
      "version": "1.0.0",
      "date": "2025-01-15",
      "changes": [
        {
          "type": "feature",
          "description": "Description of the change"
        }
      ]
    }
  ]
}
```

Change types: `feature`, `fix`, `improvement`, `breaking`, `deprecation`

## Board Integration

The `board/src/lib/website-lang.ts` file handles language mapping:

```typescript
// Board → Website language mapping
export function websiteDocsLocale(i18nLanguage: string | undefined): "en" | "zh" {
  // Returns "zh" for Chinese, "en" for all others
  // Update this when adding Japanese docs support
}
```

When adding Japanese docs, update `websiteDocsLocale` to return `"ja"` for Japanese users.
