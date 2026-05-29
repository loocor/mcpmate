import { expect, test, type Page } from "@playwright/test";

async function installReadyApiMocks(page: Page): Promise<void> {
	await page.route("**/api/**", async (route) => {
		const request = route.request();
		const url = new URL(request.url());
		const { pathname } = url;

		const ok = (data: unknown) =>
			route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify(data),
			});

		switch (pathname) {
			case "/api/system/readiness":
				return ok({ type: "ready", status: "ok" });

			case "/api/onboarding/status":
				return ok({
					success: true,
					data: {
						completed: true,
						servers_count: 3,
						clients_count: 2,
					},
				});

			case "/api/system/status":
				return ok({
					status: "running",
					uptime: 3661,
					version: "test",
				});

			case "/api/system/metrics":
				return ok({
					timestamp: new Date().toISOString(),
					cpu_usage_percent: 8,
					memory_usage_bytes: 128_000_000,
					system_memory_total: 1_000_000_000,
					total_requests_mcp: 42,
				});

			case "/api/mcp/profile/list":
				return ok({
					success: true,
					data: {
						total: 2,
						timestamp: new Date().toISOString(),
						profile: [
							{
								id: "default",
								name: "Default",
								description: "Primary operator profile",
								suit_type: "default",
								multi_select: true,
								priority: 0,
								is_active: true,
								is_default: true,
								allowed_operations: [],
							},
							{
								id: "research",
								name: "Research",
								description: "Focused research tools",
								suit_type: "custom",
								multi_select: true,
								priority: 1,
								is_active: false,
								is_default: false,
								allowed_operations: [],
							},
						],
					},
				});

			case "/api/client/list":
				return ok({
					success: true,
					data: {
						total: 2,
						last_updated: new Date().toISOString(),
						client: [
							{
								identifier: "claude-desktop",
								display_name: "Claude Desktop",
								approval_status: "approved",
								enabled: true,
								detected: true,
								config_path: "/tmp/claude.json",
								config_exists: true,
								has_mcp_config: true,
								category: "desktop",
								name: "claude-desktop",
								transport: "stdio",
								args: [],
								env: {},
								headers: {},
							},
							{
								identifier: "cursor",
								display_name: "Cursor",
								approval_status: "pending",
								enabled: true,
								detected: true,
								config_path: "/tmp/cursor.json",
								config_exists: true,
								has_mcp_config: false,
								category: "desktop",
								name: "cursor",
								transport: "stdio",
								args: [],
								env: {},
								headers: {},
							},
						],
					},
				});

			case "/api/mcp/servers/list":
				return ok({
					success: true,
					data: {
						servers: [
							{
								id: "filesystem",
								name: "filesystem",
								status: "connected",
								enabled: true,
								capability: {
									supports_tools: true,
									supports_prompts: false,
									supports_resources: true,
									tools_count: 6,
									prompts_count: 0,
									resources_count: 2,
									resource_templates_count: 0,
								},
							},
							{
								id: "github",
								name: "github",
								status: "initializing",
								enabled: true,
							},
							{
								id: "legacy",
								name: "legacy",
								status: "error",
								enabled: false,
							},
						],
					},
				});

			case "/api/audit/events":
				return ok({
					success: true,
					data: {
						events: [
							{
								id: 1,
								timestamp: new Date().toISOString(),
								category: "mcp",
								action: "tools/list",
								status: "success",
								server_id: "filesystem",
								client_id: "claude-desktop",
							},
						],
						next_cursor: null,
					},
				});

			default:
				return ok({ success: true, data: {} });
		}
	});
}

test("legacy express app mode does not replace the Dashboard", async ({
	page,
}) => {
	await installReadyApiMocks(page);
	await page.addInitScript(() => {
		window.localStorage.setItem(
			"mcp_dashboard_settings",
			JSON.stringify({
				appMode: "express",
				defaultView: "grid",
				language: "en",
			}),
		);
	});

	await page.goto("/");

	await expect(page.getByText("System Status")).toBeVisible();
	await expect(page.getByText("Operator Panel")).toHaveCount(0);
});

test("operator route renders a tray panel surface outside the Full Board shell", async ({
	page,
}) => {
	await installReadyApiMocks(page);

	await page.goto("/operator");

	await expect(page.getByText("Operator Panel")).toBeVisible();
	await expect(page.getByRole("button", { name: "Clients", exact: true })).toBeVisible();
	await expect(page.getByRole("button", { name: "Servers", exact: true })).toBeVisible();
	await expect(page.getByRole("link", { name: "Dashboard" })).toHaveCount(0);
	await expect(page.getByText("System Status")).toHaveCount(0);

	await page.getByRole("button", { name: "Servers", exact: true }).click();
	const details = page.getByTestId("operator-inline-detail");
	await expect(details.getByText("3 servers installed")).toBeVisible();
	await expect(details.getByText("1 connected · 1 needs attention")).toBeVisible();
});
