import type {
	CapabilityKindSummary,
	ServerCapabilitySummary,
	SnapshotState,
} from "./types";

export type CapabilityLifecycleState =
	| "unavailable"
	| "unsupported"
	| "unknown"
	| "empty"
	| "ready";

export type CapabilitySummaryKind =
	| "tools"
	| "prompts"
	| "resources"
	| "resourceTemplates";

export type CapabilityLifecycleLabels = Record<CapabilityLifecycleState, string>;

export function resolveCapabilityLifecycle(
	snapshotState: SnapshotState,
	kind: CapabilityKindSummary,
): CapabilityLifecycleState {
	if (
		snapshotState === "unavailable" ||
		snapshotState === "invalidated" ||
		kind.inventory === "failed"
	) {
		return "unavailable";
	}

	if (kind.declaration === "unsupported") {
		return "unsupported";
	}

	if (
		kind.declaration === "unknown" ||
		kind.inventory === "unknown"
	) {
		return "unknown";
	}

	if (!kind.currentAvailable) {
		return "unavailable";
	}

	return kind.currentCount === 0 ? "empty" : "ready";
}

export function getCapabilityLifecycle(
	summary: ServerCapabilitySummary | undefined,
	kind: CapabilitySummaryKind,
): { state: CapabilityLifecycleState; count: number | null } {
	if (!summary) {
		return { state: "unknown", count: null };
	}

	const kindSummary = summary[kind];
	return {
		state: resolveCapabilityLifecycle(summary.snapshotState, kindSummary),
		count: kindSummary.currentCount,
	};
}

export function formatCapabilityLifecycle(
	summary: ServerCapabilitySummary | undefined,
	kind: CapabilitySummaryKind,
	labels: CapabilityLifecycleLabels,
): string {
	const lifecycle = getCapabilityLifecycle(summary, kind);
	const label = labels[lifecycle.state];
	return lifecycle.count === null ? label : `${lifecycle.count} · ${label}`;
}
