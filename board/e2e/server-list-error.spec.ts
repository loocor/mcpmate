import { expect, test } from "@playwright/test";

test("Server list failures use notification center without inline diagnostics", async ({
	page,
}) => {
	let serverListRequests = 0;
	const serverDiagnostics: string[] = [];
	page.on("console", (message) => {
		const text = message.text();
		if (
			text.includes("Fetching servers") ||
			text.includes("Servers fetched") ||
			text.includes("Error fetching servers")
		) {
			serverDiagnostics.push(text);
		}
	});
	await page.addInitScript(() => {
		window.localStorage.removeItem("mcp_notifications");
	});

	await page.route("**/api/**", async (route) => {
		const url = new URL(route.request().url());
		if (url.pathname === "/api/system/readiness") {
			return route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify({ type: "ready", status: "ok" }),
			});
		}

		if (url.pathname === "/api/mcp/servers/list") {
			serverListRequests += 1;
			return route.fulfill({
				status: 500,
				contentType: "application/json",
				body: JSON.stringify({ message: "server list unavailable" }),
			});
		}

		return route.fulfill({
			status: 200,
			contentType: "application/json",
			body: JSON.stringify({ success: true, data: {} }),
		});
	});

	await page.goto("/servers");
	await expect.poll(() => serverListRequests).toBeGreaterThan(1);
	await expect.poll(() => serverDiagnostics.length).toBe(0);

	await expect(page.getByRole("alert")).toHaveCount(0);
	const notificationMenu = page.getByRole("menu", { name: "Notifications" });
	const loadFailureNotification = notificationMenu.getByText(
		"Failed to load servers",
		{ exact: true },
	);
	await expect(loadFailureNotification).toBeVisible();
	await expect(
		notificationMenu.getByText("server list unavailable", { exact: true }),
	).toBeVisible();
	await expect(page.getByText("Inspect Details", { exact: true })).toHaveCount(0);
	await expect(
		page.getByRole("button", { name: /^(Inspect|Hide Inspect)$/ }),
	).toHaveCount(0);

	await page.keyboard.press("Escape");
	const requestsBeforeRefresh = serverListRequests;
	await page.getByRole("button", { name: "Refresh" }).click();
	await expect.poll(() => serverListRequests).toBeGreaterThan(requestsBeforeRefresh);
	await page.getByRole("button", { name: "Notifications" }).click();
	await expect(loadFailureNotification).toHaveCount(1);
});
