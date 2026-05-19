---
name: cloudflare-d1
description: Use this skill when MCPMate work touches Cloudflare Workers D1 persistence, relational catalog storage, D1 schema design, migrations, local versus remote D1 development, deployment ordering, seed data, or D1 validation for the Admin catalog service.
---

# Cloudflare D1

Use this skill to keep Cloudflare D1 work predictable across local development,
tests, preview/staging, and production deployments.

## Goals

- Keep MCPMate Admin on Cloudflare Workers while using D1 as the relational
  source of truth for catalog data.
- Prevent local, preview, staging, or production environments from sharing the
  wrong D1 database.
- Keep migrations explicit, reviewable, and separate from runtime handlers.
- Keep public discovery endpoints cheap and stable by reading published
  artifacts or snapshots instead of doing complex live joins.

## When This Skill Applies

Use this skill when work involves:

- Adding or changing a D1 binding in `wrangler.toml`.
- Designing D1 tables for Admin catalog entities, reviews, audit events, or
  publish snapshots.
- Writing, applying, or validating D1 migrations.
- Seeding catalog data from static JSON, fixtures, or generated artifacts.
- Moving catalog persistence from KV/static JSON to D1.
- Testing Worker APIs that read or write D1.
- Deploying Worker changes that depend on a D1 schema change.

## Source Of Truth

- D1 should own Admin catalog source data when relational persistence is needed.
- KV may still hold OAuth state, short-lived auth helpers, and optional published
  artifact caches.
- Runtime request handlers must not create or alter tables.
- Published discovery output should be generated from D1 source data and stored
  as an immutable snapshot or cacheable artifact.

## Data Model Guidance

Prefer a relational skeleton with JSON detail columns:

- Indexable columns: `kind`, `slug`, `identifier`, `visibility`,
  `recommendation_tier`, `sort_key`, `created_at`, `updated_at`.
- JSON detail columns: `presentation_json`, `configuration_json`,
  `detection_json`, `format_rules_json`, `official_json`, `curated_json`,
  `runtime_json`, `artifact_json`, `detail_json`.
- Keep generated consumer payloads derived from semantic source rows. Do not make
  `generated` or `exports` the primary editable source.

Recommended catalog tables:

- `catalog_entries`
- `client_catalog`
- `server_catalog`
- `portal_catalog`
- `catalog_publish_records`
- `catalog_publish_snapshots`
- `catalog_audit_events`
- optional `catalog_reviews`

## Environment Discipline

Always separate D1 databases by environment:

- Local development uses local Wrangler D1 state.
- Preview/staging uses a non-production remote D1 database.
- Production uses a dedicated production D1 database.

Before editing D1 code, inspect the current bindings and environment sections in
`wrangler.toml`. Do not assume the default binding points to the intended
database.

Use one stable binding name in code, for example `env.MCPMATE_DB`, and vary the
database target through Wrangler environment configuration.

## Migration Workflow

1. Create an append-only migration under `migrations/`.
2. Apply it locally.
3. Run repository tests against a seeded local or fake D1 database.
4. Apply the remote migration for the target environment.
5. Deploy the Worker for that same environment.
6. Smoke test public read endpoints and authenticated Admin mutation endpoints.

Rules:

- Never edit a migration that has already been applied remotely.
- Do not hide schema changes inside request handlers.
- Data migrations should be explicit scripts or migrations, not opportunistic
  runtime fallback logic.
- If a migration is destructive, document the approved reason in the Project item
  and PR.

## Local Development

For local development, prefer Wrangler local D1 and deterministic seed data.

Typical command shape:

```bash
bunx --bun wrangler d1 migrations apply MCPMATE_DB --local
bun run dev
```

Local data is not production data. Tests and local development must not depend on
pre-existing local D1 rows.

## Deployment Ordering

Do not deploy Worker code that expects a new schema before applying the matching
remote migration.

Preferred sequence:

```text
create migration
apply local migration
run tests
apply remote migration for target env
deploy Worker for target env
smoke test target env
update GitHub Project evidence
```

## Testing Expectations

Use the repo-local `validation` skill with this skill.

Minimum checks for D1-backed catalog changes:

- Migration applies from an empty database.
- Seed/import creates the expected default catalog rows.
- CRUD changes do not affect public discovery until publish.
- Publish creates an immutable artifact snapshot.
- Rollback restores a previous published artifact.
- Public `/discovery/*` endpoints do not require Admin auth.
- Admin mutation endpoints require Admin auth.
- Query plans have indexes for common filters and sorts.

## Cost And Limits Awareness

Before production-facing D1 changes, verify current official D1 pricing and
limits from Cloudflare docs because limits can change.

Design defaults:

- Public discovery endpoints should read a published artifact or narrow indexed
  query, not full-scan source tables.
- Audit logs and publish snapshots need retention policy.
- Index common filters even though indexes add write cost; the read reduction is
  usually worth it.
- Track D1 `meta.rows_read` and `meta.rows_written` during smoke tests when
  query cost is uncertain.

## Reporting

When reporting D1 work, include:

- Which binding and environment were used.
- Which migrations were added and where they were applied.
- What seed data was used.
- Which tests and smoke checks passed.
- Whether public discovery reads from D1 directly, a D1 snapshot, or KV cache.
- Any remaining risk around environment binding, migration order, or row-scan
  cost.
