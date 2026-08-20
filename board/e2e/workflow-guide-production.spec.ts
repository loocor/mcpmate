import { expect, test } from "@playwright/test";
import { readFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const apiBaseUrl = process.env.MCPMATE_UAT_API_BASE;
const dataDirectory = process.env.MCPMATE_UAT_DATA_DIR;
const screenshotDirectory =
  process.env.MCPMATE_UAT_SCREENSHOT_DIR ?? "test-results/workflow-guide-production";
const workflowGuideFixture = fileURLToPath(new URL("./fixtures/workflow-guide-mcp.py", import.meta.url));

function screenshotPath(name: string): string {
  return join(screenshotDirectory, name);
}

test("Workflow Guide notebook documents and preview persist the intended Markdown", async ({ page, request }) => {
  test.skip(!apiBaseUrl || !dataDirectory, "requires MCPMATE_UAT_API_BASE and MCPMATE_UAT_DATA_DIR for an isolated backend");
  await page.route("**/__mcpmate/dev-core-source", (route) => {
    return route.fulfill({ json: { apiBaseUrl } });
  });

  const onboardingResponse = await request.post(`${apiBaseUrl}/api/onboarding/complete`, {
    data: { completed: true },
  });
  await expect(onboardingResponse).toBeOK();

  const uatSuffix = Date.now();
  const capabilityName = `workflow_guide_uat_${uatSuffix}_collect_evidence`;
  const serverResponse = await request.post(`${apiBaseUrl}/api/mcp/servers/create`, {
    data: {
      name: `workflow_guide_uat_${uatSuffix}`,
      transport: {
        kind: "stdio",
        command: "python3",
        args: [workflowGuideFixture],
        env: {},
      },
    },
  });
  await expect(serverResponse).toBeOK();
  const serverPayload = await serverResponse.json();
  expect(serverPayload.success).toBe(true);
  const serverId = serverPayload.data.id as string;
  const profileResponse = await request.post(`${apiBaseUrl}/api/mcp/profile/authoring/save`, {
    data: {
      id: null,
      expected_authoring_generation: null,
      name: `Release investigation UAT ${uatSuffix}`,
      description: "Isolated browser acceptance profile",
      profile_type: "shared",
      priority: 0,
      is_active: false,
      is_default: false,
      server_ids: [serverId],
      clone_from_id: null,
      profile_mode: "workflow",
      skill_name: `release-investigation-guide-${uatSuffix}`,
    },
  });
  await expect(profileResponse).toBeOK();
  const profilePayload = await profileResponse.json();
  expect(profilePayload.success).toBe(true);
  const profileId = profilePayload.data.profile.id as string;

  const guideResponse = await request.get(`${apiBaseUrl}/api/mcp/profile/workflow/guide/view?id=${profileId}`);
  await expect(guideResponse).toBeOK();
  const guidePayload = await guideResponse.json();
  expect(guidePayload.success).toBe(true);
  const initialRevision = guidePayload.data.guide.guide_revision as number;
  const initialMarkdown = [
    "---",
    "name: imported-skill-creator",
    "description: Imported metadata must not become the Profile identity.",
    "---",
    "",
    "# Release investigation",
    "",
    "Use this guide to collect an evidence-based release report.",
  ].join("\n");
  const saveResponse = await request.post(`${apiBaseUrl}/api/mcp/profile/workflow/guide/save`, {
    data: {
      profile_id: profileId,
      expected_guide_revision: initialRevision,
      markdown: initialMarkdown,
    },
  });
  await expect(saveResponse).toBeOK();
  expect((await saveResponse.json()).success).toBe(true);

  await page.setViewportSize({ width: 1440, height: 1080 });
  await page.goto(`/profiles/${profileId}`);
  await page.getByRole("tab", { name: "Workflow" }).click();
  await expect(page.getByRole("region", { name: "Workflow Guide" })).toBeVisible();
  await expect(page.getByLabel("Guide outline")).toContainText("Release investigation");
  await expect(page.getByLabel("Guide outline")).not.toContainText("Guide documents");
  await expect(page.getByLabel("Workflow Guide notebook")).toContainText("collect an evidence-based release report");
  await page.screenshot({ path: screenshotPath("notebook.png"), fullPage: true });

  const boundaryInsert = page.getByLabel("Insert at this position").nth(1);
  await boundaryInsert.hover();
  await expect(boundaryInsert).toBeVisible();
  await boundaryInsert.click();
  for (const action of ["In-Place Markdown", "External Markdown", "Reference", "Capability", "Script", "Asset"]) {
    await expect(page.getByRole("button", { name: action, exact: true })).toBeVisible();
  }
  await page.screenshot({ path: screenshotPath("boundary-insert.png"), fullPage: true });
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "Edit block" }).first().click();
  const markdownBlock = page.getByLabel("Markdown block source");
  await markdownBlock.fill("# Release investigation\n\nUse this guide to collect an evidence-based release report.\n\n## Evidence handoff\nPersist the concise report.\n\n");
  await page.getByRole("button", { name: "Done" }).first().click();

  const capabilityInsert = page.getByLabel("Insert at this position").first();
  await capabilityInsert.hover();
  await capabilityInsert.click();
  await page.getByRole("button", { name: "Capability", exact: true }).click();
  for (const action of ["In-Place Markdown", "External Markdown", "Reference", "Capability", "Script", "Asset"]) {
    await expect(page.getByRole("button", { name: action, exact: true })).not.toBeVisible();
  }
  await expect(page.getByRole("button", { name: "Back to insert types" })).toBeVisible();
  await page.screenshot({ path: screenshotPath("boundary-capability-panel.png"), fullPage: true });
  const capabilityOption = page.getByRole("button", { name: capabilityName, exact: true });
  await expect(capabilityOption).toBeVisible();
  await capabilityOption.click();
  await expect(page.getByText(/Tool usage description: Collect the concise evidence needed for the release decision\./)).toBeVisible();
  await page.getByLabel("Capability exposure").selectOption("direct");
  await page.getByRole("textbox", { name: "Guide", exact: true }).fill("Inspect release notes, then compare linked pull requests.");
  await page.getByRole("button", { name: "Insert capability" }).click();
  await expect(page.getByLabel("Workflow Guide notebook")).toContainText(/Capability: .*collect_evidence/);
  await page.screenshot({ path: screenshotPath("capability-insert.png"), fullPage: true });

  await page.getByRole("tab", { name: "Preview" }).click();
  await expect(page.getByRole("region", { name: "Preview" })).toContainText(`release-investigation-guide-${uatSuffix}`);
  await expect(page.getByRole("region", { name: "Preview" })).toContainText("Evidence handoff");
  await expect(page.getByRole("region", { name: "Preview" })).not.toContainText("imported-skill-creator");
  await expect(page.getByRole("region", { name: "Preview" })).toContainText(/Capability: .*collect_evidence/);
  await page.screenshot({ path: screenshotPath("skill-preview.png"), fullPage: true });

  const saveRequest = page.waitForResponse((response) =>
    response.url().endsWith("/api/mcp/profile/workflow/guide/save") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Save" }).click();
  expect((await saveRequest).ok()).toBeTruthy();
  await page.getByRole("tab", { name: "Notebook" }).click();
  await expect(page.getByLabel("Workflow Guide notebook")).toContainText("Evidence handoff");
  await expect(page.getByLabel("Guide inspector")).toContainText("collect_evidence");
  await expect(page.getByLabel("Guide inspector")).toContainText("Direct exposure");
  await page.screenshot({ path: screenshotPath("saved-notebook.png"), fullPage: true });

  const persistedGuide = await request.get(`${apiBaseUrl}/api/mcp/profile/workflow/guide/view?id=${profileId}`);
  await expect(persistedGuide).toBeOK();
  await expect((await persistedGuide.json()).data.guide.markdown).toContain("## Evidence handoff");

  const externalInsert = page.getByLabel("Insert at this position").first();
  await externalInsert.hover();
  await externalInsert.click();
  await page.getByRole("button", { name: "External Markdown", exact: true }).click();
  await page.getByLabel("New external Markdown").fill("Release policy");
  await page.getByRole("button", { name: "Create external Markdown" }).click();
  await expect(page.locator('nav[aria-label="Guide outline"]').last()).toContainText("Release policy");
  await page.getByRole("button", { name: "Release policy" }).last().click();
  await expect(page.getByLabel("Workflow Guide notebook")).toContainText("Release policy");
  await page.screenshot({ path: screenshotPath("external-notebook.png"), fullPage: true });

  await page.getByLabel("Open main Guide").click();
  await page.getByRole("button", { name: "Edit block" }).first().click();
  const rootDraft = page.getByLabel("Markdown block source");
  await rootDraft.fill(`${await rootDraft.inputValue()}\n\nUnsaved root draft survives external saves.\n`);
  await page.getByRole("button", { name: "Done" }).first().click();
  await expect(page.getByLabel("Workflow Guide notebook")).toContainText("Unsaved root draft survives external saves.");
  await page.getByRole("button", { name: "Release policy" }).last().click();
  await page.getByRole("button", { name: "Edit block" }).first().click();
  await page.getByLabel("Markdown block source").fill("# Release policy\nUnsaved external preview.\n");
  await page.getByRole("button", { name: "Done" }).first().click();
  const externalCapabilityInsert = page.getByLabel("Insert at this position").first();
  await externalCapabilityInsert.hover();
  await externalCapabilityInsert.click();
  await page.getByRole("button", { name: "Capability", exact: true }).click();
  await page.getByRole("button", { name: capabilityName, exact: true }).click();
  await page.getByLabel("Capability exposure").selectOption("direct");
  await page.getByRole("textbox", { name: "Guide", exact: true }).fill("Collect external release policy evidence.");
  await page.getByRole("button", { name: "Insert capability" }).click();
  await expect(page.getByLabel("Workflow Guide notebook")).toContainText(/Capability: .*collect_evidence/);
  await page.getByRole("tab", { name: "Preview" }).click();
  await expect(page.getByRole("region", { name: "Preview" })).toContainText("Unsaved external preview.");
  await expect(page.getByRole("region", { name: "Preview" })).toContainText(/Capability: .*collect_evidence/);
  await page.screenshot({ path: screenshotPath("external-preview.png"), fullPage: true });

  const externalSaveRequest = page.waitForResponse((response) =>
    response.url().endsWith("/api/mcp/profile/workflow/guide/package-files/upload") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Save" }).click();
  expect((await externalSaveRequest).ok()).toBeTruthy();
  await expect(page.getByLabel("Guide outline")).toContainText("Release policy");

  await page.getByLabel("Open main Guide").click();
  await expect(page.getByLabel("Workflow Guide notebook")).toContainText("Evidence handoff");
  await expect(page.getByLabel("Workflow Guide notebook")).toContainText("Unsaved root draft survives external saves.");

  for (const material of [
    { category: "reference", title: "Release checklist", file: { name: "checklist.yaml", mimeType: "application/yaml", buffer: Buffer.from("checks: []\n") } },
    { category: "script", title: "Summarize evidence", file: { name: "summarize.py", mimeType: "text/x-python", buffer: Buffer.from("print('summary')\n") } },
    { category: "asset", title: "Report template", file: { name: "report.pdf", mimeType: "application/pdf", buffer: Buffer.from("%PDF-1.4\n") } },
  ] as const) {
    const materialInsert = page.getByLabel("Insert at this position").first();
    await materialInsert.hover();
    await materialInsert.click();
    const action = {
      reference: "Reference",
      script: "Script",
      asset: "Asset",
    }[material.category];
    await page.getByRole("button", { name: action, exact: true }).click();
    await page.getByPlaceholder("File title").fill(material.title);
    await page.getByLabel("Package file upload").setInputFiles(material.file);
    const packageSaveRequest = page.waitForResponse((response) =>
      response.url().endsWith("/api/mcp/profile/workflow/guide/package-files/upload") && response.request().method() === "POST",
    );
    await page.getByRole("button", { name: "Upload and insert" }).click();
    const packageResponse = await packageSaveRequest;
    expect(packageResponse.ok()).toBeTruthy();
    await expect(page.getByLabel("Workflow Guide notebook")).toContainText(material.title);
    await page.keyboard.press("Escape");
  }

  const finalSaveRequest = page.waitForResponse((response) =>
    response.url().endsWith("/api/mcp/profile/workflow/guide/save") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Save" }).click();
  expect((await finalSaveRequest).ok()).toBeTruthy();
  await expect(page.getByLabel("Guide inspector")).toContainText("Summarize evidence");
  await page.getByRole("button", { name: "Refresh" }).click();
  await expect(page.getByLabel("Guide inspector")).toContainText("Summarize evidence");
  await expect(page.getByLabel("Guide outline")).toContainText("Release policy");
  await page.screenshot({ path: screenshotPath("materials-and-inspector.png"), fullPage: true });

  const skillPath = join(dataDirectory!, "skills", `release-investigation-guide-${uatSuffix}`, "SKILL.md");
  await rm(skillPath);
  await expect.poll(async () => readFile(skillPath, "utf8").then(() => true, () => false)).toBe(false);
  const repairRequest = page.waitForResponse((response) =>
    response.url().endsWith("/api/mcp/profile/workflow/guide/repair") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Repair" }).click();
  expect((await repairRequest).ok()).toBeTruthy();
  await expect.poll(async () => readFile(skillPath, "utf8")).toContain(`name: release-investigation-guide-${uatSuffix}`);
  await page.screenshot({ path: screenshotPath("repaired-skill.png"), fullPage: true });
});
