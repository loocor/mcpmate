import type { DashboardSettings } from "./store";
import type { ClientBackupPolicyPayload } from "./types";

export function mapDashboardSettingsToClientBackupPolicy(
	settings: DashboardSettings,
): ClientBackupPolicyPayload {
	if (settings.clientBackupStrategy === "none") {
		return { policy: "off" };
	}

	if (settings.clientBackupStrategy === "keep_last") {
		return { policy: "keep_last" };
	}

	return {
		policy: "keep_n",
		limit: Math.max(1, Math.round(settings.clientBackupLimit || 1)),
	};
}
