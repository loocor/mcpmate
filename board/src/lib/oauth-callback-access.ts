import { requireApiBaseUrl, serversApi } from "./api";
import { isTauriEnvironmentSync } from "./platform";
import type {
	OAuthCallbackAccessContract,
	OAuthCallbackNotificationPayload,
	OAuthConfigRequest,
} from "./types";

const WEB_DEV_CALLBACK_ORIGIN = "http://127.0.0.1:5173";
const OAUTH_CALLBACK_STORAGE_KEY = "mcpmate.oauth.callback";
function isHttpCallbackSurface(): boolean {
	if (typeof window === "undefined") {
		return false;
	}

	const protocol = window.location.protocol.toLowerCase();
	return protocol === "http:" || protocol === "https:";
}

export function buildWebOAuthRedirectUri(): string {
	if (typeof window === "undefined") {
		return `${WEB_DEV_CALLBACK_ORIGIN}/oauth/callback`;
	}

	if (isHttpCallbackSurface()) {
		return `${window.location.origin}/oauth/callback`;
	}

	return "";
}

export function getOAuthRedirectUriForForm(storedRedirectUri?: string | null): string {
	const trimmed = storedRedirectUri?.trim() ?? "";

	if (isHttpCallbackSurface()) {
		return trimmed || buildWebOAuthRedirectUri();
	}

	if (isTauriEnvironmentSync()) {
		if (trimmed.startsWith("http://127.0.0.1:") && trimmed.endsWith("/oauth/callback")) {
			return "";
		}

		return trimmed;
	}

	return trimmed || buildWebOAuthRedirectUri();
}

export async function resolveOAuthCallbackAccess(
	serverId: string,
): Promise<OAuthCallbackAccessContract> {
	if (!isTauriEnvironmentSync() && isHttpCallbackSurface()) {
		return {
			kind: "web",
			redirect_uri: buildWebOAuthRedirectUri(),
		};
	}

	const { invoke } = await import("@tauri-apps/api/core");
	return invoke<OAuthCallbackAccessContract>("mcp_oauth_prepare_callback_access", {
		serverId,
		apiBaseUrl: requireApiBaseUrl("desktop OAuth callback access"),
	});
}

function popupFeatures(): string {
	const width = Math.max(320, Math.min(500, window.outerWidth - 40));
	const height = Math.max(480, Math.min(700, window.outerHeight - 60));
	const left = window.screenX + (window.outerWidth - width) / 2;
	const top = window.screenY + (window.outerHeight - height) / 2;
	return `width=${width},height=${height},left=${left},top=${top}`;
}

interface OAuthPopupPreparationCopy {
	title: string;
	heading: string;
	description: string;
	language: string;
}

function tryAction(action: () => void): boolean {
	try {
		action();
		return true;
	} catch {
		return false;
	}
}

export function publishOAuthCallbackNotification(
	payload: OAuthCallbackNotificationPayload,
): void {
	const opener = window.opener;
	if (opener && tryAction(() => opener.postMessage(payload, window.location.origin))) {
		tryAction(() => opener.focus());
		return;
	}

	if ("BroadcastChannel" in window) {
		const published = tryAction(() => {
			const channel = new BroadcastChannel("mcpmate-oauth");
			try {
				channel.postMessage(payload);
			} finally {
				channel.close();
			}
		});
		if (published) {
			return;
		}
	}

	let storageSignal: OAuthCallbackNotificationPayload;
	switch (payload.type) {
		case "OAUTH_CALLBACK_SUCCESS":
			storageSignal = {
				type: "OAUTH_CALLBACK_SUCCESS",
				timestamp: Date.now(),
			};
			break;
		case "OAUTH_CALLBACK_ERROR":
			storageSignal = {
				type: "OAUTH_CALLBACK_ERROR",
				timestamp: Date.now(),
			};
			break;
		default:
			return;
	}

	tryAction(() => {
		window.localStorage.setItem(
			OAUTH_CALLBACK_STORAGE_KEY,
			JSON.stringify(storageSignal),
		);
		window.localStorage.removeItem(OAUTH_CALLBACK_STORAGE_KEY);
	});
}

function escapeHtml(value: string): string {
	return value
		.replaceAll("&", "&amp;")
		.replaceAll("<", "&lt;")
		.replaceAll(">", "&gt;")
		.replaceAll('"', "&quot;")
		.replaceAll("'", "&#039;");
}

function renderOAuthPreparation(
	authorizationWindow: Window,
	copy: OAuthPopupPreparationCopy,
): void {
	const popupDocument = authorizationWindow.document;
	popupDocument.open();
	popupDocument.write(`<!doctype html>
<html lang="${escapeHtml(copy.language)}">
	<head>
		<meta charset="utf-8">
		<meta name="viewport" content="width=device-width, initial-scale=1">
		<title>${escapeHtml(copy.title)}</title>
		<style>
			:root { color-scheme: light dark; font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
			body { margin: 0; min-height: 100vh; display: grid; place-items: center; background: #f8fafc; color: #0f172a; }
			main { width: min(360px, calc(100vw - 48px)); text-align: center; }
			.spinner { width: 28px; height: 28px; margin: 0 auto 20px; border: 3px solid #cbd5e1; border-top-color: #0f172a; border-radius: 9999px; animation: spin 0.8s linear infinite; }
			h1 { margin: 0; font-size: 18px; line-height: 1.4; font-weight: 600; }
			p { margin: 10px 0 0; color: #64748b; font-size: 14px; line-height: 1.6; }
			@keyframes spin { to { transform: rotate(360deg); } }
			@media (prefers-color-scheme: dark) {
				body { background: #020617; color: #f8fafc; }
				.spinner { border-color: #334155; border-top-color: #f8fafc; }
				p { color: #94a3b8; }
			}
		</style>
	</head>
	<body>
		<main>
			<div class="spinner" aria-hidden="true"></div>
			<h1>${escapeHtml(copy.heading)}</h1>
			<p>${escapeHtml(copy.description)}</p>
		</main>
	</body>
</html>`);
	popupDocument.close();
}

export function reserveOAuthAuthorizationWindow(
	copy: OAuthPopupPreparationCopy,
): Window | null | undefined {
	if (isTauriEnvironmentSync()) {
		return undefined;
	}

	const authorizationWindow = window.open("", "_blank", popupFeatures());
	if (authorizationWindow) {
		renderOAuthPreparation(authorizationWindow, copy);
		authorizationWindow.opener = null;
	}
	return authorizationWindow;
}

async function openOAuthAuthorizationUrl(
	authorizationUrl: string,
	authorizationWindow?: Window,
): Promise<void> {
	if (isTauriEnvironmentSync()) {
		const { invoke } = await import("@tauri-apps/api/core");
		await invoke("mcp_oauth_open_authorization_url", { authorizationUrl });
		return;
	}

	if (!authorizationWindow) {
		throw new Error("OAuth popup is unavailable");
	}
	if (authorizationWindow.closed) {
		throw new Error("OAuth popup was closed before authorization could begin");
	}

	authorizationWindow.location.replace(authorizationUrl);
}

export async function startOAuthAccessFlow(
	serverId: string,
	config: OAuthConfigRequest,
	authorizationWindow?: Window,
): Promise<void> {
	if (!isTauriEnvironmentSync() && !authorizationWindow) {
		throw new Error("OAuth popup is unavailable");
	}

	const callbackAccess = await resolveOAuthCallbackAccess(serverId);
	const effectiveConfig = {
		...config,
		redirect_uri: callbackAccess.redirect_uri,
	};
	const shouldUseManualConfig =
		Boolean(effectiveConfig.authorization_endpoint?.trim()) &&
		Boolean(effectiveConfig.token_endpoint?.trim()) &&
		Boolean(effectiveConfig.client_id?.trim());

	if (shouldUseManualConfig) {
		await serversApi.saveOAuthConfig(serverId, effectiveConfig);
	} else {
		await serversApi.prepareOAuth(serverId, {
			redirect_uri: effectiveConfig.redirect_uri,
			scopes: effectiveConfig.scopes,
		});
	}

	const redirectRes = await serversApi.initiateOAuth(serverId);
	if (redirectRes.authorization_url) {
		await openOAuthAuthorizationUrl(
			redirectRes.authorization_url,
			authorizationWindow,
		);
	}
}

export async function bindDesktopOAuthCallback(
	handler: (payload: OAuthCallbackNotificationPayload) => void | Promise<void>,
): Promise<(() => void) | undefined> {
	if (!isTauriEnvironmentSync()) {
		return undefined;
	}

	const { listen } = await import("@tauri-apps/api/event");
	return listen<OAuthCallbackNotificationPayload>("mcp-oauth/callback", (event) => {
		void handler(event.payload);
	});
}
