import { expect, test } from "@playwright/test";

function ok(data: unknown) {
  return {
    status: 200,
    contentType: "application/json",
    body: JSON.stringify({ success: true, data }),
  };
}

test("Server overview leaves capability details to the dedicated tab", async ({
  page,
}) => {
  const capabilityListRequests: string[] = [];

  await page.route("**/api/**", async (route) => {
    const url = new URL(route.request().url());
    switch (url.pathname) {
      case "/api/system/readiness":
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ type: "ready", status: "ok" }),
        });
      case "/api/mcp/servers/details":
        return route.fulfill(
          ok({
            id: "server-sequential-thinking",
            name: "sequential-thinking-server",
            status: "connected",
            enabled: true,
            server_type: "stdio",
            server_info: {
              name: "sequential-thinking-server",
              title: "Sequential Thinking",
              version: "0.2.0",
            },
            instances: [],
            source_revision_set: {
              "server-sequential-thinking": 11,
            },
            capability: {
              snapshotState: "ready",
              revision: 11,
              observedAt: "2026-07-26T07:30:00Z",
              tools: {
                declaration: "supported",
                inventory: "complete",
                currentCount: 1,
                currentAvailable: true,
                lastError: null,
              },
              prompts: {
                declaration: "unsupported",
                inventory: "complete",
                currentCount: 0,
                currentAvailable: false,
                lastError: null,
              },
              resources: {
                declaration: "unsupported",
                inventory: "complete",
                currentCount: 0,
                currentAvailable: false,
                lastError: null,
              },
              resourceTemplates: {
                declaration: "unsupported",
                inventory: "complete",
                currentCount: 0,
                currentAvailable: false,
                lastError: null,
              },
            },
          }),
        );
      case "/api/mcp/servers/capabilities/lists":
      case "/api/mcp/servers/tools":
      case "/api/mcp/servers/prompts":
      case "/api/mcp/servers/resources":
      case "/api/mcp/servers/resources/templates":
        capabilityListRequests.push(url.pathname);
        return route.fulfill(
          ok({
            items: [{ id: "tool-sequential-thinking", name: "sequentialthinking" }],
          }),
        );
      case "/api/audit/events":
        return route.fulfill(
          ok({
            events: [],
            total: 0,
            next_cursor: null,
          }),
        );
      default:
        return route.fulfill(ok({}));
    }
  });

  await page.goto("/servers/server-sequential-thinking");

  await expect(
    page.getByRole("heading", { name: "Sequential Thinking" }),
  ).toBeVisible();
  await expect(
    page.getByRole("tab", { name: "Capabilities (1)" }),
  ).toBeVisible();
  await expect(capabilityListRequests).toEqual([]);
  await expect(page.getByText("Capabilities", { exact: true })).toHaveCount(0);
  await expect(
    page.getByText(
      "Tools 1 · Ready | Prompts 0 · Unsupported | Resources 0 · Unsupported | Resource Templates 0 · Unsupported",
      { exact: true },
    ),
  ).toHaveCount(0);
});

test("Refresh observes the complete catalog once and reloads cache-first lists", async ({
  page,
}) => {
  const refreshRequests: Array<{ method: string; body: unknown }> = [];
  const capabilityListRequests: string[] = [];

  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    switch (url.pathname) {
      case "/api/system/readiness":
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ type: "ready", status: "ok" }),
        });
      case "/api/mcp/servers/details":
        return route.fulfill(
          ok({
            id: "server-everything",
            name: "everything",
            status: "connected",
            enabled: true,
            server_type: "stdio",
            server_info: {
              name: "everything-server",
              title: "Everything Reference Server",
              version: "1.0.0",
            },
            instances: [],
            source_revision_set: {
              "server-everything": 17,
            },
            capability: {
              snapshotState: "ready",
              revision: 17,
              observedAt: "2026-07-26T08:09:00Z",
              tools: {
                declaration: "supported",
                inventory: "complete",
                currentCount: 1,
                currentAvailable: true,
                lastError: null,
              },
              prompts: {
                declaration: "supported",
                inventory: "complete",
                currentCount: 0,
                currentAvailable: true,
                lastError: null,
              },
              resources: {
                declaration: "supported",
                inventory: "complete",
                currentCount: 0,
                currentAvailable: true,
                lastError: null,
              },
              resourceTemplates: {
                declaration: "supported",
                inventory: "complete",
                currentCount: 0,
                currentAvailable: true,
                lastError: null,
              },
            },
          }),
        );
      case "/api/mcp/servers/capabilities/lists":
        capabilityListRequests.push(url.search);
        return route.fulfill(
          ok({
            tools: {
              items: [{ id: "tool-echo", name: "echo" }],
            },
            resources: { items: [] },
            prompts: { items: [] },
            resource_templates: { items: [] },
          }),
        );
      case "/api/mcp/servers/capabilities/refresh":
        refreshRequests.push({
          method: request.method(),
          body: request.postDataJSON(),
        });
        return route.fulfill(
          ok({
            server_id: "server-everything",
            catalog_revision: 17,
            catalog_changed: false,
          }),
        );
      case "/api/audit/events":
        return route.fulfill(
          ok({
            events: [],
            total: 0,
            next_cursor: null,
          }),
        );
      default:
        return route.fulfill(ok({}));
    }
  });

  await page.goto("/servers/server-everything?tab=capabilities");
  await expect(
    page.getByRole("heading", { name: "Everything Reference Server" }),
  ).toBeVisible();
  await expect.poll(() => capabilityListRequests.length).toBe(1);
  capabilityListRequests.length = 0;

  await page.getByRole("button", { name: "Refresh", exact: true }).click();

  await expect.poll(() => refreshRequests.length).toBe(1);
  expect(refreshRequests).toEqual([
    {
      method: "POST",
      body: { id: "server-everything" },
    },
  ]);
  await expect.poll(() => capabilityListRequests.length).toBe(1);
  expect(
    capabilityListRequests.every(
      (search) =>
        !new URLSearchParams(search).has("refresh") ||
        new URLSearchParams(search).get("refresh") === "auto",
    ),
  ).toBe(true);
});
