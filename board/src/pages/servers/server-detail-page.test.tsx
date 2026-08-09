import { afterEach, expect, mock, test } from "bun:test";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import type { ReactNode } from "react";

import "../../lib/i18n/index";
import { serversApi } from "../../lib/api";
import type { MCPServerConfig, ServerDetail } from "../../lib/types";

type SubmitHandler = (
	config: Partial<MCPServerConfig>,
) => Promise<void> | void;

let capturedSubmit: SubmitHandler | undefined;
const notifyErrorCalls: Array<[title: string, description?: string]> = [];

mock.module("../../components/server-edit-drawer", () => ({
	ServerEditDrawer: ({ onSubmit }: { onSubmit: SubmitHandler }) => {
		capturedSubmit = onSubmit;
		return null;
	},
}));

mock.module("react-router-dom", () => ({
	Link: ({ children }: { children: ReactNode }) => children,
	useLocation: () => ({ pathname: "/servers/server-a", search: "" }),
	useNavigate: () => () => undefined,
	useParams: () => ({ serverId: server.id }),
	useSearchParams: () => [new URLSearchParams(), () => undefined],
}));

mock.module("../../lib/notify", () => ({
	notifyError: (title: string, description?: string) => {
		notifyErrorCalls.push([title, description]);
	},
	notifySuccess: () => undefined,
}));

const originalUpdateServer = serversApi.updateServer;

const server: ServerDetail = {
	id: "server-a",
	name: "server-a",
	server_type: "stdio",
	status: "idle",
	enabled: true,
	instances: [],
};

afterEach(() => {
	serversApi.updateServer = originalUpdateServer;
	capturedSubmit = undefined;
	notifyErrorCalls.length = 0;
});

async function renderServerDetail(
	serverDetail: ServerDetail = server,
): Promise<{ queryClient: QueryClient; markup: string }> {
	const { ServerDetailPage } = await import("./server-detail-page");
	const queryClient = new QueryClient({
		defaultOptions: { queries: { retry: false } },
	});
	queryClient.setQueryData(["server", server.id], serverDetail);
	queryClient.setQueryData(["server-cap", "all", server.id], {
		tools: { items: [], state: "ok" },
		resources: { items: [], state: "ok" },
		prompts: { items: [], state: "ok" },
		templates: { items: [], state: "ok" },
	});
	queryClient.invalidateQueries = mock(
		queryClient.invalidateQueries.bind(queryClient),
	);

	const markup = renderToStaticMarkup(
		<QueryClientProvider client={queryClient}>
			<ServerDetailPage />
		</QueryClientProvider>,
	);

	return { queryClient, markup };
}

test("keeps an edit successful while surfacing its failed capability discovery", async () => {
	serversApi.updateServer = async () => ({
		success: true,
		data: {
			...server,
			capability_discovery: {
				attempted: true,
				status: "failed",
				error:
					"Capability discovery failed. Check the server configuration and upstream availability.",
			},
		},
	});

	const { queryClient } = await renderServerDetail();
	expect(capturedSubmit).toBeDefined();

	await expect(
		capturedSubmit!({ name: server.name, kind: "stdio", command: "node" }),
	).resolves.toBeUndefined();

	expect(notifyErrorCalls).toEqual([
		[
			"Refresh failed",
			"Unable to refresh server capabilities: Capability discovery failed. Check the server configuration and upstream availability.",
		],
	]);
	expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
		queryKey: ["server", server.id],
	});
	expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
		queryKey: ["servers"],
	});
});

test("prioritizes an unrecognized transport over its compatibility projection", async () => {
	const { markup } = await renderServerDetail({
		...server,
		transport_validity: {
			state: "invalid",
			diagnostics: [
				{ code: "transport_unrecognized", field: "transport" },
			],
			draft: { kind: "unrecognized", declared_type: "websocket" },
		},
	});

	expect(markup).toContain("Unknown transport: websocket");
	expect(markup).toContain("Repair required");
	expect(markup).not.toContain(">stdio<");
	expect(markup).toMatch(
		/<button[^>]*border-red-200[^>]*>[\s\S]*?Unknown transport: websocket/,
	);
});

test("elevates OAuth reauthorize warning into the detail header status slot", async () => {
	const { markup } = await renderServerDetail({
		...server,
		server_type: "streamable_http",
		status: "Disconnected",
		enabled: false,
		auth_mode: "oauth",
		oauth_status: "expired",
	});

	expect(markup).toMatch(
		/<h2[^>]*>[\s\S]*?<\/h2>\s*<span[^>]*border-red-200[^>]*>[\s\S]*?Reauthorize required/,
	);
	expect(markup).not.toMatch(
		/<h2[^>]*>[\s\S]*?<\/h2>\s*<div[^>]*bg-amber-500[^>]*>[\s\S]*?Disconnected/,
	);
});

test("disables capability actions until an unrecognized transport is repaired", async () => {
	const { markup } = await renderServerDetail({
		...server,
		transport_validity: {
			state: "invalid",
			diagnostics: [
				{ code: "transport_unrecognized", field: "transport" },
			],
			draft: { kind: "unrecognized", declared_type: "websocket" },
		},
	});

	expect(markup).toMatch(
		/<button[^>]*disabled="">[\s\S]*?lucide-refresh-cw[\s\S]*?Refresh<\/button>/,
	);
	expect(markup).toMatch(
		/<button[^>]*disabled=""[^>]*>[\s\S]*?Capabilities \(0\)<\/button>/,
	);
});
