export type InspectorFeatureTab =
	| "inspect"
	| "compatibility"
	| "package_safety"
	| "llm_evaluation";

export type InspectorFooterWorkspace = "connect" | "configuration";

export type InspectorWorkspaceView = InspectorFeatureTab | InspectorFooterWorkspace;

export const INSPECTOR_WORKSPACE_MODE_LABELS: Record<InspectorWorkspaceView, string> = {
	inspect: "Inspect",
	compatibility: "Compat",
	package_safety: "Safety",
	llm_evaluation: "LLM",
	connect: "Connect",
	configuration: "Configuration",
};

export function inspectorWorkspaceModeLabel(view: InspectorWorkspaceView): string {
	return INSPECTOR_WORKSPACE_MODE_LABELS[view];
}

export type InspectorCapabilityFamily =
	| "tools"
	| "prompts"
	| "resources"
	| "resource_templates"
	| "apps"
	| "tasks"
	| "extensions";

export type InspectorCompatibilitySpecVersion =
	| "2024-11-05"
	| "2025-03-26"
	| "2025-11-25"
	| "2026-07-01";

export type InspectorPackageSafetyFactSource = "runtime_cache" | "server_manifest";

export type InspectorPackageSafetyDatabase = "osv" | "npm_audit" | "combined";

export type InspectorPackageSafetyScanDepth = "quick" | "standard" | "deep";

export type InspectorLlmEvaluationFocus =
	| "compatibility"
	| "package_safety"
	| "capabilities"
	| "runtime";

export type InspectorCapabilityListItem = {
	key: string;
	title: string;
	description?: string;
	inputSchema?: Record<string, unknown>;
	outputSchema?: Record<string, unknown>;
};

export type InspectorCapabilityFamilyState = {
	listed: boolean;
	listing: boolean;
	items: InspectorCapabilityListItem[];
	selectedKey: string | null;
};

export type InspectorCapabilityFamilyOption = {
	value: InspectorCapabilityFamily;
	label: string;
	shortLabel?: string;
	listMethod: string;
	placeholder?: boolean;
	advertisedCount?: number | null;
};

export type InspectorConfigurationState = {
	autoListOnFamilySwitch: boolean;
	sessionIdleTimeoutMinutes: number;
	defaultTransportMode: "native" | "proxy" | "bridge";
	reconnectOnExpiry: boolean;
	requestTimeoutMs: number;
	resetTimeoutOnProgress: boolean;
	maxTotalTimeoutMs: number;
};

export const DEFAULT_INSPECTOR_CONFIGURATION: InspectorConfigurationState = {
	autoListOnFamilySwitch: false,
	sessionIdleTimeoutMinutes: 30,
	defaultTransportMode: "native",
	reconnectOnExpiry: true,
	requestTimeoutMs: 300000,
	resetTimeoutOnProgress: true,
	maxTotalTimeoutMs: 600000,
};

export const INSPECTOR_REQUEST_TIMEOUT_PRESETS: Array<{
	value: string;
	label: string;
}> = [
	{ value: "10000", label: "10 seconds" },
	{ value: "30000", label: "30 seconds" },
	{ value: "60000", label: "60 seconds" },
	{ value: "300000", label: "5 minutes" },
	{ value: "600000", label: "10 minutes" },
];

export const INSPECTOR_MAX_TOTAL_TIMEOUT_PRESETS: Array<{
	value: string;
	label: string;
}> = [
	{ value: "60000", label: "60 seconds" },
	{ value: "300000", label: "5 minutes" },
	{ value: "600000", label: "10 minutes" },
	{ value: "1800000", label: "30 minutes" },
	{ value: "3600000", label: "60 minutes" },
];

export const INSPECTOR_CAPABILITY_FAMILIES: InspectorCapabilityFamilyOption[] = [
	{ value: "tools", label: "Tools", listMethod: "tools/list" },
	{ value: "prompts", label: "Prompts", listMethod: "prompts/list" },
	{ value: "resources", label: "Resources", listMethod: "resources/list" },
	{
		value: "resource_templates",
		label: "Resource Templates",
		shortLabel: "R·Template",
		listMethod: "resources/templates/list",
	},
	{
		value: "apps",
		label: "Apps",
		listMethod: "apps/list",
		placeholder: true,
	},
	{
		value: "tasks",
		label: "Tasks",
		listMethod: "tasks/list",
		placeholder: true,
	},
	{
		value: "extensions",
		label: "Extensions",
		listMethod: "extensions/list",
		placeholder: true,
	},
];

export function createEmptyCapabilityFamilyState(): InspectorCapabilityFamilyState {
	return {
		listed: false,
		listing: false,
		items: [],
		selectedKey: null,
	};
}

export function createInitialCapabilityFamilyStates(): Record<
	InspectorCapabilityFamily,
	InspectorCapabilityFamilyState
> {
	return Object.fromEntries(
		INSPECTOR_CAPABILITY_FAMILIES.map((family) => [
			family.value,
			createEmptyCapabilityFamilyState(),
		]),
	) as Record<InspectorCapabilityFamily, InspectorCapabilityFamilyState>;
}

export const INSPECTOR_COMPATIBILITY_SPEC_BASELINE_NOTE =
	"Choose the MCP specification baseline to compare against this server.";

export const INSPECTOR_COMPATIBILITY_SPEC_VERSION_TOOLTIP =
	"Choose the MCP specification revision used as the compatibility baseline.";

export const INSPECTOR_COMPATIBILITY_SPEC_OPTIONS: Array<{
	value: InspectorCompatibilitySpecVersion;
	label: string;
	segmentLabel: string;
	description: string;
	highlights: string;
	specUrl?: string;
}> = [
	{
		value: "2024-11-05",
		label: "2024-11-05",
		segmentLabel: "2024-11",
		description: "Initial MCP specification release.",
		highlights: "Core JSON-RPC transport, tools, prompts, and resources.",
		specUrl: "https://modelcontextprotocol.io/specification/2024-11-05",
	},
	{
		value: "2025-03-26",
		label: "2025-03-26",
		segmentLabel: "2025-03",
		description: "March 2025 protocol revision.",
		highlights: "Streamable HTTP transport and expanded capability negotiation.",
		specUrl: "https://modelcontextprotocol.io/specification/2025-03-26",
	},
	{
		value: "2025-11-25",
		label: "2025-11-25",
		segmentLabel: "2025-11",
		description: "November 2025 protocol revision.",
		highlights: "Tasks, apps, and richer extension surfaces for server review.",
		specUrl: "https://modelcontextprotocol.io/specification/2025-11-25",
	},
	{
		value: "2026-07-01",
		label: "2026-07 (draft)",
		segmentLabel: "2026-07",
		description: "Upcoming July 2026 revision.",
		highlights: "Draft baseline for forward-looking compatibility checks.",
	},
];

export const INSPECTOR_PACKAGE_SAFETY_FACT_SOURCES: Array<{
	value: InspectorPackageSafetyFactSource;
	label: string;
	segmentLabel: string;
	description: string;
}> = [
	{
		value: "runtime_cache",
		label: "Runtime cache index",
		segmentLabel: "Cache",
		description: "Use package metadata from the runtime cache.",
	},
	{
		value: "server_manifest",
		label: "Server manifest",
		segmentLabel: "Manifest",
		description: "Use package facts recorded by the server.",
	},
];

export const INSPECTOR_PACKAGE_SAFETY_FACT_SOURCE_TOOLTIP =
	"Choose where package metadata is loaded before advisory lookup runs.";

export const INSPECTOR_PACKAGE_SAFETY_DATABASES: Array<{
	value: InspectorPackageSafetyDatabase;
	label: string;
	segmentLabel: string;
	description: string;
}> = [
	{
		value: "combined",
		label: "Combined sources",
		segmentLabel: "All",
		description: "Query OSV and npm audit together.",
	},
	{
		value: "osv",
		label: "OSV",
		segmentLabel: "OSV",
		description: "Open Source Vulnerabilities database only.",
	},
	{
		value: "npm_audit",
		label: "npm audit",
		segmentLabel: "NPM",
		description: "npm advisory reports only.",
	},
];

export const INSPECTOR_PACKAGE_SAFETY_DATABASE_TOOLTIP =
	"Choose which advisory database feeds vulnerability findings for this scan.";

export const INSPECTOR_PACKAGE_SAFETY_SETTINGS_NOTE =
	"Choose fact source, advisory database, and scan depth before running a package safety scan.";

export const INSPECTOR_PACKAGE_SAFETY_SCAN_DEPTHS: Array<{
	value: InspectorPackageSafetyScanDepth;
	label: string;
	segmentLabel: string;
	description: string;
}> = [
	{
		value: "quick",
		label: "Quick",
		segmentLabel: "Quick",
		description: "Surface-level dependency checks.",
	},
	{
		value: "standard",
		label: "Default",
		segmentLabel: "Default",
		description: "Balanced coverage for routine review.",
	},
	{
		value: "deep",
		label: "Deep",
		segmentLabel: "Deep",
		description: "Full transitive scan and advisory lookup.",
	},
];

export const INSPECTOR_PACKAGE_SAFETY_SCAN_DEPTH_TOOLTIP =
	"Control how far the scanner walks dependencies and advisory records.";

export const INSPECTOR_LLM_EVALUATION_SETTINGS_NOTE =
	"Reuse prior scan facts and choose what the model should analyze.";

export const INSPECTOR_LLM_EVALUATION_FOCUS_TOOLTIP =
	"Choose which prior scan facts the model should include in its evaluation.";

export const INSPECTOR_LLM_EVALUATION_PROVIDER_TOOLTIP =
	"Choose which configured LLM provider runs this evaluation. The workspace default provider is selected initially.";

export const INSPECTOR_LLM_EVALUATION_FOCUS_OPTIONS: Array<{
	value: InspectorLlmEvaluationFocus;
	label: string;
	segmentLabel: string;
	description: string;
}> = [
	{
		value: "capabilities",
		label: "Capabilities",
		segmentLabel: "Capabilities",
		description: "Include tools, prompts, and resources evidence.",
	},
	{
		value: "compatibility",
		label: "Compatibility",
		segmentLabel: "Compat",
		description: "Reuse compatibility scan facts.",
	},
	{
		value: "package_safety",
		label: "Package safety",
		segmentLabel: "Safety",
		description: "Reuse package safety scan facts.",
	},
	{
		value: "runtime",
		label: "Runtime health",
		segmentLabel: "Runtime",
		description: "Include connection and runtime signals.",
	},
];

export const DEFAULT_INSPECTOR_LLM_EVALUATION_FOCUS: InspectorLlmEvaluationFocus[] = [
	"capabilities",
	"compatibility",
	"package_safety",
];
