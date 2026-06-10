import { describe, expect, it } from "vitest";
import { resolveOAuthReadiness } from "./oauth-readiness";
import type { OAuthStatus, SecretStoreStatusData } from "./types";

const readyStore: SecretStoreStatusData = {
	status: "ready",
	provider: {
		provider_id: "local",
		provider_kind: "local_file",
		provider_mode: "development",
		security_level: "development",
	},
	issue: null,
};

describe("resolveOAuthReadiness", () => {
	it("blocks OAuth actions when secure store is unavailable", () => {
		const readiness = resolveOAuthReadiness({
			secretStoreStatus: {
				status: "unavailable",
				provider: null,
				issue: {
					reason_code: "locked",
					message: "Secure Store is locked.",
				},
			},
			oauthStatus: null,
		});

		expect(readiness.actionDisabled).toBe(true);
		expect(readiness.notice?.kind).toBe("secure-store-unavailable");
		expect(readiness.notice?.message).toContain("Secure Store is locked.");
	});

	it("prompts reconnect for legacy plaintext OAuth credentials", () => {
		const oauthStatus: OAuthStatus = {
			server_id: "serv_legacy",
			configured: true,
			state: "connected",
			custody_state: "legacy_plaintext",
			requires_reconnect: true,
			issue: {
				code: "legacy_plaintext_oauth_credentials",
				message: "Reconnect OAuth to store credentials securely.",
			},
		};

		const readiness = resolveOAuthReadiness({
			secretStoreStatus: readyStore,
			oauthStatus,
		});

		expect(readiness.actionDisabled).toBe(false);
		expect(readiness.notice?.kind).toBe("legacy-reconnect-required");
		expect(readiness.notice?.message).toContain("Reconnect OAuth");
	});

	it("blocks when OAuth status reports unavailable custody without a store status", () => {
		const oauthStatus: OAuthStatus = {
			server_id: "serv_unavailable",
			configured: true,
			state: "connected",
			custody_state: "unavailable",
			requires_reconnect: true,
			issue: {
				code: "secure_store_unavailable",
				message: "Secure Store is unavailable.",
			},
		};

		const readiness = resolveOAuthReadiness({
			secretStoreStatus: undefined,
			oauthStatus,
		});

		expect(readiness.actionDisabled).toBe(true);
		expect(readiness.notice?.kind).toBe("secure-store-unavailable");
		expect(readiness.notice?.message).toContain("Secure Store is unavailable.");
	});
});
