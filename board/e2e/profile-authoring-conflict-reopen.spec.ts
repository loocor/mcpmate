import { expect, test } from "@playwright/test";

function ok(data: unknown) {
	return {
		status: 200,
		contentType: "application/json",
		body: JSON.stringify({ success: true, data }),
	};
}

function profile(name: string, authoringGeneration: number) {
	return {
		id: "profile-a",
		name,
		description: null,
		profile_type: "shared",
		multi_select: true,
		priority: 50,
		is_active: true,
		is_default: false,
		authoring_generation: authoringGeneration,
		role: "user",
		allowed_operations: ["update", "delete"],
	};
}

test("reopening after a cancelled conflict loads the current Profile baseline", async ({
	page,
}) => {
	let authoringGeneration = 1;
	let authoringName = "Initial Profile";
	let authoringViewRequests = 0;

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
			case "/api/mcp/profile/details":
				return route.fulfill(ok({ profile: profile("Initial Profile", 1) }));
			case "/api/mcp/profile/authoring/view":
				authoringViewRequests += 1;
				return route.fulfill(
					ok({
						profile: profile(authoringName, authoringGeneration),
						server_ids: [],
					}),
				);
			case "/api/mcp/profile/authoring/save":
				authoringGeneration = 2;
				authoringName = "Remote Profile";
				return route.fulfill({
					status: 409,
					contentType: "application/json",
					body: JSON.stringify({
						error: {
							message: "Profile was changed by another author",
							status: 409,
							code: "profile_authoring_changed",
							details: { currentAuthoringGeneration: 2 },
						},
					}),
				});
			case "/api/mcp/profile/servers/list":
				return route.fulfill(
					ok({
						profile_id: "profile-a",
						profile_name: authoringName,
						servers: [],
						authoring_generation: authoringGeneration,
					}),
				);
			case "/api/mcp/profile/tools/list":
			case "/api/mcp/profile/resources/list":
			case "/api/mcp/profile/prompts/list":
			case "/api/mcp/profile/resource-templates/list": {
				const kind = url.pathname.split("/").at(-2);
				return route.fulfill(
					ok({
						profile_id: "profile-a",
						profile_name: authoringName,
						[kind === "resource-templates" ? "templates" : kind ?? "tools"]: [],
						source_revision_set: {},
						authoring_generation: authoringGeneration,
					}),
				);
			}
			case "/api/mcp/profile/list":
				return route.fulfill(
					ok({
						profile: [profile("Initial Profile", 1)],
						total: 1,
						timestamp: "2026-08-07T00:00:00Z",
					}),
				);
			case "/api/mcp/servers/list":
				return route.fulfill(ok({ servers: [] }));
			default:
				return route.fulfill(ok({}));
		}
	});

	await page.goto("/profiles/profile-a");
	await page.getByRole("button", { name: "Edit" }).click();
	const nameInput = page.getByLabel("Name *");
	await expect(nameInput).toHaveValue("Initial Profile");
	await nameInput.fill("Local Draft");
	await page.getByRole("button", { name: "Next" }).click();
	await page.getByRole("button", { name: "Save Changes" }).click();
	await expect(
		page.getByRole("heading", { name: "Profile modified elsewhere" }),
	).toBeVisible();
	await page.getByRole("button", { name: "Cancel" }).click();

	await page.mouse.click(20, 20);
	await expect(page.getByRole("heading", { name: "Edit Profile" })).toBeHidden();
	authoringGeneration = 3;
	authoringName = "Newest Profile";

	await page.getByRole("button", { name: "Edit" }).click();
	await expect(page.getByLabel("Name *")).toHaveValue("Newest Profile");
	expect(authoringViewRequests).toBe(3);
});
