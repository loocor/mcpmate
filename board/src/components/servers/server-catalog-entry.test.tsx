import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

import "../../lib/i18n/index";
import { ServerCatalogEntry } from "./server-catalog-entry";

const statsLabels = {
	tools: "Tools",
	prompts: "Prompts",
	resources: "Resources",
	templates: "Templates",
};

test("list entries keep the enable switch without the legacy inspect action", () => {
	const legacyDebugProps = {
		enableServerDebug: true,
		onOpenDebug: () => undefined,
	};
	const markup = renderToStaticMarkup(
		<ServerCatalogEntry
			{...legacyDebugProps}
			variant="list"
			server={{
				id: "server-one",
				name: "Server One",
				status: "Ready",
				enabled: true,
			}}
			statsLabels={statsLabels}
			onOpen={() => undefined}
			onToggle={() => undefined}
			isToggleDisabled={false}
		/>,
	);

	expect(markup).toContain('role="switch"');
	expect(markup).not.toContain("Open inspect view");
});

test("grid entries elevate OAuth reauthorize warning into the primary status slot", () => {
	const markup = renderToStaticMarkup(
		<ServerCatalogEntry
			variant="grid"
			server={{
				id: "sentry",
				name: "Sentry",
				server_type: "streamable_http",
				status: "Disconnected",
				enabled: false,
				auth_mode: "oauth",
				oauth_status: "expired",
				url: "https://mcp.sentry.dev/mcp",
				capability: {
					snapshotState: "ready",
					revision: 1,
					observedAt: "2026-08-09T00:00:00.000Z",
					tools: {
						declaration: "supported",
						inventory: "complete",
						currentAvailable: true,
						currentCount: 9,
					},
					prompts: {
						declaration: "unsupported",
						inventory: "unknown",
						currentAvailable: false,
						currentCount: 0,
					},
					resources: {
						declaration: "unsupported",
						inventory: "unknown",
						currentAvailable: false,
						currentCount: 0,
					},
					resourceTemplates: {
						declaration: "unsupported",
						inventory: "unknown",
						currentAvailable: false,
						currentCount: 0,
					},
				},
			}}
			statsLabels={statsLabels}
			onOpen={() => undefined}
			onToggle={() => undefined}
			isToggleDisabled={false}
		/>,
	);

	expect(markup).toContain("Reauthorize required");
	expect(markup).not.toContain("Disconnected");
	expect(markup).toContain("Tools");
	expect(markup).toContain("9 ·");
	expect(markup).not.toContain("animate-pulse");
});

test("list entries elevate OAuth reauthorize warning instead of disconnected status", () => {
	const markup = renderToStaticMarkup(
		<ServerCatalogEntry
			variant="list"
			server={{
				id: "sentry",
				name: "Sentry",
				server_type: "streamable_http",
				status: "Disconnected",
				enabled: false,
				auth_mode: "oauth",
				oauth_status: "disconnected",
			}}
			statsLabels={statsLabels}
			onOpen={() => undefined}
			onToggle={() => undefined}
			isToggleDisabled={false}
		/>,
	);

	expect(markup).toContain("Reauthorize required");
	expect(markup).not.toContain("Disconnected");
});

test("grid entries prefer unrecognized transport draft over stdio compatibility projection", () => {
	const markup = renderToStaticMarkup(
		<ServerCatalogEntry
			variant="grid"
			server={{
				id: "uat_unknown_transport",
				name: "Uat Unknown Transport",
				server_type: "stdio",
				status: "Disconnected",
				enabled: false,
				transport_validity: {
					state: "invalid",
					diagnostics: [
						{ code: "transport_unrecognized", field: "transport" },
					],
					draft: { kind: "unrecognized", declared_type: "websocket" },
				},
			}}
			statsLabels={statsLabels}
			onOpen={() => undefined}
			onToggle={() => undefined}
			isToggleDisabled={false}
		/>,
	);

	expect(markup).toContain(">Unknown</span>");
	expect(markup).toContain("Repair required");
	expect(markup).toContain("border-red-200");
	expect(markup).not.toContain("bg-amber-500");
	expect(markup).not.toContain(">STDIO</");
	expect(markup).not.toContain("stdio://");
});
