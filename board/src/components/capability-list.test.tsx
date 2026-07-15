import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

import "../lib/i18n/index";
import { CapabilityList } from "./capability-list";

function renderCapability(
	kind: "tools" | "resources" | "prompts" | "templates",
	item: Record<string, unknown>,
) {
	return renderToStaticMarkup(
		<CapabilityList
			asCard={false}
			clickToToggleDetails={false}
			context="server"
			items={[item]}
			kind={kind}
		/>,
	);
}

test("uses the upstream tool name as the hover title", () => {
	const markup = renderCapability("tools", {
		unique_name: "searxng_get_status",
		tool_name: "get_searxng_status",
	});

	expect(markup).toContain("searxng_get_status");
	expect(markup).toContain('title="get_searxng_status"');
});

test("uses upstream identifiers as hover titles for non-tool capabilities", () => {
	const cases = [
		{
			kind: "prompts" as const,
			item: {
				unique_name: "docs_summary",
				prompt_name: "summary",
			},
			upstream: "summary",
		},
		{
			kind: "resources" as const,
			item: {
				unique_uri: "docs:file:///guide.md",
				resource_uri: "file:///guide.md",
			},
			upstream: "file:///guide.md",
		},
		{
			kind: "templates" as const,
			item: {
				unique_uri_template: "docs_file:///{path}",
				uri_template: "file:///{path}",
			},
			upstream: "file:///{path}",
		},
	];

	for (const { kind, item, upstream } of cases) {
		expect(renderCapability(kind, item)).toContain(`title="${upstream}"`);
	}
});
