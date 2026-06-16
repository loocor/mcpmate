import type { TFunction } from "i18next";
import { describe, expect, it } from "vitest";
import {
	normalizeSecretStoreReasonCode,
	resolveSecretStoreIssueGuidance,
} from "./secret-store-guidance";
import type { SecretStoreStatusData } from "./types";

const t = ((key: string, options?: { defaultValue?: string }) =>
	options?.defaultValue ?? key) as TFunction;

describe("resolveSecretStoreIssueGuidance", () => {
	it("returns null for ready status", () => {
		expect(
			resolveSecretStoreIssueGuidance({ status: "ready", provider: null }, t),
		).toBeNull();
	});

	it("returns null for passphrase unlock because the lock screen owns that flow", () => {
		expect(
			resolveSecretStoreIssueGuidance(
				{
					status: "unavailable",
					issue: {
						reason_code: "passphrase_unlock_required",
						message: "Unlock required",
					},
				},
				t,
			),
		).toBeNull();
	});

	it("guides OS keychain denial with retry and settings actions", () => {
		const guidance = resolveSecretStoreIssueGuidance(
			{
				status: "unavailable",
				provider: {
					provider_id: "os",
					provider_kind: "operating_system_keychain",
					provider_mode: "operating_system",
					security_level: "high",
				},
				issue: {
					reason_code: "provider_unavailable",
					message: "keyring set: denied",
				},
			} satisfies SecretStoreStatusData,
			t,
		);

		expect(guidance?.actions).toEqual(["retry_provider", "open_security_settings"]);
		expect(guidance?.retryProviderMode).toBe("operating_system");
	});

	it("guides passphrase provider_unavailable with retry provider", () => {
		const guidance = resolveSecretStoreIssueGuidance(
			{
				status: "unavailable",
				provider: {
					provider_id: "passphrase",
					provider_kind: "passphrase_wrapped_root_key",
					provider_mode: "passphrase",
					security_level: "medium",
				},
				issue: {
					reason_code: "provider_unavailable",
					message: "unlock failed",
				},
			} satisfies SecretStoreStatusData,
			t,
		);

		expect(guidance?.actions).toEqual(["retry_provider", "open_security_settings"]);
		expect(guidance?.retryProviderMode).toBe("passphrase");
	});

	it("falls back to status retry when provider mode is unknown", () => {
		const guidance = resolveSecretStoreIssueGuidance(
			{
				status: "unavailable",
				provider: {
					provider_id: "unknown",
					provider_kind: "unknown",
					provider_mode: "unknown_mode" as "operating_system",
					security_level: "low",
				},
				issue: {
					reason_code: "provider_unavailable",
					message: "init failed",
				},
			} satisfies SecretStoreStatusData,
			t,
		);

		expect(guidance?.actions).toEqual(["retry_status", "open_security_settings"]);
		expect(guidance?.retryProviderMode).toBeUndefined();
	});

	it("guides read lock failures with status retry only", () => {
		const guidance = resolveSecretStoreIssueGuidance(
			{
				status: "unavailable",
				issue: {
					reason_code: "read_lock_failed",
					message: "lock timeout",
				},
			} satisfies SecretStoreStatusData,
			t,
		);

		expect(guidance?.actions).toEqual(["retry_status"]);
	});

	it("guides missing root key without generic fallback", () => {
		const guidance = resolveSecretStoreIssueGuidance(
			{
				status: "unavailable",
				provider: {
					provider_id: "local-file",
					provider_kind: "local_file_root_key",
					provider_mode: "local_file",
					security_level: "medium",
				},
				issue: {
					reason_code: "missing_root_key",
					message: "root key file missing",
				},
			} satisfies SecretStoreStatusData,
			t,
		);

		expect(guidance?.title).toBe("Root key material is missing");
		expect(guidance?.actions).toEqual(["retry_provider", "open_security_settings"]);
		expect(guidance?.retryProviderMode).toBe("local_file");
	});

	it("guides secret key mismatch without retrying the same provider", () => {
		const guidance = resolveSecretStoreIssueGuidance(
			{
				status: "unavailable",
				provider: {
					provider_id: "local-file",
					provider_kind: "local_file_root_key",
					provider_mode: "local_file",
					security_level: "medium",
				},
				issue: {
					reason_code: "secret_key_mismatch",
					message: "record cannot be decrypted",
				},
			} satisfies SecretStoreStatusData,
			t,
		);

		expect(guidance?.title).toBe("Secure store records need repair");
		expect(guidance?.actions).toEqual(["open_security_settings"]);
		expect(guidance?.retryProviderMode).toBeUndefined();
	});

	it("uses generic guidance for unknown reason codes", () => {
		const guidance = resolveSecretStoreIssueGuidance(
			{
				status: "unavailable",
				issue: {
					reason_code: "database_unavailable",
					message: "db locked",
				},
			} satisfies SecretStoreStatusData,
			t,
		);

		expect(guidance?.actions).toEqual(["retry_status", "open_security_settings"]);
		expect(guidance?.title).toBe("Secure store unavailable");
	});

	it("normalizes unknown reason codes", () => {
		expect(normalizeSecretStoreReasonCode("future_backend_code")).toBe("unknown");
		expect(normalizeSecretStoreReasonCode("read_lock_failed")).toBe(
			"read_lock_failed",
		);
	});
});
