import { expect, test, type Page } from "@playwright/test";

function ok(data: unknown) {
	return {
		status: 200,
		contentType: "application/json",
		body: JSON.stringify({ success: true, data }),
	};
}

function serverEntries(count: number) {
	return Array.from({ length: count }, (_, index) => ({
		name: `Server ${String(index + 1).padStart(2, "0")}`,
		transport: "stdio",
		args: [],
		env: {},
		headers: {},
	}));
}

async function currentServersScrollMetrics(page: Page) {
	const currentServersCard = page
		.getByText("Current Servers", { exact: true })
		.locator("xpath=ancestor::div[contains(@class, 'rounded-xl')][1]");
	const scrollRegion = currentServersCard.locator(".overflow-y-auto");
	await expect(scrollRegion).toHaveCount(1);
	return scrollRegion.evaluate((element) => ({
		clientHeight: element.clientHeight,
		scrollHeight: element.scrollHeight,
	}));
}

let configuredServerEntries = serverEntries(1);

test.beforeEach(async ({ page }) => {
	configuredServerEntries = serverEntries(1);
	await page.setViewportSize({ width: 1280, height: 720 });
	await page.route("**/api/**", async (route) => {
		const url = new URL(route.request().url());
		switch (url.pathname) {
			case "/api/system/readiness":
				return route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify({ type: "ready", status: "ok" }),
				});
			case "/api/client/list":
				return route.fulfill(
					ok({
						total: 1,
						last_updated: "2026-08-04T00:00:00Z",
						client: [
							{
								identifier: "client-overflow",
								display_name: "Overflow Client",
								detected: true,
								approval_status: "approved",
								writable_config: false,
								attachment_state: "not_applicable",
							},
						],
					}),
				);
			case "/api/client/config/details":
				return route.fulfill(
					ok({
						config_exists: true,
						config_path: "/tmp/client.json",
						content: {},
						has_mcp_config: true,
						configured_server_entries: configuredServerEntries,
						mcp_servers_count: configuredServerEntries.length,
						approval_status: "approved",
						attachment_state: "not_applicable",
						writable_config: false,
						template_merge_strategy: "deep_merge",
						effective_merge_strategy: "deep_merge",
						merge_strategy_source: "template",
						supported_merge_strategies: ["deep_merge"],
						template: {},
					}),
				);
			case "/api/mcp/servers/list":
				return route.fulfill(ok({ servers: [] }));
			default:
				return route.fulfill(ok({}));
		}
	});
});

test("keeps a short current-servers list at its natural height", async ({ page }) => {
	await page.goto("/clients/client-overflow");
	await expect(page.getByText("Server 01", { exact: true })).toBeVisible();

	const metrics = await currentServersScrollMetrics(page);
	expect(metrics.scrollHeight).toBe(metrics.clientHeight);
});

test("scrolls a long current-servers list inside its overview card", async ({
	page,
}) => {
	configuredServerEntries = serverEntries(24);
	await page.goto("/clients/client-overflow");
	await expect(page.getByText("Server 24", { exact: true })).toBeAttached();

	const metrics = await currentServersScrollMetrics(page);
	expect(metrics.scrollHeight).toBeGreaterThan(metrics.clientHeight);
});
