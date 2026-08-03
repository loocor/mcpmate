import { expect, test, type Page } from "@playwright/test";

function ok(data: unknown) {
	return {
		status: 200,
		contentType: "application/json",
		body: JSON.stringify({ success: true, data }),
	};
}

async function expectPaginationControlsDisabled(page: Page): Promise<void> {
	for (const name of ["First", "Previous", "Next", "Last"]) {
		await expect(
			page.getByRole("button", { name, exact: true }),
		).toBeDisabled();
	}
}

const servers = Array.from({ length: 10 }, (_, index) => {
	const number = String(index + 1).padStart(2, "0");
	return {
		id: `server-${number}`,
		name: `Server ${number}`,
		status: "Ready",
		server_type: "stdio",
		enabled: true,
		instances: [{ id: `instance-${number}`, name: "default", status: "Ready" }],
	};
});

const clients = Array.from({ length: 10 }, (_, index) => {
	const number = String(index + 1).padStart(2, "0");
	return {
		identifier: `client-${number}`,
		display_name: `Client ${number}`,
		description: `Client fixture ${number}`,
		detected: true,
		approval_status: "allowed",
	};
});
const reviewItems = clients.map((client, index) => ({
	review_item_id: `review-${index + 1}`,
	owners: [
		{
			owner_type: "consumer_direct_exposure",
			owner_id: client.identifier,
		},
	],
}));

let serverFixtures = servers;
let clientFixtures = clients;
let reviewFixtures: typeof reviewItems = [];
let holdReviewResponse = false;
let releaseReviewResponse: (() => void) | null = null;

test.beforeEach(async ({ page }) => {
	serverFixtures = servers;
	clientFixtures = clients;
	reviewFixtures = [];
	holdReviewResponse = false;
	releaseReviewResponse = null;
	await page.setViewportSize({ width: 1024, height: 720 });
	await page.route("**/api/**", async (route) => {
		const url = new URL(route.request().url());
		switch (url.pathname) {
			case "/api/system/readiness":
				return route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify({ type: "ready", status: "ok" }),
				});
			case "/api/mcp/servers/list":
				return route.fulfill(ok({ servers: serverFixtures }));
			case "/api/client/list":
				return route.fulfill(
					ok({
						total: clientFixtures.length,
						last_updated: "2026-08-03T00:00:00Z",
						client: clientFixtures,
					}),
				);
			case "/api/client/surface/reviews":
				if (holdReviewResponse) {
					await new Promise<void>((resolve) => {
						releaseReviewResponse = resolve;
					});
				}
				return route.fulfill(ok({ items: reviewFixtures }));
			default:
				return route.fulfill(ok({}));
		}
	});
});

test("Servers paginate responsive grid results and reset after search", async ({
	page,
}) => {
	await page.goto("/servers?view=grid&page=2");
	await expect(page).toHaveURL(/page=2/);
	await expect(page.getByText("Server 07", { exact: true })).toBeVisible();

	await page.goto("/servers?view=grid");

	await expect(page.getByText("Server 01", { exact: true })).toBeVisible();
	await expect(page.getByText("Server 06", { exact: true })).toBeVisible();
	await expect(page.getByText("Server 07", { exact: true })).toHaveCount(0);

	await page.getByRole("button", { name: "Next" }).click();
	await expect(page).toHaveURL(/page=2/);
	await expect(page.getByText("Server 07", { exact: true })).toBeVisible();
	await expect(page.getByText("Server 01", { exact: true })).toHaveCount(0);

	await page.getByPlaceholder("Search servers...").fill("Server");
	await expect(page).not.toHaveURL(/page=/);
	await expect(page.getByText("Server 01", { exact: true })).toBeVisible();
	await expect(page.getByText("Server 07", { exact: true })).toHaveCount(0);

	await page.getByRole("button", { name: "Next" }).click();
	await expect(page).toHaveURL(/page=2/);
	await page.getByRole("combobox", { name: "Per page", exact: true }).click();
	await page.getByRole("option", { name: "3", exact: true }).click();
	await expect(page).not.toHaveURL(/page=/);
	await page.getByRole("button", { name: "Next", exact: true }).click();
	await expect(page).toHaveURL(/page=2/);
	await page.setViewportSize({ width: 1280, height: 720 });
	await expect
		.poll(() =>
			page.evaluate(() => ({
				innerWidth: window.innerWidth,
				matches: window.matchMedia("(min-width: 1280px)").matches,
			})),
		)
		.toEqual({ innerWidth: 1280, matches: true });
	await page.evaluate(() => window.dispatchEvent(new Event("resize")));
	await expect(page).not.toHaveURL(/page=/);
	await expect(page.getByText("Server 01", { exact: true })).toBeVisible();
	await expect(page.getByText("Server 04", { exact: true })).toHaveCount(0);

	await page.goto("/servers?view=list");
	await expect(page.getByText("Server 01", { exact: true })).toBeVisible();
	await expect(
		page.getByRole("button", { name: "Open inspect view" }),
	).toHaveCount(0);

	await page.goto("/servers?view=grid&page=invalid");
	await expect(page).not.toHaveURL(/page=/);
	await expect(page.getByText("Server 01", { exact: true })).toBeVisible();
});

test("Clients paginate responsive grid results and reset after search", async ({
	page,
}) => {
	await page.goto("/clients?view=grid&page=2");
	await expect(page).toHaveURL(/page=2/);
	await expect(page.getByText("Client 07", { exact: true })).toBeVisible();

	await page.goto("/clients?view=grid");

	await expect(page.getByText("Client 01", { exact: true })).toBeVisible();
	await expect(page.getByText("Client 06", { exact: true })).toBeVisible();
	await expect(page.getByText("Client 07", { exact: true })).toHaveCount(0);

	await page.getByRole("button", { name: "Next" }).click();
	await expect(page).toHaveURL(/page=2/);
	await expect(page.getByText("Client 07", { exact: true })).toBeVisible();

	await page.getByPlaceholder("Search clients...").fill("Client");
	await expect(page).not.toHaveURL(/page=/);
	await expect(page.getByText("Client 01", { exact: true })).toBeVisible();
	await expect(page.getByText("Client 07", { exact: true })).toHaveCount(0);
});

test("empty and zero-match catalogs normalize stale pages", async ({ page }) => {
	serverFixtures = [];
	await page.goto("/servers?view=grid&page=2");
	await expect(page).not.toHaveURL(/page=/);
	await expect(page.getByText("No servers found", { exact: true })).toBeVisible();

	serverFixtures = servers;
	await page.goto("/servers?view=grid&q=NoMatch&page=2");
	await expect(page).not.toHaveURL(/page=/);
	await expect(page.getByText("No servers found", { exact: true })).toBeVisible();

	clientFixtures = [];
	await page.goto("/clients?view=grid&page=2");
	await expect(page).not.toHaveURL(/page=/);
	await expect(page.getByText("No clients found", { exact: true })).toBeVisible();
	await expectPaginationControlsDisabled(page);
});

test("needs-review deep links wait for review data before clamping", async ({
	page,
}) => {
	reviewFixtures = reviewItems;
	holdReviewResponse = true;
	await page.goto("/clients?view=grid&filter=needs_review&page=2");

	await expect.poll(() => releaseReviewResponse !== null).toBe(true);
	await expect(page).toHaveURL(/page=2/);
	await expectPaginationControlsDisabled(page);
	releaseReviewResponse?.();

	await expect(page.getByText("Client 07", { exact: true })).toBeVisible();
	await expect(page).toHaveURL(/page=2/);
});
