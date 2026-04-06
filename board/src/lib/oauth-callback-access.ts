import { API_BASE_URL, serversApi } from "./api";
import { isTauriEnvironmentSync } from "./platform";
import type {
	OAuthCallbackAccessContract,
	OAuthCallbackNotificationPayload,
	OAuthConfigRequest,
} from "./types";

const WEB_DEV_CALLBACK_ORIGIN = "http://127.0.0.1:5173";
const DESKTOP_DEFAULT_API_BASE = "http://127.0.0.1:8080";

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

function resolveDesktopApiBaseUrl(): string {
	const trimmed = API_BASE_URL.trim();
	return trimmed.length > 0 ? trimmed : DESKTOP_DEFAULT_API_BASE;
}

export async function resolveOAuthCallbackAccess(
	serverId: string,
): Promise<OAuthCallbackAccessContract> {
	if (isHttpCallbackSurface() || !isTauriEnvironmentSync()) {
		return {
			kind: "web",
			redirect_uri: buildWebOAuthRedirectUri(),
		};
	}

	const { invoke } = await import("@tauri-apps/api/core");
	return invoke<OAuthCallbackAccessContract>("mcp_oauth_prepare_callback_access", {
		serverId,
		apiBaseUrl: resolveDesktopApiBaseUrl(),
	});
}

async function openOAuthAuthorizationUrl(
	authorizationUrl: string,
): Promise<void> {
	if (isTauriEnvironmentSync()) {
		const { invoke } = await import("@tauri-apps/api/core");
		await invoke("mcp_oauth_open_authorization_url", { authorizationUrl });
		return;
	}

	const width = Math.min(500, window.outerWidth - 40);
	const height = Math.min(700, window.outerHeight - 60);
	const left = window.screenX + (window.outerWidth - width) / 2;
	const top = window.screenY + (window.outerHeight - height) / 2;
	const popup = window.open(
		authorizationUrl,
		"oauth_window",
		`width=${width},height=${height},left=${left},top=${top}`,
	);
	if (!popup) {
		window.location.assign(authorizationUrl);
	}
}

export async function startOAuthAccessFlow(
	serverId: string,
	config: OAuthConfigRequest,
): Promise<void> {
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
		await openOAuthAuthorizationUrl(redirectRes.authorization_url);
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
