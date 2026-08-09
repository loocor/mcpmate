import { afterEach, expect, mock, test } from "bun:test";
import * as ReactQuery from "@tanstack/react-query";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import type { ReactNode } from "react";

import "../../lib/i18n/index";
import { serversApi } from "../../lib/api";
import type { MCPServerConfig, ServerDetail, ServerListResponse } from "../../lib/types";

type UpdateMutation = {
	mutationFn: (variables: {
		serverId: string;
		config: Partial<MCPServerConfig>;
	}) => Promise<unknown>;
	onSuccess: (result: unknown, variables: { serverId: string }) => void;
};

let updateMutation: UpdateMutation | undefined;
const notifyErrorCalls: Array<[title: string, description?: string]> = [];

mock.module("@tanstack/react-query", () => ({
	...ReactQuery,
	useMutation: (options: UpdateMutation) => {
		updateMutation = options;
		return {
			mutateAsync: async (variables: Parameters<UpdateMutation["mutationFn"]>[0]) => {
				const result = await options.mutationFn(variables);
				options.onSuccess(result, variables);
				return result;
			},
		};
	},
}));

mock.module("react-router-dom", () => ({
	Link: ({ children }: { children: ReactNode }) => children,
	useLocation: () => ({ pathname: "/servers", search: "" }),
	useNavigate: () => () => undefined,
	useParams: () => ({ serverId: server.id }),
	useSearchParams: () => [new URLSearchParams(), () => undefined],
}));

mock.module("../../components/server-install", () => ({
	ServerInstallManualForm: () => null,
	ServerInstallWizard: () => null,
}));

mock.module("../../lib/notify", () => ({
	notifyError: (title: string, description?: string) => {
		notifyErrorCalls.push([title, description]);
	},
	notifyInfo: () => undefined,
	notifySuccess: () => undefined,
	notifyWarning: () => undefined,
}));

const server: ServerDetail = {
	id: "server-a",
	name: "server-a",
	server_type: "stdio",
	status: "idle",
	enabled: true,
	instances: [],
};

const originalUpdateServer = serversApi.updateServer;

afterEach(() => {
	serversApi.updateServer = originalUpdateServer;
	updateMutation = undefined;
	notifyErrorCalls.length = 0;
});

test("keeps a list edit successful while surfacing its failed capability discovery", async () => {
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
	const { ServerListPage } = await import("./server-list-page");
	const queryClient = new QueryClient({
		defaultOptions: { queries: { retry: false } },
	});
	queryClient.setQueryData<ServerListResponse>(["servers"], { servers: [server] });

	renderToStaticMarkup(
		<QueryClientProvider client={queryClient}>
			<ServerListPage />
		</QueryClientProvider>,
	);

	expect(updateMutation).toBeDefined();
	const result = await updateMutation!.mutationFn({
		serverId: server.id,
		config: { kind: "stdio", command: "node" },
	});
	updateMutation!.onSuccess(result, { serverId: server.id });

	expect(notifyErrorCalls).toEqual([
		[
			"Refresh failed",
			"Unable to refresh server capabilities: Capability discovery failed. Check the server configuration and upstream availability.",
		],
	]);
});
