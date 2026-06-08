import type { SecretOrigin } from "./types";

const ALIAS_SEGMENT_PATTERN = /[^a-z0-9]+/g;

export function slugifySecretSegment(value: string): string {
	return value
		.toLowerCase()
		.replace(ALIAS_SEGMENT_PATTERN, "-")
		.replace(/^-+|-+$/g, "");
}

function objectTypeSlugFromOrigin(origin: SecretOrigin): string {
	if (
		origin.source?.startsWith("server_") ||
		origin.server_id ||
		origin.server_name
	) {
		return "server";
	}
	if (origin.source?.trim()) {
		return slugifySecretSegment(origin.source) || "secret";
	}
	return "secret";
}

function objectNameSlugFromOrigin(origin: SecretOrigin): string {
	const raw =
		origin.server_name?.trim() ||
		origin.server_id?.trim() ||
		"secret";
	return slugifySecretSegment(raw) || "secret";
}

function fieldGroupSlugFromOrigin(origin: SecretOrigin): string {
	const fieldGroup = origin.field_group?.trim().toLowerCase();
	const fieldKey = origin.field_key?.trim().toLowerCase();

	switch (fieldGroup) {
		case "url_params":
			return "url-parameters";
		case "env":
			return "env";
		case "headers":
			return "headers";
		case "args":
			return "args";
		case "stdio":
			return fieldKey === "command" || origin.field_path === "command"
				? "command"
				: "stdio";
		case "streamable_http":
			return fieldKey === "url" || origin.field_path === "url" ? "url" : "streamable-http";
		default:
			if (fieldGroup) {
				return slugifySecretSegment(fieldGroup) || "field";
			}
			return "field";
	}
}

function keySlugFromOrigin(origin: SecretOrigin): string {
	const fieldKey = origin.field_key?.trim();
	if (fieldKey) {
		const keySlug = slugifySecretSegment(fieldKey);
		if (keySlug) return keySlug;
	}
	const fieldIndex = origin.field_index ?? 0;
	return `k${fieldIndex + 1}`;
}

function isSingletonField(origin: SecretOrigin): boolean {
	const fieldGroup = origin.field_group?.trim().toLowerCase();
	const fieldKey = origin.field_key?.trim().toLowerCase();
	return (
		(fieldGroup === "stdio" && fieldKey === "command") ||
		(fieldGroup === "streamable_http" && fieldKey === "url")
	);
}

export function generateSecretAliasFromOrigin(origin: SecretOrigin): string {
	const parts = [
		objectTypeSlugFromOrigin(origin),
		objectNameSlugFromOrigin(origin),
		fieldGroupSlugFromOrigin(origin),
	];
	if (!isSingletonField(origin)) {
		parts.push(keySlugFromOrigin(origin));
	}
	return parts.filter(Boolean).join("-");
}

export function resolveUniqueSecretAlias(
	baseAlias: string,
	existingAliases: Iterable<string>,
): string {
	const normalizedBase = baseAlias.trim();
	if (!normalizedBase) return normalizedBase;

	const existing = new Set(existingAliases);
	if (!existing.has(normalizedBase)) {
		return normalizedBase;
	}

	let suffix = 2;
	while (existing.has(`${normalizedBase}-${suffix}`)) {
		suffix += 1;
	}
	return `${normalizedBase}-${suffix}`;
}

export function suggestSecretAliasFromOrigin(
	origin: SecretOrigin,
	existingAliases: Iterable<string>,
): string {
	const baseAlias = generateSecretAliasFromOrigin(origin);
	return resolveUniqueSecretAlias(baseAlias, existingAliases);
}
