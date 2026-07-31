import { afterEach, describe, expect, test } from "bun:test";
import {
	publishOAuthCallbackNotification,
	reserveOAuthAuthorizationWindow,
	startOAuthAccessFlow,
} from "./oauth-callback-access";
import { serversApi } from "./api";
import type { OAuthConfigRequest, OAuthStatus } from "./types";

const originalPrepareOAuth = serversApi.prepareOAuth;
const originalInitiateOAuth = serversApi.initiateOAuth;
const originalWindow = globalThis.window;
const originalBroadcastChannel = globalThis.BroadcastChannel;

const config: OAuthConfigRequest = {
	authorization_endpoint: "",
	token_endpoint: "",
	client_id: "",
	scopes: "mcp:read",
	redirect_uri: "",
};

function installWebWindow(
	open: Window["open"],
	assign: Location["assign"],
	dimensions: { outerWidth: number; outerHeight: number } = {
		outerWidth: 1440,
		outerHeight: 900,
	},
): void {
	Object.defineProperty(globalThis, "window", {
		configurable: true,
		value: {
			location: {
				protocol: "http:",
				origin: "http://localhost:5173",
				assign,
			},
			outerWidth: dimensions.outerWidth,
			outerHeight: dimensions.outerHeight,
			screenX: 0,
			screenY: 0,
			open,
		},
	});
}

function installOAuthApi(): { prepare: number; initiate: number } {
	const calls = { prepare: 0, initiate: 0 };
	serversApi.prepareOAuth = async (serverId): Promise<OAuthStatus> => {
		calls.prepare += 1;
		return {
			server_id: serverId,
			configured: true,
			state: "disconnected",
		};
	};
	serversApi.initiateOAuth = async (serverId) => {
		calls.initiate += 1;
		return {
			server_id: serverId,
			authorization_url: "https://issuer.example/authorize",
			state: "oauth-state",
		};
	};
	return calls;
}

afterEach(() => {
	serversApi.prepareOAuth = originalPrepareOAuth;
	serversApi.initiateOAuth = originalInitiateOAuth;
	Object.defineProperty(globalThis, "window", {
		configurable: true,
		value: originalWindow,
	});
	Object.defineProperty(globalThis, "BroadcastChannel", {
		configurable: true,
		value: originalBroadcastChannel,
	});
});

describe("startOAuthAccessFlow", () => {
	test("navigates a popup reserved by the user gesture", async () => {
		installOAuthApi();
		let authorizationUrl = "";
		const authorizationWindow = {
			location: {
				replace: (url: string) => {
					authorizationUrl = url;
				},
			},
		} as unknown as Window;
		installWebWindow(() => null, () => undefined);

		await startOAuthAccessFlow("server-google-ads", config, authorizationWindow);

		expect(authorizationUrl).toBe("https://issuer.example/authorize");
	});

	test("rejects a blocked popup without replacing the Board tab", async () => {
		const oauthApiCalls = installOAuthApi();
		let assignedUrl = "";
		installWebWindow(
			() => null,
			(url: string | URL) => {
				assignedUrl = String(url);
			},
		);

		await expect(
			startOAuthAccessFlow("server-google-ads", config),
		).rejects.toThrow("popup");
		expect(assignedUrl).toBe("");
		expect(oauthApiCalls).toEqual({ prepare: 0, initiate: 0 });
	});

	test("rejects when the reserved popup closes during preparation", async () => {
		installOAuthApi();
		const authorizationWindow = {
			closed: true,
			location: {
				replace: () => undefined,
			},
		} as unknown as Window;
		installWebWindow(() => authorizationWindow, () => undefined);

		await expect(
			startOAuthAccessFlow("server-google-ads", config, authorizationWindow),
		).rejects.toThrow("closed");
	});
});

describe("reserveOAuthAuthorizationWindow", () => {
	test("renders preparation status immediately in the reserved popup", () => {
		let popupHtml = "";
		let popupTarget = "";
		const authorizationWindow = {
			opener: window,
			document: {
				open: () => undefined,
				write: (html: string) => {
					popupHtml = html;
				},
				close: () => undefined,
			},
		} as unknown as Window;
		installWebWindow((_url, target) => {
			popupTarget = String(target);
			return authorizationWindow;
		}, () => undefined);

		reserveOAuthAuthorizationWindow({
			title: "MCPMate OAuth",
			heading: "Preparing authorization",
			description: "Discovering OAuth metadata and registering a secure client.",
			language: "en",
		});

		expect(popupTarget).toBe("_blank");
		expect(authorizationWindow.opener).toBeNull();
		expect(popupHtml).toContain('<html lang="en">');
		expect(popupHtml).toContain("<title>MCPMate OAuth</title>");
		expect(popupHtml).toContain("Preparing authorization");
		expect(popupHtml).toContain("Discovering OAuth metadata and registering a secure client.");
	});

	test("uses positive popup dimensions in a very small browser window", () => {
		let features = "";
		const authorizationWindow = {
			opener: window,
			document: {
				open: () => undefined,
				write: () => undefined,
				close: () => undefined,
			},
		} as unknown as Window;
		installWebWindow(
			(_url, _target, windowFeatures) => {
				features = String(windowFeatures);
				return authorizationWindow;
			},
			() => undefined,
			{ outerWidth: 20, outerHeight: 40 },
		);

		reserveOAuthAuthorizationWindow({
			title: "MCPMate OAuth",
			heading: "Preparing authorization",
			description: "Preparing a secure authorization window.",
			language: "en",
		});

		expect(features).toContain("width=320");
		expect(features).toContain("height=480");
	});
});

describe("publishOAuthCallbackNotification", () => {
	test("publishes an opener-independent storage event payload", () => {
		let storedPayload = "";
		let removedKey = "";
		installWebWindow(() => null, () => undefined);
		Object.assign(window, {
			opener: null,
			localStorage: {
				setItem: (key: string, value: string) => {
					if (key === "mcpmate.oauth.callback") {
						storedPayload = value;
					}
				},
				removeItem: (key: string) => {
					removedKey = key;
				},
			},
		});
		const payload = {
			type: "OAUTH_CALLBACK_ERROR" as const,
			error: "Authorization failed",
			timestamp: 123,
		};

		publishOAuthCallbackNotification(payload);

		expect(JSON.parse(storedPayload)).toEqual(payload);
		expect(removedKey).toBe("mcpmate.oauth.callback");
	});

	test("does not publish a storage fallback when BroadcastChannel succeeds", () => {
		const channelMessages: unknown[] = [];
		let storageWrites = 0;
		class TestBroadcastChannel {
			constructor(_name: string) {}

			postMessage(payload: unknown): void {
				channelMessages.push(payload);
			}

			close(): void {}
		}
		installWebWindow(() => null, () => undefined);
		Object.assign(window, {
			opener: null,
			BroadcastChannel: TestBroadcastChannel,
			localStorage: {
				setItem: () => {
					storageWrites += 1;
				},
				removeItem: () => undefined,
			},
		});
		Object.defineProperty(globalThis, "BroadcastChannel", {
			configurable: true,
			value: TestBroadcastChannel,
		});
		const payload = {
			type: "OAUTH_CALLBACK_ERROR" as const,
			error: "Authorization failed",
			timestamp: 123,
		};

		publishOAuthCallbackNotification(payload);

		expect(channelMessages).toEqual([payload]);
		expect(storageWrites).toBe(0);
	});
});
