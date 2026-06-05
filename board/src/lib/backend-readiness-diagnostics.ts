export interface BackendReadinessAttempt {
	attempt: number;
	lastReportedAtMs: number | null;
	lastStatusKey: string | null;
	nowMs: number;
	statusKey: string;
	throttleMs?: number;
}

const DEFAULT_READINESS_REPORT_THROTTLE_MS = 30_000;

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
		return "error";
	}
	return "unknown";
}
