import { expect, test } from "@playwright/test";

function ok(data: unknown) {
  return {
    status: 200,
    contentType: "application/json",
    body: JSON.stringify({ success: true, data }),
  };
}

const reviewItem = {
  review_item_id: "review-a",
  proposal_id: "proposal-a",
  consumer_id: "client-a",
  binding_generation: 3,
  ref_id: "ref-a",
  before_capability_id: "capability-before",
  target_capability_id: "capability-target",
  target_key: "capability:capability-target",
  change_class: "model_visible",
  policy_action: "review",
  lifecycle: "pending",
  owners: [
    {
      owner_type: "consumer_direct_exposure",
      owner_id: "client-a",
    },
  ],
  before_record: {
    source: {
      serverId: "server-a",
    },
    name: "weather",
    description: "Old description",
  },
  target_record: {
    source: {
      serverId: "server-a",
    },
    name: "weather",
    description: "New description",
  },
  field_diff: [
    {
      path: "/description",
      before: "Old description",
      target: "New description",
    },
  ],
  created_at: "2026-07-25T00:00:00Z",
  updated_at: "2026-07-25T00:00:00Z",
};

test("Dashboard exposes pending Surface review todos at their owning config", async ({
  page,
}) => {
  await page.route("**/api/**", async (route) => {
    const url = new URL(route.request().url());
    switch (url.pathname) {
      case "/api/system/readiness":
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify({ type: "ready", status: "ok" }),
        });
      case "/api/client/surface/reviews":
        return route.fulfill(ok({ items: [reviewItem] }));
      case "/api/client/surface/reviews/summary":
        return route.fulfill(
          ok({
            pending_count: 1,
            failed_reconciliation_count: 1,
            entries: [],
          }),
        );
      case "/api/client/surface/reviews/review-a":
        return route.fulfill(ok(reviewItem));
      case "/api/client/capability-config":
        return route.fulfill(
          ok({
            identifier: "client-a",
            capability_source: "activated",
            selected_profile_ids: [],
            source_revision_set: { "server-a": 2 },
            unify_direct_exposure: {
              route_mode: "capability_level",
              capability_refs: { tool_refs: ["ref-a"] },
            },
          }),
        );
      case "/api/mcp/servers/details":
        return route.fulfill(
          ok({
            id: "server-a",
            name: "Server A",
            server_type: "stdio",
            status: "connected",
          }),
        );
      case "/api/mcp/servers/capabilities/lists":
        return route.fulfill(
          ok({
            tools: { items: [{ id: "ref-a", name: "weather" }] },
            resources: { items: [] },
            prompts: { items: [] },
            resource_templates: { items: [] },
          }),
        );
      case "/api/mcp/servers/tools":
        return route.fulfill(
          ok({ items: [{ id: "ref-a", name: "weather" }] }),
        );
      case "/api/mcp/servers/prompts":
      case "/api/mcp/servers/resources":
      case "/api/mcp/servers/resources/templates":
        return route.fulfill(ok({ items: [] }));
      case "/api/client/list":
        return route.fulfill(
          ok({
            client: [
              {
                identifier: "client-a",
                display_name: "Client A",
                category: "terminal",
                enabled: true,
                detected: true,
                config_path: "",
                config_exists: false,
                has_mcp_config: false,
                template: {
                  format: "json",
                  container_type: "json",
                  merge_strategy: "merge",
                  keep_original_config: true,
                  storage: { kind: "file" },
                },
              },
            ],
            total: 1,
            last_updated: "2026-07-25T00:00:00Z",
          }),
        );
      case "/api/mcp/profile/list":
        return route.fulfill(
          ok({ profile: [], total: 0, timestamp: "2026-07-25T00:00:00Z" }),
        );
      case "/api/server/list":
        return route.fulfill(ok({ servers: [] }));
      case "/api/system/status":
        return route.fulfill(ok({ status: "running", uptime: 1 }));
      default:
        return route.fulfill(ok({}));
    }
  });

  await page.goto("/");

  const todo = page.getByRole("link", { name: /client-a/i });
  await expect(todo).toBeVisible();
  await expect(
    page.getByText(/1 Surface reconciliation jobs failed/i),
  ).toBeVisible();
  await expect(todo).toHaveAttribute(
    "href",
    "/clients/client-a/direct/server-a?review_item=review-a&ref_id=ref-a",
  );
  await todo.click();

  await expect(
    page.getByRole("heading", { name: "Capability change review" }),
  ).toBeVisible();
  await expect(page.getByText("/description", { exact: true })).toBeVisible();
  await expect(page.getByText("Old description", { exact: true })).toBeVisible();
  await expect(page.getByText("New description", { exact: true })).toBeVisible();
});
