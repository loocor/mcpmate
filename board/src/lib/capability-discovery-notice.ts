import { notifyError } from "./notify";
import type { ServerCapabilityDiscovery } from "./types";

type Translate = (
	key: string,
	options?: { defaultValue?: string; message?: string },
) => string;

export function notifyCapabilityDiscoveryFailure(
	capabilityDiscovery: ServerCapabilityDiscovery | undefined,
	t: Translate,
) {
	if (
		!capabilityDiscovery?.attempted ||
		capabilityDiscovery.status !== "failed"
	) {
		return;
	}

	const message =
		capabilityDiscovery.error?.trim() ||
		t("detail.notifications.refreshFailed.defaultMessage", {
			defaultValue: "Unknown error",
		});
	notifyCapabilityRefreshFailure(t, message);
}

export function notifyCapabilityRefreshFailure(t: Translate, message: string) {
	notifyError(
		t("detail.notifications.refreshFailed.title", {
			defaultValue: "Refresh failed",
		}),
		t("detail.notifications.refreshFailed.message", {
			message,
			defaultValue: "Unable to refresh server capabilities: {{message}}",
		}),
	);
}
