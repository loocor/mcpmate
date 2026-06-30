---
name: release-alignment
description: Use this skill for MCPMate pre-release or post-release documentation alignment, including GitHub Release Notes, website changelog entries, README release-facing copy, roadmap positioning, localized website docs, and release-description/changelog consistency checks.
---

# Release Alignment

Use this skill when MCPMate release work needs public-facing documentation to stay aligned with what actually shipped.

## Goals

- Treat final GitHub Release Notes as the release truth source.
- Treat the tag-to-tag code diff as the provisional fact source only while the release workflow is still running.
- Keep the website changelog as the durable, localized product record.
- Update only website and README pages whose current copy would become stale, misleading, or disconnected from the release.
- Preserve the boundary between shipped facts, current focus, and future bets.

## Preflight

Before drafting or editing release-alignment docs:

1. Confirm repository context:
   - `git rev-parse --show-toplevel`
   - `git remote get-url origin`
   - `git status --short --branch`
2. Confirm GitHub access and release state:
   - `gh auth status`
   - `git ls-remote --tags origin <target-tag>`
   - `gh release view <target-tag>`
3. If the GitHub Release is missing because the release workflow is still running, inspect the workflow run with `gh run view <run-id>` or the exact run URL provided by Loocor.
4. If GitHub auth, remote tag visibility, or release workflow state cannot be verified, report the gap before making release claims.

## GitHub CLI Auth Boundaries

Release alignment often needs network-backed GitHub CLI checks. In sandboxed environments, a first `gh` command may fail because network access or command escalation has not been granted yet.

- If a `gh` command fails with an access, network, or sandbox permission error, request the required permission and retry before treating GitHub state as unavailable.
- If `gh auth status` shows no valid authentication after permissions are available, report that GitHub auth is missing and request re-authentication instead of guessing release state from local data.
- Prefer GitHub CLI's one-time device-code flow for re-authentication when browser access is unreliable. Use `gh auth login --web --clipboard` when available so Loocor can complete the code-based authorization outside the sandboxed browser path.
- Do not use `--with-token` unless Loocor explicitly provides a token flow for the current session.
- Do not store, echo, commit, or summarize credentials. Only report whether auth is available and which release checks were completed.

## Source Priority

1. If the GitHub Release already exists, start from the Release Notes for the target tag.
2. If the target tag exists but the release workflow is still running and the GitHub Release is not available yet, use the previous-tag-to-target-tag code fact window to draft the release overview and highlights.
3. Use PRs, commits, and Project items only to fill gaps or confirm provenance.
4. Use code and docs diffs to verify that release claims match shipped behavior.
5. Do not synthesize a post-release website changelog from commit history first when final Release Notes already exist.

## Release Fact Window

Default to the facts between the previous release tag and the target release tag.

- Use the previous public release tag, such as `v0.3.3-beta`.
- Exclude nightly tags, test tags, local-only tags, and non-public release tags from the release window unless Loocor explicitly says otherwise.
- Changelogs describe what changed from the previous release to the current release.
- Do not cross release-version boundaries unless the user explicitly asks for a multi-release summary.
- If a change belongs to an earlier tag, do not restate it as part of the current release.
- If a release note needs context from an earlier version, keep that context brief and clearly separate it from current-release changes.

## Pending Release Workflow

When a tag has just been created and the release workflow is still running:

1. Treat the tag-to-tag code diff as the stable draft fact window unless the workflow fails, the tag is moved, or the generated release changes the evidence.
2. Draft the release overview and highlights from that fact window.
3. Use the same drafted release overview and highlights as the changelog fact reference so release description and changelog remain consistent.
4. Website and changelog draft work may proceed, but mark the result as pending workflow verification.
5. Do not report release alignment as complete until the workflow succeeds and the generated GitHub Release Notes have been re-checked.

If the release workflow fails, stop and report the failure as a release-alignment blocker instead of publishing final website changelog claims.

## Release Description Strategy

Write or review the GitHub release description in this shape:

1. Opening paragraph: explain why this release exists and what user/operator problem it addresses.
2. `Highlights`: curate about five changes users should notice; do not mirror every PR.
3. `What's Changed`: preserve PR-level provenance.
4. `Full Changelog`: include the compare link.

Prefer user-facing outcomes over implementation details. Mention internal architecture only when it changes trust, install flow, provenance, reliability, security posture, or operator control.

Keep the opening paragraph and highlight entries concise. Add more than about five highlights only when the release genuinely carries enough user-facing information to justify it.

For fix releases, name the externally shipped failure mode and the recovery impact clearly. Keep the tone factual; do not turn the release note into an incident report.

Do not highlight fixes for problems introduced and corrected while building the same release before it shipped. Users do not need a record of temporary development mistakes that never existed in a released version.

## Website Changelog Strategy

Update `website/src/docs/changelog/{en,zh,ja}.json` from the release notes.

Each release entry should keep this structure:

- `version`: public version without the `v` prefix.
- `date`: release date in `YYYY-MM-DD`.
- `highlights`: one concise paragraph describing the release theme.
- `changes`: 4-6 durable change records with `type` and user-readable `text`.

Rules:

- Keep `highlights` close to the release opening paragraph, but make it suitable for long-term browsing.
- Use `feat`, `fix`, `ref`, `docs`, or `chore` for `type`.
- Avoid raw PR titles, low-level implementation details, and one-off review cleanup.
- Keep changelog facts consistent with the release description.
- Describe only user-facing or operator-facing outcomes, not a technical task log.
- Keep English, Chinese, and Japanese semantically aligned without forcing literal translation.
- Preserve stable product terms such as MCPMate, Market, Secure Store, Uni-Import, GitHub MCP, Cursor.directory, and Server Install Wizard.

## Website And README Alignment

After the changelog is aligned, inspect release-affected public docs only.

Common surfaces:

- `README.md`, `README_CN.md`, and `README_JP.md` for current feature surface, install/download copy, and roadmap summary.
- `website/src/docs/pages/*/Roadmap.tsx` for current focus, next bets, and recently shipped foundations.
- Feature docs such as Browser Extension, Market, Marketplace, Server Import, Secure Store, OAuth, desktop download, or onboarding pages when the shipped release changes those workflows.
- Extension or package README files when release behavior changes outside the website.

Rules:

- Do not turn Roadmap into a changelog. It should summarize product-level direction and point detailed shipped records to the website changelog.
- Do not update unrelated website pages just because they are nearby.
- Do not present aspirational future work as already shipped.
- Keep backend implementation details out of public copy unless users need them to operate the product.
- If a release changes a workflow, update every page that describes the entry points, handoff path, validation step, or operator outcome.

## Branch And Preview Handoff

When updating website or README release-alignment docs, use a release-prepare branch name:

- `docs/release-x.x.x-prepare`

Replace `x.x.x` with the target public version. Keep the branch scoped to release-facing documentation alignment.

Before editing release-facing website or README docs:

1. Check the current branch name.
2. If it is not the expected release-prepare branch, switch to or create the expected branch before editing.
3. Do not mix release-alignment edits into unrelated skill-building, feature, or hotfix branches.

After the first complete draft:

1. Push the branch to `origin`.
2. Return the Cloudflare Pages preview URL for review when the preview is available.
3. Do not create a PR unless Loocor explicitly asks for one.
4. Prefer GitHub deployment/status data, Pages build output, or existing repository release-flow evidence when locating the preview URL.
5. If the preview URL cannot be found or deployment did not start, report that as a handoff blocker instead of inventing a URL.

## Review Checklist

Before reporting alignment complete:

1. Release description has a clear theme, curated highlights, PR provenance, and compare link.
2. Website changelog entry exists in English, Chinese, and Japanese.
3. README release-facing claims match the current shipped surface.
4. Roadmap separates current focus, next bets, and shipped foundations.
5. Feature docs touched by the release describe the same workflow and terminology.
6. Changelog and release description stay within the previous-tag-to-current-tag fact window.
7. Fix highlights describe only issues that existed outside the current release-building process.
8. No unrelated website copy was rewritten.
9. Claims are evidence-backed by release notes, PRs, commits, or current code.

## Reporting

Report:

- Target release or tag.
- Source used: final GitHub Release Notes, or provisional tag-to-tag fact window while workflow is pending.
- Changelog entries updated or reviewed.
- Website/README surfaces updated or intentionally left unchanged.
- Branch name, remote push status, and Cloudflare Pages preview URL when website or README docs were changed.
- Validation performed.

## Validation

- For changelog JSON or README-only edits, run at least `git diff --check`.
- For website `.tsx` docs or rendered-doc changes, run `bun run --cwd website build`.
- Run `bun run --cwd website lint` when the touched surface or local baseline makes lint useful for catching TypeScript, React, or formatting issues.
- If validation is skipped or blocked, report why and what risk remains.
