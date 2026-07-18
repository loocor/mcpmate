import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

import "../lib/i18n/index";
import { useAppStore } from "../lib/store";
import {
	mergeCapabilityInspectorItem,
	resolveCapabilityRawPayload,
} from "../lib/capability-detail";
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
				unique_uri: "mcpmate://resources/docs/file/guide.md",
				resource_uri: "file:///guide.md",
			},
			upstream: "file:///guide.md",
		},
		{
			kind: "templates" as const,
			item: {
				unique_uri_template: "mcpmate://resources/template/docs/file/{path}",
				uri_template: "file:///{path}",
			},
			upstream: "file:///{path}",
		},
	];

	for (const { kind, item, upstream } of cases) {
		expect(renderCapability(kind, item)).toContain(`title="${upstream}"`);
	}
});

test("does not expose management DTO fields as raw MCP details", () => {
	useAppStore
		.getState()
		.setDashboardSetting("showRawCapabilityJson", true);

	try {
		const markup = renderToStaticMarkup(
			<CapabilityList
				asCard={false}
				clickToToggleDetails={false}
				context="server"
				items={[
					{
						id: "tool-id",
						name: "everything_get-tiny-image",
						tool_name: "get-tiny-image",
						unique_name: "everything_get-tiny-image",
						__serverCapabilityKind: "tools",
					},
				]}
				kind="tools"
				loadDetails={async () => ({
					name: "everything_get-tiny-image",
					inputSchema: { type: "object" },
				})}
			/>,
		);

		expect(markup).not.toContain("tool-id");
		expect(markup).not.toContain("__serverCapabilityKind");
		expect(markup).not.toContain("unique_name");
	} finally {
		useAppStore
			.getState()
			.setDashboardSetting("showRawCapabilityJson", false);
	}
});

test("uses the protocol detail payload as raw JSON when lazy details are enabled", () => {
	const managementItem = {
		id: "tool-id",
		tool_name: "get-tiny-image",
		unique_name: "everything_get-tiny-image",
	};
	const protocolDetail = {
		name: "everything_get-tiny-image",
		inputSchema: { type: "object" },
	};

	expect(
		resolveCapabilityRawPayload(managementItem, protocolDetail, true),
	).toEqual(protocolDetail);
	expect(resolveCapabilityRawPayload(managementItem, null, true)).toBeUndefined();
	expect(
		resolveCapabilityRawPayload(managementItem, null, false),
	).toEqual(managementItem);
});

test("preserves routing identities when protocol details open the Inspector", () => {
	const managementItem = {
		id: "template-id",
		uri_template: "demo://resource/dynamic/text/{resourceId}",
		unique_uri_template:
			"mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
	};
	const protocolDetail = {
		name: "Dynamic Text Resource",
		uriTemplate:
			"mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
	};

	expect(
		mergeCapabilityInspectorItem(managementItem, protocolDetail),
	).toEqual({
		...managementItem,
		...protocolDetail,
	});
});
