import type { OAuthStatus, SecretStoreStatusData } from "./types";

export type OAuthReadinessNoticeKind =
	| "secure-store-unavailable"
	| "legacy-reconnect-required";

export interface OAuthReadinessNotice {
	kind: OAuthReadinessNoticeKind;
	/** i18n translation key for the notice body */
	messageKey: string;
	/** English fallback shown until the key is translated */
	defaultMessage: string;
}

export interface OAuthReadiness {
	actionDisabled: boolean;
	notice: OAuthReadinessNotice | null;
}

export type OAuthReadinessActionTarget = "auth-flow" | "security-settings";

export interface ServerOAuthReadinessSource {
	id: string;
	oauth_status?: OAuthStatus["state"] | null;
	oauth_custody_state?: OAuthStatus["custody_state"] | null;
	oauth_requires_reconnect?: boolean | null;
	oauth_issue?: OAuthStatus["issue"] | null;
}

function secureStoreUnavailable(message?: string): OAuthReadiness {
	return {
		actionDisabled: true,
		notice: {
			kind: "secure-store-unavailable",
			messageKey: "manual.auth.oauth.secureStoreUnavailable.message",
			defaultMessage:
				message ??
				"Secure Store is not ready. Unlock or initialize it before connecting OAuth.",
		},
	};
}

export function resolveOAuthReadiness({
	secretStoreStatus,
	oauthStatus,
}: {
	secretStoreStatus?: SecretStoreStatusData | null;
	oauthStatus?: OAuthStatus | null;
}): OAuthReadiness {
	if (
		oauthStatus?.custody_state === "unavailable" ||
		oauthStatus?.issue?.code === "secure_store_unavailable"
	) {
		return secureStoreUnavailable(oauthStatus.issue?.message);
	}

	if (secretStoreStatus && secretStoreStatus.status !== "ready") {
		return secureStoreUnavailable(secretStoreStatus.issue?.message);
	}

	if (oauthStatus?.requires_reconnect || oauthStatus?.custody_state === "legacy_plaintext") {
		return {
			actionDisabled: false,
			notice: {
				kind: "legacy-reconnect-required",
				messageKey: "manual.auth.oauth.legacyReconnect.message",
				defaultMessage:
					oauthStatus.issue?.message ??
					"Reconnect OAuth to move existing credentials into Secure Store custody.",
			},
		};
	}

	return {
		actionDisabled: false,
		notice: null,
	};
}

export function resolveServerOAuthReadiness(
	server: ServerOAuthReadinessSource,
): OAuthReadiness {
	return resolveOAuthReadiness({
		oauthStatus: {
			server_id: server.id,
			configured: true,
			state: server.oauth_status ?? null,
			custody_state: server.oauth_custody_state ?? null,
			requires_reconnect: server.oauth_requires_reconnect ?? false,
			issue: server.oauth_issue ?? null,
		},
	});
}

export function getOAuthReadinessActionTarget(
	readiness?: OAuthReadiness | null,
): OAuthReadinessActionTarget {
	return readiness?.notice?.kind === "secure-store-unavailable"
		? "security-settings"
		: "auth-flow";
}
