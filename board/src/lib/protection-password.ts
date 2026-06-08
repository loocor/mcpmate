export type ProtectionLevel = "off" | "startup" | "settings";

export function resolveProtectionLevel(data?: {
	enabled: boolean;
	has_password: boolean;
	scope: string[];
}): ProtectionLevel {
	if (!data?.enabled || !data.has_password) {
		return "off";
	}
	if (data.scope.includes("settings") && !data.scope.includes("startup")) {
		return "settings";
	}
	return "startup";
}

export function protectionScopeForLevel(level: Exclude<ProtectionLevel, "off">): string[] {
	return level === "settings" ? ["settings"] : ["startup"];
}

export function requiresStartupPasswordGate(data?: {
	enabled: boolean;
	has_password: boolean;
	scope: string[];
}): boolean {
	return Boolean(data?.enabled && data.has_password && data.scope.includes("startup"));
}

export function requiresSettingsPasswordGate(data?: {
	enabled: boolean;
	has_password: boolean;
	scope: string[];
}): boolean {
	if (!data?.enabled || !data.has_password || !data.scope.includes("settings")) {
		return false;
	}
	if (data.scope.includes("startup") && sessionStorage.getItem("mcp_password_verified") === "true") {
		return false;
	}
	return sessionStorage.getItem("mcp_password_settings_verified") !== "true";
}

export function requiresEncryptionUnlock(status?: {
	status: string;
	provider?: { provider_mode: string } | null;
	issue?: { reason_code: string } | null;
}): boolean {
	if (!status) {
		return false;
	}
	if (status.issue?.reason_code === "passphrase_unlock_required") {
		return true;
	}
	return (
		status.status !== "ready" &&
		status.provider?.provider_mode === "passphrase"
	);
}
