import type { OAuthStatus, SecretStoreStatusData } from "./types";

export type OAuthReadinessNoticeKind =
	| "secure-store-unavailable"
	| "legacy-reconnect-required";

export interface OAuthReadinessNotice {
	kind: OAuthReadinessNoticeKind;
	message: string;
}

export interface OAuthReadiness {
	actionDisabled: boolean;
	notice: OAuthReadinessNotice | null;
}

const SECURE_STORE_UNAVAILABLE_MESSAGE =
	"Secure Store is not ready. Unlock or initialize it before connecting OAuth.";

function secureStoreUnavailable(message?: string): OAuthReadiness {
	return {
		actionDisabled: true,
		notice: {
			kind: "secure-store-unavailable",
			message: message ?? SECURE_STORE_UNAVAILABLE_MESSAGE,
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
				message:
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
