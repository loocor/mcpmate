export const MAX_SERVER_NAMESPACE_LENGTH = 64;
export const SERVER_NAMESPACE_PATTERN =
	/^[a-z][a-z0-9]*(?:_[a-z0-9]+)*$/;

export function isCanonicalServerNamespace(value: string): boolean {
	return (
		value.length >= 1 &&
		value.length <= MAX_SERVER_NAMESPACE_LENGTH &&
		SERVER_NAMESPACE_PATTERN.test(value)
	);
}

export function suggestServerNamespace(input: string): string | null {
	const trimmed = input.trim();
	if (!trimmed) return null;

	let suggestion = "";
	let separatorPending = false;
	for (const character of trimmed) {
		if (/^[A-Za-z0-9]$/.test(character)) {
			if (separatorPending && suggestion) suggestion += "_";
			separatorPending = false;
			suggestion += character.toLowerCase();
			continue;
		}
		if (
			character === "-" ||
			character === "." ||
			character === "_" ||
			[" ", "\t", "\n", "\v", "\f", "\r"].includes(character)
		) {
			separatorPending = Boolean(suggestion);
			continue;
		}
		return null;
	}

	return isCanonicalServerNamespace(suggestion) ? suggestion : null;
}

export function namespaceInputIsReadOnly(
	mode: "create" | "edit" | "market",
	hasPendingOAuthServer: boolean,
	remediationAllowed = false,
): boolean {
	return hasPendingOAuthServer || (mode === "edit" && !remediationAllowed);
}

export function serverNamespaceImportPreview(
	original: string | undefined,
	namespace: string,
): { original: string; namespace: string } | null {
	if (!original || original === namespace) return null;
	return { original, namespace };
}
