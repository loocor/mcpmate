export type ClientFormMode = "create" | "edit";

export const CLIENT_IDENTIFIER_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

export function normalizeClientIdentifier(value: string): string {
	return value
		.trim()
		.toLowerCase()
		.replace(/[\s_]+/g, "-")
		.replace(/[^a-z0-9-]+/g, "")
		.replace(/-+/g, "-")
		.replace(/^-+|-+$/g, "");
}

export function sanitizeClientIdentifierInput(value: string): string {
	return value
		.trimStart()
		.toLowerCase()
		.replace(/[\s_]+/g, "-")
		.replace(/[^a-z0-9-]+/g, "")
		.replace(/-+/g, "-")
		.replace(/^-+/, "");
}

export function resolveClientIdentifierForSave(
	mode: ClientFormMode,
	formIdentifier: string,
	persistedIdentifier?: string,
): string {
	if (mode === "create") return normalizeClientIdentifier(formIdentifier);
	return persistedIdentifier ?? formIdentifier;
}
