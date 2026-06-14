import type { TFunction } from "i18next";
import type {
	SecretStoreIssueReasonCode,
	SecretStoreStatusData,
	SwitchableSecretStoreProviderMode,
} from "./types";
import { SECRET_STORE_REASON_CODES } from "./types";

type SecretStoreGuidanceAction =
	| "retry_status"
	| "retry_provider"
	| "open_security_settings";

export type { SecretStoreProviderMode, SwitchableSecretStoreProviderMode } from "./types";

export interface SecretStoreIssueGuidance {
	title: string;
	description: string;
	technicalDetail?: string;
	actions: SecretStoreGuidanceAction[];
	retryProviderMode?: SwitchableSecretStoreProviderMode;
}

export function isSwitchableSecretStoreProviderMode(
	value: string | undefined,
): value is SwitchableSecretStoreProviderMode {
	return (
		value === "operating_system" ||
		value === "passphrase" ||
		value === "local_file"
	);
}

export function normalizeSecretStoreReasonCode(
	value: string | undefined,
): SecretStoreIssueReasonCode {
	if (value && (SECRET_STORE_REASON_CODES as readonly string[]).includes(value)) {
		return value as SecretStoreIssueReasonCode;
	}
	return "unknown";
}

export function resolveSecretStoreIssueGuidance(
	status: SecretStoreStatusData | undefined,
	t: TFunction,
): SecretStoreIssueGuidance | null {
	if (!status || status.status === "ready") {
		return null;
	}

	const reasonCode = normalizeSecretStoreReasonCode(status.issue?.reason_code);
	const technicalDetail = status.issue?.message;
	const providerMode = status.provider?.provider_mode;

	if (reasonCode === "passphrase_unlock_required") {
		return null;
	}

	if (
		reasonCode === "provider_unavailable" &&
		providerMode === "operating_system"
	) {
		return {
			title: t("guidance.providerUnavailable.os.title", {
				defaultValue: "OS secure storage is unavailable",
			}),
			description: t("guidance.providerUnavailable.os.description", {
				defaultValue:
					"MCPMate could not access the OS keychain. Grant access when prompted, unlock Keychain Access on macOS, or switch to Password or Local File mode below.",
			}),
			technicalDetail,
			actions: ["retry_provider", "open_security_settings"],
			retryProviderMode: "operating_system",
		};
	}

	if (reasonCode === "provider_unavailable") {
		const mode = isSwitchableSecretStoreProviderMode(providerMode) ? providerMode : undefined;
		return {
			title: t("guidance.providerUnavailable.title", {
				defaultValue: "Secure store provider unavailable",
			}),
			description: t("guidance.providerUnavailable.description", {
				defaultValue:
					"The configured root-key provider could not be initialized. Retry after fixing the environment, or choose a different security mode in Settings → Security.",
			}),
			technicalDetail,
			actions: mode
				? ["retry_provider", "open_security_settings"]
				: ["retry_status", "open_security_settings"],
			retryProviderMode: mode,
		};
	}

	if (reasonCode === "read_lock_failed") {
		return {
			title: t("guidance.readLockFailed.title", {
				defaultValue: "Secure store is busy",
			}),
			description: t("guidance.readLockFailed.description", {
				defaultValue:
					"MCPMate could not read the secure store status. Wait a moment and retry.",
			}),
			technicalDetail,
			actions: ["retry_status"],
		};
	}

	if (reasonCode === "missing_root_key") {
		const mode = isSwitchableSecretStoreProviderMode(providerMode) ? providerMode : undefined;
		return {
			title: t("guidance.missingRootKey.title", {
				defaultValue: "Root key material is missing",
			}),
			description: t("guidance.missingRootKey.description", {
				defaultValue:
					"Existing encrypted secrets need the original root key material. Restore access to the configured provider before editing stored secrets or switching encryption mode.",
			}),
			technicalDetail,
			actions: mode
				? ["retry_provider", "open_security_settings"]
				: ["retry_status", "open_security_settings"],
			retryProviderMode: mode,
		};
	}

	return {
		title: t("guidance.generic.title", {
			defaultValue: "Secure store unavailable",
		}),
		description: t("guidance.generic.description", {
			defaultValue:
				"Secret storage is not ready. Create and update operations stay disabled until the issue is resolved.",
		}),
		technicalDetail,
		actions: ["retry_status", "open_security_settings"],
	};
}
