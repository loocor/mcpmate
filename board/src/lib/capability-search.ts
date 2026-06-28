import type { CapabilityRecord } from "../types/capabilities";

function fuzzyTextMatches(value: string, query: string): boolean {
	const haystack = value.toLowerCase();
	const needle = query.trim().toLowerCase();
	if (!needle) return true;
	if (haystack.includes(needle)) return true;

	const normalizedHaystack = haystack.replace(/[^a-z0-9]+/g, " ");
	const tokens = needle.split(/[^a-z0-9]+/).filter(Boolean);
	if (tokens.length && tokens.every((token) => normalizedHaystack.includes(token))) {
		return true;
	}

	const compactHaystack = haystack.replace(/[^a-z0-9]+/g, "");
	const compactNeedle = needle.replace(/[^a-z0-9]+/g, "");
	return Boolean(compactNeedle && compactHaystack.includes(compactNeedle));
}

export function capabilityRecordMatchesSearch(
	item: CapabilityRecord,
	query: string,
): boolean {
	if (!query.trim()) return true;
	const fields = [
		item.name,
		item.unique_name,
		item.tool_name,
		item.prompt_name,
		item.resource_uri,
		item.uri,
		item.uriTemplate,
		item.uri_template,
		item.description,
		item.server_name,
	]
		.filter((value): value is string => typeof value === "string")
		.join(" ");
	return fuzzyTextMatches(fields, query);
}
