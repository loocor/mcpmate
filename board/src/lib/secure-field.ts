export const REDACTED_FULL = "***REDACTED***";
/** User-facing label for stored secrets in form fields. */
export const STORED_SECRET_DISPLAY = "Stored secret";
/** Canonical secret placeholder for redacted values in JSON preview. */
export const STORED_SECRET_JSON_PLACEHOLDER = "[[secret:stored-secret]]";

const SECRET_PLACEHOLDER_PATTERN = /\[\[secret:([^\]]+)\]\]/g;
const WHOLE_SECRET_PLACEHOLDER_PATTERN = /^\[\[secret:([^\]]+)\]\]$/;
// IMPORTANT: keep in sync with backend/src/config/server/headers.rs is_redacted_display_value.
const PARTIAL_REDACTED_PATTERN = /^.{6}\*\*\*.{2}$/;

export type FieldValueKind = "plain" | "secret_ref" | "redacted";

export function isRedactedMask(value: string): boolean {
	const trimmed = value.trim();
	if (!trimmed) return false;
	if (trimmed === REDACTED_FULL) return true;
	return PARTIAL_REDACTED_PATTERN.test(trimmed);
}

export function containsSecretPlaceholder(value: string): boolean {
	SECRET_PLACEHOLDER_PATTERN.lastIndex = 0;
	return SECRET_PLACEHOLDER_PATTERN.test(value);
}

export function extractWholeSecretAlias(value: string): string | null {
	const match = value.trim().match(WHOLE_SECRET_PLACEHOLDER_PATTERN);
	return match?.[1] ?? null;
}

export function classifyFieldValue(value: string): FieldValueKind {
	if (!value.trim()) return "plain";
	if (isRedactedMask(value)) return "redacted";
	if (containsSecretPlaceholder(value)) return "secret_ref";
	return "plain";
}

export interface BearerSecretParts {
	prefix: string;
	secretAlias: string | null;
	redacted: boolean;
}

export function parseBearerSecretValue(value: string): BearerSecretParts | null {
	const trimmed = value.trim();
	const lower = trimmed.toLowerCase();
	if (!lower.startsWith("bearer")) return null;

	if (isRedactedMask(trimmed)) {
		return { prefix: "Bearer ", secretAlias: null, redacted: true };
	}

	const token = trimmed.startsWith("Bearer ")
		? trimmed.slice("Bearer ".length)
		: trimmed.slice("Bearer".length).trimStart();

	if (isRedactedMask(token)) {
		return { prefix: "Bearer ", secretAlias: null, redacted: true };
	}

	const alias = extractWholeSecretAlias(token);
	if (alias) {
		return { prefix: "Bearer ", secretAlias: alias, redacted: false };
	}

	return { prefix: "Bearer ", secretAlias: null, redacted: false };
}

function formatRedactedToken(value: string, token: string): string | null {
	const bearer = parseBearerSecretValue(value);
	if (bearer?.redacted) return `${bearer.prefix}${token}`;
	if (isRedactedMask(value)) return token;
	return null;
}

export function formatRedactedDisplayValue(
	value: string,
	displayLabel: string = STORED_SECRET_DISPLAY,
): string {
	return formatRedactedToken(value, displayLabel) ?? value;
}

/** JSON preview for redacted values, e.g. `Bearer [[secret:stored-secret]]`. */
export function formatRedactedJsonPreviewValue(value: string): string {
	return formatRedactedToken(value, STORED_SECRET_JSON_PLACEHOLDER) ?? value;
}

export function isAuthorizationHeaderKey(key?: string | null): boolean {
	if (!key) return false;
	const normalized = key.trim().toLowerCase();
	return (
		normalized === "authorization" || normalized === "proxy-authorization"
	);
}

export function resolveSecureFieldVariant(
	value: string,
	headerKey?: string | null,
): "plain" | "whole-secret" | "bearer-secret" | "redacted" | "bearer-redacted" {
	const bearer = parseBearerSecretValue(value);
	if (bearer) {
		if (bearer.redacted) return "bearer-redacted";
		if (bearer.secretAlias) return "bearer-secret";
	}

	if (isRedactedMask(value)) return "redacted";

	const wholeAlias = extractWholeSecretAlias(value);
	if (wholeAlias) return "whole-secret";

	if (isAuthorizationHeaderKey(headerKey) && containsSecretPlaceholder(value)) {
		return "bearer-secret";
	}

	return "plain";
}

export function buildBearerSecretValue(placeholder: string): string {
	return `Bearer ${placeholder.trim()}`;
}

export function sanitizeStringForSave(value: string): string | undefined {
	if (isRedactedMask(value)) return undefined;
	return value;
}

export function sanitizeRecordForSave(
	record?: Record<string, string> | null,
): Record<string, string> | undefined {
	if (!record) return undefined;

	const next: Record<string, string> = {};
	for (const [rawKey, rawValue] of Object.entries(record)) {
		const key = rawKey.trim();
		if (!key) continue;
		const value = rawValue == null ? "" : String(rawValue);
		const sanitized = sanitizeStringForSave(value.trim());
		if (sanitized !== undefined) {
			next[key] = sanitized;
		}
	}

	return Object.keys(next).length ? next : undefined;
}

export function recordHasRedactedValues(
	record?: Record<string, string> | null,
): boolean {
	if (!record) return false;
	return Object.values(record).some((value) => isRedactedMask(String(value ?? "")));
}

export function recordsEqualIgnoringRedacted(
	left?: Record<string, string> | null,
	right?: Record<string, string> | null,
): boolean {
	const leftKeys = Object.keys(left ?? {}).sort();
	const rightKeys = Object.keys(right ?? {}).sort();
	if (leftKeys.length !== rightKeys.length) return false;

	for (const key of leftKeys) {
		const leftValue = String(left?.[key] ?? "");
		const rightValue = String(right?.[key] ?? "");
		if (leftValue === rightValue) continue;
		if (isRedactedMask(leftValue) && isRedactedMask(rightValue)) continue;
		return false;
	}

	return true;
}

/**
 * Resolve env/headers for server update: omit when unchanged, send `{}` when
 * clearing a record that previously had values, otherwise send the new map.
 */
export function resolveRecordUpdatePayload(
	current?: Record<string, string> | null,
	baseline?: Record<string, string> | null,
): Record<string, string> | undefined {
	if (baseline && recordsEqualIgnoringRedacted(current, baseline)) {
		return undefined;
	}
	if (baseline && !current) {
		return {};
	}
	return current ?? undefined;
}
