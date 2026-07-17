export type InspectorOperationKind =
	| "tool"
	| "prompt"
	| "resource"
	| "template";

export type InspectorOperationMode = "native" | "proxy";

export function collectLoadedInspectorOptions<T>(
	options: Partial<Record<InspectorOperationKind, T[] | undefined>>,
): Partial<Record<InspectorOperationKind, T[]>> {
	const loaded: Partial<Record<InspectorOperationKind, T[]>> = {};
	for (const kind of ["tool", "prompt", "resource", "template"] as const) {
		const optionList = options[kind];
		if (optionList !== undefined) {
			loaded[kind] = optionList;
		}
	}
	return loaded;
}

const OPERATION_LABEL_KEYS: Record<InspectorOperationKind, string> = {
	tool: "modes.toolCall",
	prompt: "modes.getPrompt",
	resource: "modes.readResource",
	template: "modes.readTemplate",
};

const PRIMARY_ACTION_KEYS: Record<InspectorOperationKind, string> = {
	tool: "actions.call",
	prompt: "actions.get",
	resource: "actions.read",
	template: "actions.read",
};

export function getInspectorOperationLabelKey(
	kind: InspectorOperationKind,
): string {
	return OPERATION_LABEL_KEYS[kind];
}

export function getInspectorPrimaryActionKey(
	kind: InspectorOperationKind,
): string {
	return PRIMARY_ACTION_KEYS[kind];
}

export function shouldAutoLoadInspectorOptions(state: {
	canUseCurrentMode: boolean;
	hasAttemptedAutoLoad: boolean;
	hasListedOptions: boolean;
	isDrawerInitialized: boolean;
	isProxyChecking: boolean;
	open: boolean;
}): boolean {
	return (
		state.open &&
		state.isDrawerInitialized &&
		state.canUseCurrentMode &&
		!state.isProxyChecking &&
		!state.hasListedOptions &&
		!state.hasAttemptedAutoLoad
	);
}

export function switchInspectorOperationSnapshot<T>(
	snapshots: Map<string, T>,
	mode: InspectorOperationMode,
	currentKind: InspectorOperationKind,
	nextKind: InspectorOperationKind,
	currentSnapshot: T,
): T | undefined {
	snapshots.set(`${mode}:${currentKind}`, currentSnapshot);
	return snapshots.get(`${mode}:${nextKind}`);
}

export function shouldOfferCustomCapabilityValue(
	query: string,
	listedValues: string[],
): boolean {
	const candidate = query.trim();
	return candidate.length > 0 && !listedValues.includes(candidate);
}

function nonEmptyString(value: unknown): string | undefined {
	return typeof value === "string" && value.length > 0 ? value : undefined;
}

export function getInspectorModeIdentity(
	kind: InspectorOperationKind,
	mode: InspectorOperationMode,
	source: Record<string, unknown>,
): string {
	if (kind === "tool") {
		return mode === "proxy"
			? (nonEmptyString(source.unique_name) ?? "")
			: (nonEmptyString(source.tool_name) ??
				nonEmptyString(source.name) ??
				"");
	}

	if (kind === "prompt") {
		return mode === "proxy"
			? (nonEmptyString(source.unique_name) ?? "")
			: (nonEmptyString(source.prompt_name) ??
				nonEmptyString(source.name) ??
				"");
	}

	if (kind === "resource") {
		return mode === "proxy"
			? (nonEmptyString(source.unique_uri) ?? "")
			: (nonEmptyString(source.resource_uri) ??
				nonEmptyString(source.uri) ??
				"");
	}

	return mode === "proxy"
		? (nonEmptyString(source.unique_uri_template) ??
				nonEmptyString(source.unique_name) ??
				"")
		: (nonEmptyString(source.uri_template) ??
				nonEmptyString(source.uriTemplate) ??
				"");
}

export function resolveInspectorCounterpartIdentity(
	kind: InspectorOperationKind,
	sourceMode: InspectorOperationMode,
	targetMode: InspectorOperationMode,
	sourceIdentity: string,
	mappings: Record<string, unknown>[],
): string {
	if (!sourceIdentity || sourceMode === targetMode) {
		return "";
	}

	const mapping = mappings.find(
		(candidate) =>
			getInspectorModeIdentity(kind, sourceMode, candidate) === sourceIdentity,
	);
	return mapping
		? getInspectorModeIdentity(kind, targetMode, mapping)
		: "";
}

export function normalizeInspectorCapabilityOption(
	kind: InspectorOperationKind,
	mode: InspectorOperationMode,
	source: Record<string, unknown>,
): Record<string, unknown> {
	const normalized = { ...source };
	if (mode !== "proxy") {
		return normalized;
	}

	if (kind === "tool" || kind === "prompt") {
		const canonicalName =
			nonEmptyString(source.unique_name) ?? nonEmptyString(source.name);
		if (canonicalName) {
			normalized.unique_name = canonicalName;
		}
		return normalized;
	}

	if (kind === "resource") {
		const canonicalUri =
			nonEmptyString(source.unique_uri) ?? nonEmptyString(source.uri);
		if (canonicalUri) {
			normalized.unique_uri = canonicalUri;
		}
		return normalized;
	}

	const canonicalTemplate =
		nonEmptyString(source.unique_uri_template) ??
		nonEmptyString(source.unique_name) ??
		nonEmptyString(source.uriTemplate) ??
		nonEmptyString(source.uri_template);
	if (canonicalTemplate) {
		normalized.unique_uri_template = canonicalTemplate;
	}
	return normalized;
}
