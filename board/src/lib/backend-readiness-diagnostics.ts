export interface BackendReadinessAttempt {
	attempt: number;
	lastReportedAtMs: number | null;
	lastStatusKey: string | null;
	nowMs: number;
	statusKey: string;
	throttleMs?: number;
}

export interface BackendReadinessIssue {
	detail: string;
	kind:
		| "backend_starting"
		| "core_stopped"
		| "core_unhealthy"
		| "error"
		| "network_error"
		| "not_ready"
		| "unknown";
	messageKey: BackendReadinessIssueMessageKey;
	messageParams?: Record<string, string>;
	statusKey: string;
}

export interface CoreStartupSnapshot {
	apiBaseUrl?: string;
	localService?: {
		detail?: string;
		label?: string;
		running?: boolean;
		status?: string;
	};
}

const DEFAULT_READINESS_REPORT_THROTTLE_MS = 30_000;

type BackendReadinessIssueMessageKey =
	| "backendStarting"
	| "coreService"
	| "networkError"
	| "notReady"
	| "unknown";

type TranslateBackendReadinessIssue = (
	key: string,
	options: Record<string, string>,
) => string;

export function shouldReportBackendReadinessAttempt(
	attempt: BackendReadinessAttempt,
): boolean {
	if (attempt.attempt <= 1 || attempt.lastReportedAtMs === null) {
		return true;
	}
	if (attempt.lastStatusKey !== attempt.statusKey) {
		return true;
	}
	const throttleMs = attempt.throttleMs ?? DEFAULT_READINESS_REPORT_THROTTLE_MS;
	return attempt.nowMs - attempt.lastReportedAtMs >= throttleMs;
}

export function backendReadinessStatusKey(
	payload: { status?: string; type?: string } | null,
	error: unknown,
): string {
	if (payload?.type || payload?.status) {
		return `${payload.type ?? "unknown"}:${payload.status ?? "unknown"}`;
	}
	if (error instanceof TypeError) {
		return "network_error";
	}
	if (error instanceof Error) {
		if (error.message === "Backend is starting") {
			return "backend_starting";
		}
		return "error";
	}
	return "unknown";
}

export function describeBackendReadinessIssue(
	payload: { reason?: string; status?: string; type?: string } | null,
	error: unknown,
	apiBaseUrl?: string,
): BackendReadinessIssue | null {
	if (payload?.type === "ready" && payload.status === "ok") {
		return null;
	}

	const statusKey = backendReadinessStatusKey(payload, error);
	if (payload?.type || payload?.status) {
		const reason = payload.reason ? ` (${payload.reason})` : "";
		return {
			kind: "not_ready",
			detail: `Backend readiness is ${statusKey}${reason}`,
			messageKey: "notReady",
			messageParams: {
				reason,
				statusKey,
			},
			statusKey,
		};
	}

	if (error instanceof TypeError) {
		const target = apiBaseUrl ? ` at ${apiBaseUrl}` : "";
		return {
			kind: "network_error",
			detail: `Backend API is not reachable${target}: ${error.message}`,
			messageKey: "networkError",
			messageParams: {
				error: error.message,
				target,
			},
			statusKey,
		};
	}

	if (error instanceof Error) {
		if (error.message === "Backend is starting") {
			const target = apiBaseUrl ? ` at ${apiBaseUrl}` : "";
			return {
				kind: "backend_starting",
				detail: `Backend process is starting${target}`,
				messageKey: "backendStarting",
				messageParams: {
					target,
				},
				statusKey: "backend_starting",
			};
		}
		return {
			kind: "error",
			detail: error.message,
			messageKey: "unknown",
			messageParams: {
				detail: error.message,
			},
			statusKey,
		};
	}

	return {
		kind: "unknown",
		detail: "Backend readiness has not completed yet",
		messageKey: "unknown",
		messageParams: {
			detail: "Backend readiness has not completed yet",
		},
		statusKey,
	};
}

export function describeCoreStartupIssue(
	snapshot: CoreStartupSnapshot,
): BackendReadinessIssue | null {
	const service = snapshot.localService;
	if (!service?.status) {
		return null;
	}

	if (service.status === "running") {
		return null;
	}

	const label = service.label ? `${service.label}: ` : "";
	const detail = `${label}${service.detail ?? "Local core is not ready yet"}`;
	if (service.status === "running_unhealthy") {
		return {
			kind: "core_unhealthy",
			detail,
			messageKey: "coreService",
			messageParams: {
				detail: service.detail ?? "Local core is not ready yet",
				label,
			},
			statusKey: "core:running_unhealthy",
		};
	}

	if (service.status === "stopped" || service.status === "not_installed") {
		return {
			kind: "core_stopped",
			detail,
			messageKey: "coreService",
			messageParams: {
				detail: service.detail ?? "Local core is not ready yet",
				label,
			},
			statusKey: `core:${service.status}`,
		};
	}

	return {
		kind: "unknown",
		detail,
		messageKey: "coreService",
		messageParams: {
			detail: service.detail ?? "Local core is not ready yet",
			label,
		},
		statusKey: `core:${service.status}`,
	};
}

export function translateBackendReadinessIssue(
	t: TranslateBackendReadinessIssue,
	issue: BackendReadinessIssue,
): string {
	return t(`backendReadiness.issue.${issue.messageKey}`, {
		defaultValue: issue.detail,
		...(issue.messageParams ?? {}),
	});
}
