import { expect, test, type Page } from "@playwright/test";

declare global {
	interface Window {
		__TAURI_INTERNALS__?: unknown;
		__MCPMATE_TEST_EMIT_TAURI_EVENT__?: (event: string) => void;
	}
}

type TestDashboardLanguage = "en" | "zh-cn" | "ja";

async function installDashboardLanguage(
	page: Page,
	language: TestDashboardLanguage,
): Promise<void> {
	await page.addInitScript((selectedLanguage) => {
		window.localStorage.setItem("i18nextLng", selectedLanguage);
		window.localStorage.setItem(
			"mcp_dashboard_settings",
			JSON.stringify({
				defaultView: "grid",
				language: selectedLanguage,
			}),
		);
	}, language);
}

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

			case "/api/system/settings":
				return ok({
					success: true,
					data: {
						api_port: 8080,
						mcp_port: 8000,
						api_url: "http://127.0.0.1:8080",
						mcp_http_url: "http://127.0.0.1:8000/mcp",
						first_contact_behavior: "review",
						onboarding_policy: "auto_manage",
						inspector_timeout_ms: 30_000,
						default_config_mode: "unify",
					},
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
								profile_type: "default",
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
								profile_type: "custom",
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
							{
								id: "fetch",
								name: "fetch",
								status: "connected",
								enabled: true,
							},
							{
								id: "search",
								name: "search",
								status: "connected",
								enabled: true,
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
								occurred_at_ms: Date.now() - 60_000,
								category: "management",
								action: "local_core_service_restart",
								status: "success",
							},
							{
								id: 2,
								occurred_at_ms: Date.now() - 120_000,
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

async function installIncompleteOnboardingMocks(page: Page): Promise<void> {
	await installReadyApiMocks(page);
	await page.route("**/api/onboarding/status", (route) =>
		route.fulfill({
			status: 200,
			contentType: "application/json",
			body: JSON.stringify({
				success: true,
				data: {
					completed: false,
					servers_count: 0,
					clients_count: 0,
				},
			}),
		}),
	);
}

async function installOperatorDataErrorMocks(page: Page): Promise<void> {
	await installReadyApiMocks(page);

	for (const path of ["/api/mcp/profile/list", "/api/mcp/servers/list"]) {
		await page.route(`**${path}`, (route) =>
			route.fulfill({
				status: 503,
				contentType: "application/json",
				body: JSON.stringify({
					success: false,
					error: "operator data source unavailable",
				}),
			}),
		);
	}
}

async function installMalformedClientsMock(page: Page): Promise<void> {
	await installReadyApiMocks(page);
	await page.route("**/api/client/list**", (route) =>
		route.fulfill({
			status: 200,
			contentType: "application/json",
			body: JSON.stringify({
				success: true,
				data: {
					total: 2,
					last_updated: new Date().toISOString(),
					client: { identifier: "malformed-client" },
				},
			}),
		}),
	);
}

async function installMalformedProfilesMock(page: Page): Promise<void> {
	await installReadyApiMocks(page);
	await page.route("**/api/mcp/profile/list", (route) =>
		route.fulfill({
			status: 200,
			contentType: "application/json",
			body: JSON.stringify({
				success: true,
				data: {
					total: 1,
					timestamp: new Date().toISOString(),
					profile: [
						{
							id: "legacy-shape",
							name: "Legacy Shape",
							description: "Payload already shaped like ConfigSuit",
							suit_type: "custom",
							multi_select: true,
							priority: 0,
							is_active: true,
							is_default: true,
							allowed_operations: [],
						},
					],
				},
			}),
		}),
	);
}

async function installTauriPendingFullBoardPathMock(
	page: Page,
	path: string | null,
): Promise<void> {
	await page.addInitScript((initialPath) => {
		const listeners = new Map<string, Array<(event: unknown) => void>>();
		let nextCallbackId = 1;
		let nextEventId = 1;
		let pendingFullBoardPath = initialPath;
		const callbacks = new Map<number, (event: unknown) => void>();

		window.__TAURI_INTERNALS__ = {
			transformCallback(callback: (event: unknown) => void) {
				const id = nextCallbackId;
				nextCallbackId += 1;
				callbacks.set(id, callback);
				return id;
			},
			unregisterCallback(id: number) {
				callbacks.delete(id);
			},
			async invoke(command: string, args?: Record<string, unknown>) {
				if (command === "mcp_shell_read_core_source") {
					return {};
				}
				if (command === "mcp_shell_take_pending_full_board_path") {
					const value = pendingFullBoardPath;
					pendingFullBoardPath = null;
					return value;
				}
				if (command === "mcp_shell_close_operator_panel") {
					throw new Error("operator close bridge unavailable in test");
				}
				if (command === "plugin:event|listen") {
					const event = String(args?.event ?? "");
					const handlerId = Number(args?.handler);
					const handler = callbacks.get(handlerId);
					if (handler) {
						const handlers = listeners.get(event) ?? [];
						handlers.push(handler);
						listeners.set(event, handlers);
					}
					const eventId = nextEventId;
					nextEventId += 1;
					return eventId;
				}
				if (command === "plugin:event|unlisten") {
					return null;
				}
				throw new Error(`Unexpected Tauri command: ${command}`);
			},
		};

		window.__MCPMATE_TEST_EMIT_TAURI_EVENT__ = (event: string) => {
			for (const handler of listeners.get(event) ?? []) {
				handler({ event, payload: null });
			}
		};
	}, path);
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
	await expect(
		page.evaluate(() => ({
			html: document.documentElement.getAttribute("data-mcpmate-surface"),
			body: document.body.getAttribute("data-mcpmate-surface"),
			root: document.getElementById("root")?.getAttribute("data-mcpmate-surface") ?? null,
		})),
	).resolves.toEqual({
		html: null,
		body: null,
		root: null,
	});
});

test("operator route renders a tray panel surface outside the Full Board shell", async ({
	page,
}) => {
	await installReadyApiMocks(page);

	await page.goto("/operator");

	await expect(page.getByRole("heading", { name: "MCPMate" })).toBeVisible();
	await expect(page.getByTestId("operator-chart-carousel")).toBeVisible();
	await expect(page.getByTestId("operator-core-hero")).toBeVisible();
	await expect(page.getByTestId("operator-chart-carousel")).toBeVisible();
	await expect(page.getByTestId("operator-chart-carousel-dots")).toBeVisible();
	await expect(page.getByRole("tab", { name: "Go to Metrics" })).toBeVisible();
	await expect(page.getByRole("tab", { name: "Go to Token Savings" })).toBeVisible();
	await expect(page.getByText("Core ready")).toHaveCount(0);
	await expect(page.getByText("Traffic")).toHaveCount(0);
	await expect(page.getByRole("button", { name: "Expand Clients details" })).toBeVisible();
	await expect(page.getByText(/\d+ to review/)).toHaveCount(0);
	await expect(
		page.getByRole("button", { name: "Expand Servers details" }),
	).toBeVisible();
	await expect(page.getByRole("link", { name: "Dashboard" })).toHaveCount(0);
	await expect(page.getByText("System Status")).toHaveCount(0);

	const headerState = await page.locator("header").first().evaluate((header) => {
		const logo = header.querySelector('img[src="/logo.svg"]');
		const fullBoardControl = header.querySelector('[aria-label="Open Full Board"]');

		return {
			headerDrag: header.hasAttribute("data-operator-drag-region"),
			logoPresent: Boolean(logo),
			fullBoardControlPresent: Boolean(fullBoardControl),
		};
	});

	expect(headerState).toEqual({
		headerDrag: true,
		logoPresent: true,
		fullBoardControlPresent: true,
	});

	const surface = await page.evaluate(() => {
		const root = document.getElementById("root");

		return {
			htmlAttr: document.documentElement.getAttribute("data-mcpmate-surface"),
			bodyAttr: document.body.getAttribute("data-mcpmate-surface"),
			rootAttr: root?.getAttribute("data-mcpmate-surface") ?? null,
			htmlBackground: window.getComputedStyle(document.documentElement).backgroundColor,
			bodyBackground: window.getComputedStyle(document.body).backgroundColor,
			rootBackground: root ? window.getComputedStyle(root).backgroundColor : null,
		};
	});

	expect(surface).toEqual({
		htmlAttr: "operator",
		bodyAttr: "operator",
		rootAttr: "operator",
		htmlBackground: "rgba(0, 0, 0, 0)",
		bodyBackground: "rgba(0, 0, 0, 0)",
		rootBackground: "rgba(0, 0, 0, 0)",
	});

	const panelFrameClass = await page
		.locator('[data-operator-panel-frame="true"]')
		.first()
		.getAttribute("class");
	expect(panelFrameClass).toContain("max-w-[420px]");

	const panelShellClass = await page.locator("main").first().getAttribute("class");
	expect(panelShellClass).toContain("rounded-xl");

	await expect(page.getByRole("link", { name: /in Full Board$/ })).toHaveCount(0);

	const coreToggle = page.getByRole("button", { name: "Expand Core details" });
	const coreDetailId = await coreToggle.getAttribute("aria-controls");
	expect(coreDetailId).toBe("operator-row-detail-core");
	await coreToggle.click();
	const coreDetails = page.locator("#operator-row-detail-core");
	await expect(coreDetails).toBeVisible();
	await expect(coreDetails.getByRole("button", { name: "Restart" })).toBeVisible();
	await expect(coreDetails.getByRole("button", { name: "Stop" })).toBeDisabled();
	await expect(coreDetails.getByTestId("operator-core-mcp-endpoint")).toHaveText(
		"http://127.0.0.1:8000/mcp",
	);
	await expect(
		coreDetails.getByRole("button", { name: "Copy MCPMate Server Endpoint" }),
	).toBeEnabled();

	const profilesToggle = page.getByRole("button", { name: "Expand Profiles details" });
	const profilesDetailId = await profilesToggle.getAttribute("aria-controls");
	expect(profilesDetailId).toBe("operator-row-detail-profiles");
	await profilesToggle.click();
	const profilesDetails = page.locator("#operator-row-detail-profiles");
	await expect(profilesDetails).toBeVisible();
	await expect(profilesDetails.getByRole("button", { name: "Activate Research" })).toBeVisible();
	await expect(profilesDetails.getByRole("button", { name: "Deactivate Default" })).toBeVisible();
	await expect(profilesDetails.getByRole("link", { name: "Open Profiles in Full Board" })).toBeVisible();
	await expect(profilesDetails.getByText("More...")).toBeVisible();

	const clientsToggle = page.getByRole("button", { name: "Expand Clients details" });
	const clientsDetailId = await clientsToggle.getAttribute("aria-controls");
	expect(clientsDetailId).toBe("operator-row-detail-clients");
	await clientsToggle.click();
	const clientsDetails = page.locator("#operator-row-detail-clients");
	await expect(clientsDetails).toBeVisible();
	await expect(
		clientsDetails.getByRole("button", { name: "Approve Cursor" }),
	).toBeVisible();
	await expect(
		clientsDetails.getByRole("link", { name: "Open Claude Desktop in Full Board" }),
	).toBeVisible();
	await expect(
		clientsDetails.getByRole("link", { name: "Open Clients in Full Board" }),
	).toBeVisible();

	const serversToggle = page.getByRole("button", { name: "Expand Servers details" });
	const serversDetailId = await serversToggle.getAttribute("aria-controls");
	expect(serversDetailId).toBe("operator-row-detail-servers");
	await serversToggle.click();
	const serversDetails = page.locator("#operator-row-detail-servers");
	await expect(serversDetails).toBeVisible();
	await expect(
		serversDetails.getByRole("link", { name: "Open legacy in Full Board" }),
	).toBeVisible();
	await expect(
		serversDetails.getByRole("link", { name: "Open filesystem in Full Board" }),
	).toBeVisible();
	await expect(serversDetails.getByRole("button", { name: "1 more" })).toBeVisible();
	await expect(
		serversDetails.getByRole("button", { name: /Drop-in\. Drag an MCP server JSON snippet/i }),
	).toBeVisible();

	const activityToggle = page.getByRole("button", { name: "Expand Activity details" });
	const activityDetailId = await activityToggle.getAttribute("aria-controls");
	expect(activityDetailId).toBe("operator-row-detail-activity");
	await activityToggle.click();
	const activityDetails = page.locator("#operator-row-detail-activity");
	await expect(activityDetails).toBeVisible();
	await expect(page.getByTestId("operator-activity-scroll")).toBeVisible();
	await expect(activityDetails.getByText("Restart Local Core")).toBeVisible();
	await expect(activityDetails.getByText("tools/list")).toBeVisible();
	await expect(
		activityDetails.getByRole("link", { name: "Open Logs in Full Board" }),
	).toBeVisible();
});

test("operator route keeps incomplete onboarding out of the tray surface in web preview", async ({
	page,
}) => {
	await installIncompleteOnboardingMocks(page);

	await page.goto("/operator");

	await expect(page.getByRole("heading", { name: "Setup required" })).toBeVisible();
	await expect(page.getByText("Open Full Board setup")).toBeVisible();
	await expect(page.getByText("Runtime requirements")).toHaveCount(0);
});

test("operator route refreshes incomplete onboarding status while gated", async ({
	page,
}) => {
	await installReadyApiMocks(page);
	let statusRequests = 0;
	await page.route("**/api/onboarding/status", (route) => {
		statusRequests += 1;
		const completed = statusRequests > 1;
		return route.fulfill({
			status: 200,
			contentType: "application/json",
			body: JSON.stringify({
				success: true,
				data: {
					completed,
					servers_count: completed ? 3 : 0,
					clients_count: completed ? 2 : 0,
				},
			}),
		});
	});

	await page.goto("/operator");

	await expect(page.getByRole("heading", { name: "Setup required" })).toBeVisible();
	await expect(page.getByRole("heading", { name: "Setup required" })).toHaveCount(0);
	await expect(page.getByTestId("operator-chart-carousel")).toBeVisible();
	expect(statusRequests).toBeGreaterThan(1);
});

test("operator route follows dashboard language storage updates", async ({
	page,
}) => {
	await installReadyApiMocks(page);
	await installDashboardLanguage(page, "en");

	await page.goto("/operator");

	await expect(page.getByText("Operator Panel")).toBeVisible();
	await expect(page.getByText("Profiles")).toBeVisible();
	await expect(page.getByText(/running · .* uptime/)).toBeVisible();

	await page.evaluate(() => {
		const key = "mcp_dashboard_settings";
		const oldValue = window.localStorage.getItem(key);
		const nextValue = JSON.stringify({
			defaultView: "grid",
			language: "zh-cn",
		});
		window.localStorage.setItem(key, nextValue);
		window.dispatchEvent(
			new StorageEvent("storage", {
				key,
				oldValue,
				newValue: nextValue,
				storageArea: window.localStorage,
				url: window.location.href,
			}),
		);
	});

	await expect(page.getByText("操作面板")).toBeVisible();
	await expect(page.getByText("配置集")).toBeVisible();
	await expect(page.getByText(/运行中 · 已运行/)).toBeVisible();
	await expect(page.getByText("Profiles")).toHaveCount(0);
	await expect(page.getByText("Operator Panel")).toHaveCount(0);
	await expect(page.getByText(/running · .* uptime/)).toHaveCount(0);
});

test("operator route places close as the rightmost header control in Tauri shell", async ({
	page,
}) => {
	await installReadyApiMocks(page);
	await installTauriPendingFullBoardPathMock(page, null);
	await installDashboardLanguage(page, "en");

	await page.goto("/operator");
	await expect(page.getByRole("button", { name: "Pin on top" })).toBeVisible();

	const controlLabels = await page
		.locator("header [aria-label]")
		.evaluateAll((elements) =>
			elements
				.map((element) => element.getAttribute("aria-label"))
				.filter((label) =>
					label === "Pin on top" ||
					label === "Open Full Board" ||
					label === "Close",
				),
		);

	expect(controlLabels).toEqual(["Pin on top", "Open Full Board", "Close"]);
});

test("operator route preserves explicit system error status in core meta", async ({
	page,
}) => {
	await installReadyApiMocks(page);
	await page.route("**/api/system/status", (route) =>
		route.fulfill({
			status: 200,
			contentType: "application/json",
			body: JSON.stringify({
				status: "error",
				uptime: 3661,
				version: "test",
			}),
		}),
	);

	await page.goto("/operator");

	await expect(page.getByText(/error · .* uptime/)).toBeVisible();
	await expect(page.getByText(/unknown · .* uptime/)).toHaveCount(0);
});

test("operator route presents query errors instead of empty successful rows", async ({
	page,
}) => {
	await installOperatorDataErrorMocks(page);

	await page.goto("/operator");

	await expect(page.getByText("Profiles are unavailable")).toBeVisible();
	await expect(page.getByText("Servers are unavailable")).toBeVisible();
	await expect(page.getByText("0 active of 0 profiles")).toHaveCount(0);
	await expect(page.getByText("0 servers installed")).toHaveCount(0);
});

test("operator route presents malformed successful clients payload as an error", async ({
	page,
}) => {
	await installMalformedClientsMock(page);

	await page.goto("/operator");

	await expect(page.getByText("Clients are unavailable")).toBeVisible();
	await expect(page.getByText("0 clients detected")).toHaveCount(0);
});

test("operator route rejects profile payloads without profile_type", async ({
	page,
}) => {
	await installMalformedProfilesMock(page);

	await page.goto("/operator");

	await expect(page.getByText("Profiles are unavailable")).toBeVisible();
	await expect(page.getByText("1 active of 1 profiles")).toHaveCount(0);
});

test("operator route ignores Esc outside the desktop shell", async ({ page }) => {
	await installReadyApiMocks(page);

	await page.goto("/operator");
	await expect(page.getByText("Operator Panel")).toBeVisible();
	await page.keyboard.press("Escape");

	await expect(page.getByText(/Desktop action failed:/)).toHaveCount(0);
});

test("operator route maps Esc to the desktop close bridge in Tauri shell", async ({ page }) => {
	await installReadyApiMocks(page);
	await installTauriPendingFullBoardPathMock(page, null);

	await page.goto("/operator");
	await expect(page.getByTestId("operator-chart-carousel")).toBeVisible();
	await page.keyboard.press("Escape");

	await expect(page.getByText(/Desktop action failed:/)).toBeVisible();
});

test("full board path delivery is handled outside the Layout route", async ({ page }) => {
	await installReadyApiMocks(page);
	await installTauriPendingFullBoardPathMock(page, "/clients");
	await page.addInitScript(() => {
		window.localStorage.removeItem("mcp_dashboard_settings");
	});

	await page.goto("/onboarding");

	await expect(page).toHaveURL(/\/clients$/);
});
