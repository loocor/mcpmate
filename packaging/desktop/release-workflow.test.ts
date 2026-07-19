import { describe, expect, test } from "bun:test";

const releaseWorkflowUrl = new URL("../../.github/workflows/release.yml", import.meta.url);
const dockerWorkflowUrl = new URL("../../.github/workflows/docker-publish.yml", import.meta.url);
const injectVersionActionUrl = new URL("../../.github/actions/inject-version/action.yml", import.meta.url);
const nightlyWorkflowUrl = new URL("../../.github/workflows/nightly.yml", import.meta.url);
const desktopWorkflowUrls = ["desktop-macos.yml", "desktop-windows.yml", "desktop-linux.yml"].map(
  (name) => new URL(`../../.github/workflows/${name}`, import.meta.url),
);

describe("release workflow contract", () => {
  test("classifies the release before platform builds", async () => {
    const workflow = await Bun.file(releaseWorkflowUrl).text();
    const validateJob = workflow.indexOf("  validate-tag:");
    const firstBuildJob = workflow.indexOf("  build-macos:");

    expect(validateJob).toBeGreaterThanOrEqual(0);
    expect(firstBuildJob).toBeGreaterThan(validateJob);
    expect(workflow.slice(validateJob, firstBuildJob)).toContain("bun packaging/desktop/release-tag.ts");
    expect(workflow).toContain("release-channel: ${{ steps.classify.outputs.release-channel }}");
    expect(workflow).toContain("app-version: ${{ steps.classify.outputs.app-version }}");
  });

  test("fails on missing distribution tokens before platform builds or release mutation", async () => {
    const workflow = await Bun.file(releaseWorkflowUrl).text();
    const validateJob = workflow.indexOf("  validate-tag:");
    const firstBuildJob = workflow.indexOf("  build-macos:");
    const prebuild = workflow.slice(validateJob, firstBuildJob);

    expect(prebuild).toContain("- name: Validate distribution credentials");
    expect(prebuild).toContain("DOWNLOADS_WORKFLOW_TOKEN: ${{ secrets.DOWNLOADS_WORKFLOW_TOKEN }}");
    expect(prebuild).toContain("HOMEBREW_TAP_TOKEN: ${{ secrets.HOMEBREW_TAP_TOKEN }}");
    expect(prebuild).toContain('test -n "$DOWNLOADS_WORKFLOW_TOKEN"');
    expect(prebuild).toContain('test -n "$HOMEBREW_TAP_TOKEN"');
  });

  test("uses the classified channel and app version throughout the release", async () => {
    const workflow = await Bun.file(releaseWorkflowUrl).text();

    expect(workflow).toContain("RELEASE_CHANNEL: ${{ needs.validate-tag.outputs.release-channel }}");
    expect(workflow).toContain("APP_VERSION: ${{ needs.validate-tag.outputs.app-version }}");
    expect(workflow).toContain("prerelease: ${{ env.RELEASE_CHANNEL == 'beta' }}");
    expect(workflow).not.toContain("contains(env.RELEASE_TAG, '-')");
  });

  test("requires Windows ARM64 artifacts consistently with the release manifest", async () => {
    const workflow = await Bun.file(releaseWorkflowUrl).text();
    const windowsJob = workflow.slice(
      workflow.indexOf("  build-windows:"),
      workflow.indexOf("  build-linux:"),
    );

    expect(windowsJob).not.toContain("experimental:");
    expect(windowsJob).not.toContain("continue-on-error:");
    expect(workflow).toContain('"MCPMate_${VERSION}_windows_arm64.msi"');
    expect(workflow).toContain(
      '::error::Missing required release asset: MCPMate_${VERSION}_windows_arm64.msi.sig or MCPMate_${VERSION}_windows_arm64.msi.zip.sig',
    );
    expect(workflow).not.toContain("::warning::Experimental asset not found");
  });

  test("synchronizes Admin and release notes before dispatching the Tap", async () => {
    const workflow = await Bun.file(releaseWorkflowUrl).text();
    const synchronize = workflow.indexOf("- name: Synchronize exact-tag download manifest");
    const updateNotes = workflow.indexOf("- name: Update GitHub Release body");
    const dispatch = workflow.indexOf("- name: Dispatch Homebrew Tap update");

    expect(synchronize).toBeGreaterThanOrEqual(0);
    expect(updateNotes).toBeGreaterThan(synchronize);
    expect(dispatch).toBeGreaterThan(updateNotes);
    expect(workflow.slice(dispatch)).toContain("bun packaging/desktop/dispatch-homebrew-tap.ts");
  });

  test("gates GitHub Release publishing on the reusable Docker workflow", async () => {
    const releaseWorkflow = await Bun.file(releaseWorkflowUrl).text();
    const dockerWorkflow = await Bun.file(dockerWorkflowUrl).text();
    const dockerJob = releaseWorkflow.slice(
      releaseWorkflow.indexOf("  publish-docker:"),
      releaseWorkflow.indexOf("  release:"),
    );
    const releaseJob = releaseWorkflow.slice(releaseWorkflow.indexOf("  release:"));

    expect(dockerWorkflow).toContain("  workflow_call:");
    expect(dockerWorkflow).toContain("  pull_request:\n    branches:\n      - main");
    expect(dockerWorkflow.match(/default: false/g)).toHaveLength(2);
    expect(dockerWorkflow).not.toContain('    tags:\n      - "v*"');
    expect(dockerWorkflow).toContain("uses: dtolnay/rust-toolchain@1.97.1");
    expect(dockerWorkflow).toContain("ref: ${{ inputs.release_tag || github.ref }}");
    expect(dockerWorkflow).toContain("enable=${{ inputs.push_image == true }}");
    expect(dockerWorkflow).toContain("push: ${{ inputs.push_image == true }}");
    expect(dockerJob).toContain("uses: ./.github/workflows/docker-publish.yml");
    expect(dockerJob).toContain("packages: write");
    expect(dockerJob).toContain("release_tag: ${{ needs.validate-tag.outputs.tag }}");
    expect(dockerJob).toContain("push_image: true");
    expect(releaseJob).toContain(
      "needs: [validate-tag, build-macos, build-windows, build-linux, publish-docker]",
    );
  });

  test("does not dispatch Homebrew from Nightly or desktop build workflows", async () => {
    const workflows = await Promise.all(
      [nightlyWorkflowUrl, ...desktopWorkflowUrls].map((url) => Bun.file(url).text()),
    );

    for (const workflow of workflows) {
      expect(workflow).not.toContain("mcpmate_release");
      expect(workflow).not.toContain("dispatch-homebrew-tap.ts");
    }
  });
});

describe("inject-version action contract", () => {
  test("uses the canonical classifier for tag-driven releases without a configured-version fallback", async () => {
    const action = await Bun.file(injectVersionActionUrl).text();

    expect(action).toContain("bun packaging/desktop/release-tag.ts");
    expect(action).toContain('if [[ -n "$RELEASE_TAG" ]]');
    expect(action).toContain("jq -r '.appVersion'");
    expect(action).toContain('--configured-version "$CONFIGURED_VERSION"');
    expect(action).not.toContain("fall back");
    expect(action).not.toContain('VERSION="${VERSION%%-*}"');
  });
});
