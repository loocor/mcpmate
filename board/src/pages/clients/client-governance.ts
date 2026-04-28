import type { ClientInfo } from "../../lib/types";

export type ClientGovernanceStatus = "allowed" | "pending" | "denied";

export type ClientAttentionClasses = {
	cardClassName: string;
	titleClassName: string;
};

export function getGovernanceStatus(
	client: ClientInfo,
): ClientGovernanceStatus {
	if (client.approval_status === "approved") return "allowed";
	if (client.approval_status === "suspended") return "denied";
	return "pending";
}

export function getClientAttentionClasses(
	status: ClientGovernanceStatus,
): ClientAttentionClasses {
	if (status === "pending") {
		return {
			cardClassName:
				"border-amber-300/90 hover:border-amber-400 dark:border-amber-700/80 dark:hover:border-amber-600",
			titleClassName: "text-amber-700 dark:text-amber-400",
		};
	}

	if (status === "denied") {
		return {
			cardClassName:
				"border-red-300/90 hover:border-red-400 dark:border-red-800/80 dark:hover:border-red-700",
			titleClassName: "text-red-700 dark:text-red-400",
		};
	}

	return {
		cardClassName: "",
		titleClassName: "",
	};
}
