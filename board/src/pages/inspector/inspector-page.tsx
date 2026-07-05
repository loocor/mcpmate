import {
	GitCompareArrows,
	MessageSquareText,
	Microscope,
	PackageSearch,
	RefreshCcw,
	Route,
	ShieldCheck,
	Waypoints,
} from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { InspectorWindowLayout } from "../../components/layout/inspector-layout";
import { InspectorChromeProvider } from "../../components/layout/inspector-chrome-context";
import { ActivityLogTable } from "../../components/activity-log-table";
import { InspectorBottomPanel } from "../../components/inspector-bottom-panel";
import { INSPECTOR_BOTTOM_BAR_ICON_BUTTON_CLASSNAME } from "../../lib/inspector-bottom-bar";
import { InspectorEventDetailDrawer } from "../../components/inspector-event-detail-drawer";
import { InspectorServerPicker } from "../../components/inspector-server-picker";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Label } from "../../components/ui/label";
import { Segment } from "../../components/ui/segment";
import { Textarea } from "../../components/ui/textarea";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "../../components/ui/tooltip";
import { inspectorApi, isInspectorSessionUnavailableError, serversApi } from "../../lib/api";
import {
	inspectorEventMatchesSearch,
	type InspectorLogEventEntry,
	type InspectorLogTranslate,
	type InspectorMcpListKind,
} from "../../lib/inspector-event-log";
import {
	clearInspectorStandaloneConnectionTargetKey,
	loadInspectorStandaloneConnectionTargetKey,
	saveInspectorStandaloneConnectionTargetKey,
	type InspectorStandaloneConnectionTargetKey,
} from "../../lib/inspector-standalone-storage";
import { useInspectorStandaloneLog } from "../../lib/hooks/use-inspector-standalone-log";
import { useInspectorNativeSession } from "../../lib/hooks/use-inspector-native-session";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { mapInspectorEventToActivityLogRow } from "../../lib/map-inspector-event-to-activity-log-row";
import { notifyError, stringifyError } from "../../lib/notify";
import { urlWithMergedSearchParams } from "../../lib/server-import-payload";
import { formatInspectorServerLabel } from "../../lib/inspector-capability";
import { useAppStore } from "../../lib/store";
import { cn } from "../../lib/utils";
import {
	inspectorSidebarExpandedControlsClassName,
	inspectorConnectWorkspaceClassName,
	inspectorWorkspaceContentClassName,
	sidebarFeatureTabIconSize,
	sidebarIconLabeledActionClassName,
	sidebarIconRailClassName,
	sidebarIconRailGridClassName,
	sidebarNavItemClassName,
	SidebarNavIcon,
} from "../../components/layout/sidebar-nav-item";
import type { ServerCapabilitySummary, ServerSummary } from "../../lib/types";
import {
	capabilityFamilyListMethod,
	fetchInspectorCapabilityList,
} from "./inspector-capability-list-api";
import { InspectorConnectWorkspace } from "./inspector-connect-workspace";
import type { InspectorConnectCandidate } from "./inspector-connect-server-form";
import { InspectorCapabilityWorkspace } from "./inspector-capability-workspace";
import { InspectorConfigurationWorkspace } from "./inspector-configuration-workspace";
import {
	DEFAULT_INSPECTOR_CONFIGURATION,
	DEFAULT_INSPECTOR_LLM_EVALUATION_FOCUS,
	INSPECTOR_CAPABILITY_FAMILIES,
	createEmptyCapabilityFamilyState,
	createInitialCapabilityFamilyStates,
	type InspectorCapabilityFamily,
	type InspectorCapabilityFamilyOption,
	type InspectorCompatibilitySpecVersion,
	type InspectorConfigurationState,
	type InspectorFeatureTab,
	type InspectorFooterWorkspace,
	type InspectorLlmEvaluationFocus,
	type InspectorPackageSafetyDatabase,
	type InspectorPackageSafetyFactSource,
	type InspectorPackageSafetyScanDepth,
	type InspectorWorkspaceView,
	inspectorWorkspaceModeLabel,
} from "./inspector-feature-config";
import { InspectorFeatureSidebarPanel } from "./inspector-feature-sidebar-panel";
import { BrushCleaning } from "./inspector-icons";
import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";

type InspectorSnapshotKind = "compatibility" | "package_safety";

type InspectorScratchServerRecord = {
	id: string;
	name: string;
	config: Record<string, unknown>;
	provenance?: {
		kind?: string;
		origin?: string | null;
		server_id?: string | null;
		server_name?: string | null;
	};
};

type InspectorManagedTarget = {
	source: "managed";
	id: string;
	name: string;
	enabled: boolean;
	serverType?: string;
	capability?: ServerCapabilitySummary;
};

type InspectorScratchTarget = {
	source: "scratch";
	id: string;
	name: string;
	config: Record<string, unknown>;
};

type InspectorTarget = InspectorManagedTarget | InspectorScratchTarget;

type InspectorConnectionMode = "native" | "proxy" | "bridge";

type InspectorSnapshotState = {
	kind: InspectorSnapshotKind;
	payload: Record<string, unknown>;
	loadedAt: string;
};

type InspectorEvaluationState = {
	evaluation: Record<string, unknown>;
	loadedAt: string;
};

const FEATURE_TABS: Array<{
	value: InspectorFeatureTab;
	icon: typeof Microscope;
	label: string;
	shortLabel: string;
	description: string;
}> = [
		{
			value: "inspect",
			icon: Microscope,
			label: "Capabilities",
			shortLabel: "Inspect",
			description: "List and inspect tools, prompts, resources, and templates.",
		},
		{
			value: "compatibility",
			icon: ShieldCheck,
			label: "Compatibility",
			shortLabel: "Compat",
			description: "Check protocol and capability compatibility.",
		},
		{
			value: "package_safety",
			icon: PackageSearch,
			label: "Package Safety",
			shortLabel: "Safety",
			description: "Review dependency and package safety evidence.",
		},
		{
			value: "llm_evaluation",
			icon: MessageSquareText,
			label: "LLM Evaluation",
			shortLabel: "LLM",
			description: "Evaluate behavior with configured model providers.",
		},
	];

const INSPECTOR_TRANSPORT_MODE_OPTIONS: Array<{
	value: InspectorConnectionMode;
	label: string;
	ariaLabel: string;
	icon: React.ReactNode;
	tooltip: string;
	disabled?: boolean;
}> = [
		{
			value: "native",
			label: "Native",
			ariaLabel: "Native",
			icon: <Route className="h-4 w-4" />,
			tooltip: "Connect directly to the selected server through the local native session.",
		},
		{
			value: "proxy",
			label: "Proxy",
			ariaLabel: "Proxy",
			icon: <Waypoints className="h-4 w-4" />,
			tooltip: "Exercise the server through MCPMate proxy routing and profile policy.",
		},
		{
			value: "bridge",
			label: "Bridge",
			ariaLabel: "Bridge",
			icon: <GitCompareArrows className="h-4 w-4" />,
			tooltip: "Simulate the middle layer between a host app and server for bidirectional testing.",
			disabled: true,
		},
	];

function targetKey(target: InspectorTarget): string {
	return `${target.source}:${target.id}`;
}

function targetLabel(target: InspectorTarget | null): string {
	if (!target) return "No target selected";
	const name = formatInspectorServerLabel(target.name, target.id);
	return target.source === "managed" ? name : `Scratch: ${name}`;
}

function hasEntries(value?: Record<string, string>): value is Record<string, string> {
	return Boolean(value && Object.keys(value).length > 0);
}

function buildScratchServerConfig(draft: ServerInstallDraft): Record<string, unknown> {
	const config: Record<string, unknown> = {
		type: draft.kind,
	};
	if (draft.kind === "stdio") {
		if (draft.command) {
			config.command = draft.command;
		}
		if (draft.args?.length) {
			config.args = draft.args;
		}
		if (hasEntries(draft.env)) {
			config.env = draft.env;
		}
		return config;
	}
	if (draft.url) {
		config.url = hasEntries(draft.urlParams)
			? urlWithMergedSearchParams(draft.url, draft.urlParams)
			: draft.url;
	}
	if (hasEntries(draft.headers)) {
		config.headers = draft.headers;
	}
	return config;
}

function snapshotTitle(kind: InspectorSnapshotKind): string {
	return kind === "compatibility"
		? "Compatibility snapshot"
		: "Package safety snapshot";
}

function capabilityFamilyToListKind(
	family: InspectorCapabilityFamily,
): InspectorMcpListKind | null {
	switch (family) {
		case "tools":
			return "tools";
		case "prompts":
			return "prompts";
		case "resources":
			return "resources";
		case "resource_templates":
			return "templates";
		default:
			return null;
	}
}

function managedTargetsFromServers(
	servers: ServerSummary[],
): InspectorManagedTarget[] {
	return servers
		.map((server) => ({
			source: "managed" as const,
			id: server.id,
			name: server.name || server.id,
			enabled: Boolean(server.enabled ?? server.globally_enabled),
			serverType: server.server_type,
			capability: server.capability ?? server.capabilities,
		}))
		.sort((left, right) => left.name.localeCompare(right.name));
}

function capabilityFamilyIsAdvertised(
	capability: ServerCapabilitySummary,
	family: InspectorCapabilityFamily,
): boolean {
	switch (family) {
		case "tools":
			return capability.supports_tools || capability.tools_count > 0;
		case "prompts":
			return capability.supports_prompts || capability.prompts_count > 0;
		case "resources":
			return capability.supports_resources || capability.resources_count > 0;
		case "resource_templates":
			return capability.resource_templates_count > 0;
		default:
			return false;
	}
}

export function InspectorPage() {
	const { t } = useTranslation("inspector");
	usePageTranslations("inspector");
	const sidebarOpen = useAppStore((state) => state.sidebarOpen);
	const [searchParams] = useSearchParams();
	const initialServerId = searchParams.get("server_id");
	const initialServerName = searchParams.get("server_name");
	const evaluationScenarioId = useId();
	const [featureTab, setFeatureTab] = useState<InspectorFeatureTab>("inspect");
	const [workspaceView, setWorkspaceView] = useState<InspectorWorkspaceView>("connect");
	const [footerWorkspace, setFooterWorkspace] =
		useState<InspectorFooterWorkspace | null>(null);
	const [capabilityFamilyStates, setCapabilityFamilyStates] = useState(
		createInitialCapabilityFamilyStates,
	);
	const capabilityListRequestRef = useRef(0);
	const loggedSessionOpenRef = useRef<string | null>(null);
	const [activeCapabilityFamily, setActiveCapabilityFamily] =
		useState<InspectorCapabilityFamily | null>("tools");
	const [inspectorConfig, setInspectorConfig] = useState<InspectorConfigurationState>(
		DEFAULT_INSPECTOR_CONFIGURATION,
	);
	const [connectionMode, setConnectionMode] = useState<InspectorConnectionMode>("native");
	const [managedTargets, setManagedTargets] = useState<InspectorManagedTarget[]>([]);
	const [scratchTargets, setScratchTargets] = useState<InspectorScratchTarget[]>([]);
	const [targetsLoaded, setTargetsLoaded] = useState(false);
	const [targetsError, setTargetsError] = useState<string | null>(null);
	const [selectedTargetKey, setSelectedTargetKey] = useState<string | null>(null);
	const [connectedTargetKey, setConnectedTargetKey] = useState<string | null>(null);
	const [pendingConnectTargetKey, setPendingConnectTargetKey] = useState<string | null>(null);
	const [restoringConnectTargetKey, setRestoringConnectTargetKey] = useState<string | null>(null);
	const [restoredConnectedTargetKey, setRestoredConnectedTargetKey] = useState(
		loadInspectorStandaloneConnectionTargetKey,
	);
	const [snapshotLoading, setSnapshotLoading] =
		useState<InspectorSnapshotKind | null>(null);
	const [snapshotState, setSnapshotState] =
		useState<InspectorSnapshotState | null>(null);
	const [evaluationScenario, setEvaluationScenario] = useState("");
	const [evaluationProviderIdValue, setEvaluationProviderIdValue] =
		useState("");
	const [compatibilitySpecVersion, setCompatibilitySpecVersion] =
		useState<InspectorCompatibilitySpecVersion>("2025-11-25");
	const [packageSafetyFactSource, setPackageSafetyFactSource] =
		useState<InspectorPackageSafetyFactSource>("runtime_cache");
	const [packageSafetyDatabase, setPackageSafetyDatabase] =
		useState<InspectorPackageSafetyDatabase>("combined");
	const [packageSafetyScanDepth, setPackageSafetyScanDepth] =
		useState<InspectorPackageSafetyScanDepth>("standard");
	const [llmEvaluationFocus, setLlmEvaluationFocus] = useState<
		InspectorLlmEvaluationFocus[]
	>(DEFAULT_INSPECTOR_LLM_EVALUATION_FOCUS);
	const [evaluationLoading, setEvaluationLoading] = useState(false);
	const [evaluationState, setEvaluationState] =
		useState<InspectorEvaluationState | null>(null);
	const [activityPanelExpanded, setActivityPanelExpanded] = useState(false);
	const [activityPanelHeight, setActivityPanelHeight] = useState(280);
	const [activityPanelPinned, setActivityPanelPinned] = useState(false);
	const [activitySearch, setActivitySearch] = useState("");
	const [activitySearchOpen, setActivitySearchOpen] = useState(false);
	const [capabilitySearch, setCapabilitySearch] = useState("");
	const [capabilitySearchOpen, setCapabilitySearchOpen] = useState(false);
	const [selectedActivityEntryId, setSelectedActivityEntryId] = useState<string | null>(
		null,
	);

	const targets = useMemo(
		() => [...managedTargets, ...scratchTargets],
		[managedTargets, scratchTargets],
	);
	const connectedTarget = useMemo(
		() => targets.find((target) => targetKey(target) === connectedTargetKey) ?? null,
		[connectedTargetKey, targets],
	);
	const connectedTargetSnapshot = useMemo(() => {
		if (!connectedTarget) return null;
		if (connectedTarget.source === "managed") {
			return {
				source: "managed" as const,
				serverId: connectedTarget.id,
				name: connectedTarget.name,
			};
		}
		return {
			source: "scratch" as const,
			scratchId: connectedTarget.id,
			name: connectedTarget.name,
			config: connectedTarget.config,
		};
	}, [connectedTarget]);
	const visibleCapabilityFamilies = useMemo<InspectorCapabilityFamilyOption[]>(() => {
		if (!connectedTarget) return [];

		const stableFamilies = INSPECTOR_CAPABILITY_FAMILIES.filter(
			(family) => !family.placeholder,
		);

		if (connectedTarget.source !== "managed" || !connectedTarget.capability) {
			return stableFamilies;
		}

		const { capability } = connectedTarget;
		return stableFamilies.filter((family) =>
			capabilityFamilyIsAdvertised(capability, family.value),
		);
	}, [connectedTarget]);
	const connectedTargetLogId = connectedTarget ? targetKey(connectedTarget) : "";
	const selectedTargetIsLoaded =
		!!selectedTargetKey &&
		targets.some((target) => targetKey(target) === selectedTargetKey);
	const activityTargetLogId =
		connectedTargetLogId || (selectedTargetIsLoaded ? selectedTargetKey : "");
	const { events: activityEvents, appendEvent, clearEvents } =
		useInspectorStandaloneLog(activityTargetLogId);
	const translateInspectorEvent = useCallback<InspectorLogTranslate>(
		(key, options) => {
			if (key.startsWith("inspector:")) {
				return t(key.replace(/^inspector:/, ""), options);
			}
			return t(key, options);
		},
		[t],
	);
	const filteredActivityEvents = useMemo(
		() =>
			activityEvents.filter((entry) =>
				inspectorEventMatchesSearch(entry, activitySearch, translateInspectorEvent),
			),
		[activityEvents, activitySearch, translateInspectorEvent],
	);
	const activityRows = useMemo(
		() =>
			filteredActivityEvents.map((entry, index) =>
				mapInspectorEventToActivityLogRow(entry, index, translateInspectorEvent),
			),
		[filteredActivityEvents, translateInspectorEvent],
	);
	const selectedActivityEntry = useMemo<InspectorLogEventEntry | null>(
		() =>
			activityEvents.find((entry) => entry.id === selectedActivityEntryId) ?? null,
		[activityEvents, selectedActivityEntryId],
	);
	const activeFamilyState = activeCapabilityFamily
		? capabilityFamilyStates[activeCapabilityFamily]
		: null;
	const selectedCapabilityItem = useMemo(() => {
		if (!activeCapabilityFamily || !activeFamilyState?.selectedKey) {
			return null;
		}
		return (
			activeFamilyState.items.find(
				(item) => item.key === activeFamilyState.selectedKey,
			) ?? null
		);
	}, [activeCapabilityFamily, activeFamilyState]);
	const targetRequest = useMemo(() => {
		if (!connectedTarget) return null;
		return connectedTarget.source === "managed"
			? { mode: "native" as const, server_id: connectedTarget.id }
			: { mode: "native" as const, scratch_id: connectedTarget.id };
	}, [connectedTarget]);
	const {
		ensureSession,
		invalidateSession,
		connected: sessionConnected,
		sessionId: currentSessionId,
	} =
		useInspectorNativeSession(targetRequest);

	const selectFeatureTab = useCallback(
		(tab: InspectorFeatureTab) => {
			setFooterWorkspace(null);
			setFeatureTab(tab);
			if (!connectedTarget || !sessionConnected) {
				setWorkspaceView("connect");
				return;
			}
			setWorkspaceView(tab);
		},
		[connectedTarget, sessionConnected],
	);

	const selectFooterWorkspace = useCallback((workspace: InspectorFooterWorkspace) => {
		setFooterWorkspace(workspace);
		setWorkspaceView(workspace);
	}, []);

	const refreshTargets = useCallback(
		async (preferredTargetKey?: string) => {
			setTargetsError(null);
			try {
				const [managedResponse, scratchResponse] = await Promise.all([
					serversApi.getAll(),
					inspectorApi.scratchServerList(),
				]);
				const nextManaged = managedTargetsFromServers(managedResponse.servers);
				if (!scratchResponse?.success || !scratchResponse.data) {
					throw new Error(
						scratchResponse?.error
							? String(scratchResponse.error)
							: "Failed to list Inspector scratch servers",
					);
				}
				const nextScratch = (scratchResponse.data.records ?? [])
					.map((record: InspectorScratchServerRecord) => ({
						source: "scratch" as const,
						id: record.id,
						name: record.name || record.id,
						config: record.config,
					}))
					.sort((left, right) => left.name.localeCompare(right.name));
				setManagedTargets(nextManaged);
				setScratchTargets(nextScratch);
				setTargetsLoaded(true);
				if (preferredTargetKey) {
					setSelectedTargetKey(preferredTargetKey);
				}
			} catch (error) {
				const message = stringifyError(error);
				setTargetsError(message);
				notifyError(
					t("standalone.targetsFailedTitle", {
						defaultValue: "Targets failed to load",
					}),
					message,
				);
			}
		},
		[t],
	);

	useEffect(() => {
		void refreshTargets();
	}, [refreshTargets]);

	useEffect(() => {
		if (!restoredConnectedTargetKey || !targetsLoaded) {
			return;
		}

		if (targets.some((target) => targetKey(target) === restoredConnectedTargetKey)) {
			setSelectedTargetKey(restoredConnectedTargetKey);
			setConnectedTargetKey(restoredConnectedTargetKey);
			setPendingConnectTargetKey(restoredConnectedTargetKey);
			setRestoringConnectTargetKey(restoredConnectedTargetKey);
		} else {
			setRestoringConnectTargetKey(null);
			clearInspectorStandaloneConnectionTargetKey();
		}
		setRestoredConnectedTargetKey(null);
	}, [restoredConnectedTargetKey, targets, targetsLoaded]);

	useEffect(() => {
		if (selectedTargetKey && targets.some((target) => targetKey(target) === selectedTargetKey)) {
			return;
		}

		if (!initialServerId && !initialServerName) {
			setSelectedTargetKey(null);
			return;
		}

		const requestedManaged = initialServerId
			? managedTargets.find((target) => target.id === initialServerId)
			: initialServerName
				? managedTargets.find((target) => target.name === initialServerName)
				: null;
		setSelectedTargetKey(requestedManaged ? targetKey(requestedManaged) : null);
	}, [
		initialServerId,
		initialServerName,
		managedTargets,
		selectedTargetKey,
		targets,
	]);

	useEffect(() => {
		setCapabilityFamilyStates(createInitialCapabilityFamilyStates());
		setActiveCapabilityFamily("tools");
		setSnapshotState(null);
		setEvaluationState(null);
	}, [connectedTargetKey]);

	useEffect(() => {
		if (!connectedTargetKey || targets.some((target) => targetKey(target) === connectedTargetKey)) {
			return;
		}
		setConnectedTargetKey(null);
		setPendingConnectTargetKey(null);
		setRestoringConnectTargetKey(null);
		if (targetsLoaded) {
			clearInspectorStandaloneConnectionTargetKey();
		}
	}, [connectedTargetKey, targets, targetsLoaded]);

	useEffect(() => {
		if (!sessionConnected || !connectedTargetKey) {
			return;
		}
		if (
			!connectedTargetKey.startsWith("managed:") &&
			!connectedTargetKey.startsWith("scratch:")
		) {
			return;
		}
		saveInspectorStandaloneConnectionTargetKey(
			connectedTargetKey as InspectorStandaloneConnectionTargetKey,
		);
	}, [connectedTargetKey, sessionConnected]);

	useEffect(() => {
		const firstFamily = visibleCapabilityFamilies[0]?.value ?? null;
		if (!firstFamily) {
			setActiveCapabilityFamily(null);
			return;
		}
		if (
			!activeCapabilityFamily ||
			!visibleCapabilityFamilies.some((family) => family.value === activeCapabilityFamily)
		) {
			setActiveCapabilityFamily(firstFamily);
		}
	}, [activeCapabilityFamily, visibleCapabilityFamilies]);

	const requireTargetRequest = useCallback(
		(action: string) => {
			if (targetRequest) return targetRequest;
			notifyError(
				t("standalone.targetRequiredTitle", {
					defaultValue: "Select a server",
				}),
				t("standalone.targetRequiredDescription", {
					defaultValue: "{{action}} requires a managed or scratch target.",
					action,
				}),
			);
			return null;
		},
		[targetRequest, t],
	);

	const logActivityStep = useCallback(
		(entry: Parameters<typeof appendEvent>[0]) => {
			appendEvent(entry);
			if (activityPanelPinned) {
				setActivityPanelExpanded(true);
			}
		},
		[activityPanelPinned, appendEvent],
	);

	const logCurrentSessionClose = useCallback(() => {
		if (!currentSessionId || !connectedTargetLogId) {
			return;
		}
		logActivityStep({
			data: {
				event: "session_close",
				session_id: currentSessionId,
				server_id: connectedTargetLogId,
			},
			request: { session_id: currentSessionId },
		});
		loggedSessionOpenRef.current = null;
	}, [connectedTargetLogId, currentSessionId, logActivityStep]);

	const handleCapabilityList = useCallback(
		async (family: InspectorCapabilityFamily) => {
			const requestTarget = requireTargetRequest(
				capabilityFamilyListMethod(family),
			);
			if (!requestTarget || !connectedTargetLogId) {
				return;
			}

			const requestId = ++capabilityListRequestRef.current;

			setActiveCapabilityFamily(family);
			setCapabilityFamilyStates((previous) => ({
				...previous,
				[family]: {
					...previous[family],
					listing: true,
				},
			}));

			const listKind = capabilityFamilyToListKind(family);
			const listMethod = capabilityFamilyListMethod(family);

			try {
				const sessionId = await ensureSession();
				if (!sessionId) {
					throw new Error("Failed to open inspector session");
				}

				if (requestId !== capabilityListRequestRef.current) {
					return;
				}

				if (listKind) {
					logActivityStep({
						data: {
							event: "mcp_exchange",
							direction: "outbound",
							method: listMethod,
							server_id: connectedTargetLogId,
							mode: "native",
							session_id: sessionId,
						},
						request: { jsonrpc: "2.0", method: listMethod },
					});
				}

				const items = await fetchInspectorCapabilityList({
					family,
					targetRequest: requestTarget,
					sessionId,
					refresh: true,
				});

				if (requestId !== capabilityListRequestRef.current) {
					return;
				}

				if (listKind) {
					logActivityStep({
						data: {
							event: "mcp_exchange",
							direction: "inbound",
							method: listMethod,
							server_id: connectedTargetLogId,
							mode: "native",
							session_id: sessionId,
						},
						response: { jsonrpc: "2.0", result: { [listKind]: items } },
					});
				}

				setCapabilityFamilyStates((previous) => ({
					...previous,
					[family]: {
						listed: true,
						listing: false,
						items,
						selectedKey: items[0]?.key ?? null,
					},
				}));
			} catch (error) {
				if (requestId !== capabilityListRequestRef.current) {
					return;
				}
				if (isInspectorSessionUnavailableError(error)) {
					invalidateSession();
				}
				setCapabilityFamilyStates((previous) => ({
					...previous,
					[family]: {
						...previous[family],
						listing: false,
					},
				}));
				notifyError(
					t("standalone.capabilityListFailedTitle", {
						defaultValue: "Capability list failed",
					}),
					stringifyError(error),
				);
			}
		},
		[
			ensureSession,
			invalidateSession,
			logActivityStep,
			requireTargetRequest,
			connectedTargetLogId,
			t,
		],
	);

	const handleCapabilityClear = useCallback(
		(family: InspectorCapabilityFamily) => {
			setCapabilityFamilyStates((previous) => ({
				...previous,
				[family]: createEmptyCapabilityFamilyState(),
			}));
			setCapabilitySearch("");
			setCapabilitySearchOpen(false);
			if (activeCapabilityFamily === family) {
				setActiveCapabilityFamily("tools");
			}
		},
		[activeCapabilityFamily],
	);

	const handleCapabilitySelectItem = useCallback(
		(family: InspectorCapabilityFamily, key: string) => {
			setActiveCapabilityFamily(family);
			setCapabilityFamilyStates((previous) => ({
				...previous,
				[family]: {
					...previous[family],
					selectedKey: key,
				},
			}));
		},
		[],
	);

	const handleActiveCapabilityFamilyChange = useCallback(
		(family: InspectorCapabilityFamily | null) => {
			capabilityListRequestRef.current += 1;
			setActiveCapabilityFamily(family);
		},
		[],
	);

	useEffect(() => {
		if (!pendingConnectTargetKey || pendingConnectTargetKey !== connectedTargetKey) {
			return;
		}
		void ensureSession()
			.then((sessionId) => {
				if (!sessionId) {
					setConnectedTargetKey(null);
					return;
				}
				if (connectedTargetLogId) {
					const sessionOpenLogKey = `${connectedTargetLogId}:${sessionId}`;
					if (loggedSessionOpenRef.current !== sessionOpenLogKey) {
						logActivityStep({
							data: {
								event: "session_open",
								server_id: connectedTargetLogId,
								mode: "native",
							},
							request: targetRequest,
							response: { session_id: sessionId },
						});
						loggedSessionOpenRef.current = sessionOpenLogKey;
					}
				}
			})
			.catch((error) => {
				setConnectedTargetKey(null);
				setRestoringConnectTargetKey(null);
				notifyError("Connection failed", stringifyError(error));
			})
			.finally(() => {
				setPendingConnectTargetKey(null);
				setRestoringConnectTargetKey(null);
			});
	}, [
		connectedTargetKey,
		connectedTargetLogId,
		ensureSession,
		logActivityStep,
		pendingConnectTargetKey,
		targetRequest,
	]);

	const handleConnect = useCallback(
		async (candidate: InspectorConnectCandidate) => {
			logCurrentSessionClose();
			clearInspectorStandaloneConnectionTargetKey();
			setRestoredConnectedTargetKey(null);
			setRestoringConnectTargetKey(null);
			if (candidate.source === "managed") {
				const key = `managed:${candidate.serverId}`;
				setSelectedTargetKey(key);
				setConnectedTargetKey(key);
				setPendingConnectTargetKey(key);
				return;
			}

			if (candidate.scratchId) {
				const key = `scratch:${candidate.scratchId}`;
				setSelectedTargetKey(key);
				setConnectedTargetKey(key);
				setPendingConnectTargetKey(key);
				return;
			}

			setPendingConnectTargetKey("scratch:create");
			try {
				const response = await inspectorApi.scratchServerCreate({
					name: candidate.draft.name,
					config: buildScratchServerConfig(candidate.draft),
					origin: "inspector-connect",
				});
				if (!response.success || !response.data?.record) {
					throw new Error(
						response.error
							? String(response.error)
							: "Failed to create Inspector scratch server",
					);
				}
				const record = response.data.record as InspectorScratchServerRecord;
				if (!record.id) {
					throw new Error("Inspector scratch server response is missing an id");
				}
				const key = `scratch:${record.id}`;
				await refreshTargets(key);
				setConnectedTargetKey(key);
				setPendingConnectTargetKey(key);
			} catch (error) {
				setConnectedTargetKey(null);
				setPendingConnectTargetKey(null);
				setRestoringConnectTargetKey(null);
				notifyError("Connection failed", stringifyError(error));
			}
		},
		[logCurrentSessionClose, refreshTargets],
	);

	const handleDisconnect = useCallback(() => {
		logCurrentSessionClose();
		invalidateSession();
		setConnectedTargetKey(null);
		setPendingConnectTargetKey(null);
		setRestoringConnectTargetKey(null);
		clearInspectorStandaloneConnectionTargetKey();
	}, [invalidateSession, logCurrentSessionClose]);

	const handleSnapshotLoad = useCallback(
		async (snapshotKind: InspectorSnapshotKind) => {
			const requestTarget = requireTargetRequest(snapshotTitle(snapshotKind));
			if (!requestTarget) return;

			setSnapshotLoading(snapshotKind);
			try {
				const request = { ...requestTarget, refresh: true };
				const response =
					snapshotKind === "compatibility"
						? await inspectorApi.compatibilitySnapshot(request)
						: await inspectorApi.packageSafetySnapshot(request);

				if (!response.success || !response.data?.snapshot) {
					throw new Error(
						typeof response.error === "string"
							? response.error
							: "Inspector snapshot request failed",
					);
				}

				setSnapshotState({
					kind: snapshotKind,
					payload: response.data.snapshot,
					loadedAt: new Date().toLocaleTimeString(),
				});
			} catch (error) {
				notifyError(
					t("standalone.snapshotFailedTitle", {
						defaultValue: "Snapshot failed",
					}),
					stringifyError(error),
				);
			} finally {
				setSnapshotLoading(null);
			}
		},
		[requireTargetRequest, t],
	);

	const handleEvaluationRun = useCallback(async () => {
		const requestTarget = requireTargetRequest("LLM evaluation");
		if (!requestTarget) return;

		const scenario = evaluationScenario.trim();
		if (!scenario) {
			notifyError(
				t("standalone.evaluationScenarioRequiredTitle", {
					defaultValue: "Scenario is required",
				}),
			);
			return;
		}

		setEvaluationLoading(true);
		try {
			const providerId = evaluationProviderIdValue.trim();
			const response = await inspectorApi.llmEvaluate({
				...requestTarget,
				scenario,
				provider_id: providerId || undefined,
			});

			if (!response.success || !response.data?.evaluation) {
				throw new Error(
					typeof response.error === "string"
						? response.error
						: "Inspector LLM evaluation failed",
				);
			}

			setEvaluationState({
				evaluation: response.data.evaluation,
				loadedAt: new Date().toLocaleTimeString(),
			});
		} catch (error) {
			notifyError(
				t("standalone.evaluationFailedTitle", {
					defaultValue: "Evaluation failed",
				}),
				stringifyError(error),
			);
		} finally {
			setEvaluationLoading(false);
		}
	}, [
		evaluationProviderIdValue,
		evaluationScenario,
		requireTargetRequest,
		t,
	]);

	const capabilityControlsDisabled = !connectedTarget || !sessionConnected;
	const sessionConnecting = pendingConnectTargetKey !== null;
	const sessionRestoring =
		!!restoringConnectTargetKey &&
		restoringConnectTargetKey === connectedTargetKey &&
		sessionConnecting &&
		!sessionConnected;

	const connectionComposer = (
		<InspectorConnectWorkspace
			selectedTargetKey={selectedTargetKey}
			connectedTargetKey={connectedTargetKey}
			connectedTargetSnapshot={connectedTargetSnapshot}
			connected={sessionConnected}
			connecting={sessionConnecting}
			onConnect={handleConnect}
			onDisconnect={handleDisconnect}
		/>
	);

	const inspectorChrome = useMemo(
		() => ({
			activityPanelExpanded,
			toggleActivityPanel: () => setActivityPanelExpanded((previous) => !previous),
		}),
		[activityPanelExpanded],
	);

	const headerActions = (
		<Segment
			value={connectionMode}
			onValueChange={(value) => setConnectionMode(value as InspectorConnectionMode)}
			options={INSPECTOR_TRANSPORT_MODE_OPTIONS}
			showDots={false}
			className="w-auto"
			listClassName="h-10 min-h-0 w-auto rounded-full"
			triggerClassName="h-8 gap-1.5 rounded-full px-3 py-0 text-xs"
		/>
	);

	const sidebarContent = sidebarOpen ? (
		<div className={inspectorSidebarExpandedControlsClassName()}>
			<div className="shrink-0">
				<InspectorServerPicker
					serverName={connectedTarget ? targetLabel(connectedTarget) : null}
					connected={sessionConnected}
					restoring={sessionRestoring}
					onOpenConnect={() => selectFooterWorkspace("connect")}
				/>
			</div>
			<div className={cn(sidebarIconRailClassName(), "shrink-0")}>
				<div className={sidebarIconRailGridClassName()}>
					{FEATURE_TABS.map((tab) => {
						const Icon = tab.icon;
						const selected = featureTab === tab.value && footerWorkspace === null;
						return (
							<button
								key={tab.value}
								type="button"
								aria-label={tab.label}
								className={sidebarIconLabeledActionClassName({
									selected,
								})}
								onClick={() => selectFeatureTab(tab.value)}
							>
								<Icon size={sidebarFeatureTabIconSize} aria-hidden />
								<span className="max-w-full truncate text-[10px] font-medium leading-none">
									{tab.shortLabel}
								</span>
							</button>
						);
					})}
				</div>
			</div>
			<InspectorFeatureSidebarPanel
				featureTab={featureTab}
				onFeatureTabActivate={selectFeatureTab}
				hasSelectedTarget={!!connectedTarget && sessionConnected}
				capabilityFamilyStates={capabilityFamilyStates}
				capabilityFamilies={sessionConnected ? visibleCapabilityFamilies : []}
				activeCapabilityFamily={activeCapabilityFamily}
				onActiveCapabilityFamilyChange={handleActiveCapabilityFamilyChange}
				onCapabilityList={(family) => void handleCapabilityList(family)}
				onCapabilityClear={handleCapabilityClear}
				onCapabilitySelectItem={handleCapabilitySelectItem}
				capabilitySearch={capabilitySearch}
				onCapabilitySearchChange={setCapabilitySearch}
				capabilitySearchOpen={capabilitySearchOpen}
				onCapabilitySearchOpenChange={setCapabilitySearchOpen}
				capabilityControlsDisabled={capabilityControlsDisabled}
				compatibilitySpecVersion={compatibilitySpecVersion}
				onCompatibilitySpecVersionChange={setCompatibilitySpecVersion}
				packageSafetyFactSource={packageSafetyFactSource}
				onPackageSafetyFactSourceChange={setPackageSafetyFactSource}
				packageSafetyDatabase={packageSafetyDatabase}
				onPackageSafetyDatabaseChange={setPackageSafetyDatabase}
				packageSafetyScanDepth={packageSafetyScanDepth}
				onPackageSafetyScanDepthChange={setPackageSafetyScanDepth}
				llmEvaluationFocus={llmEvaluationFocus}
				onLlmEvaluationFocusChange={setLlmEvaluationFocus}
				llmEvaluationProviderId={evaluationProviderIdValue}
				onLlmEvaluationProviderIdChange={setEvaluationProviderIdValue}
			/>
			{targetsError ? (
				<p className="shrink-0 text-xs text-red-600 dark:text-red-400">{targetsError}</p>
			) : null}
		</div>
	) : (
		<>
			{FEATURE_TABS.map((tab) => {
				const Icon = tab.icon;
				const selected = featureTab === tab.value && footerWorkspace === null;
				return (
					<Tooltip key={tab.value}>
						<TooltipTrigger asChild>
							<button
								type="button"
								aria-label={tab.label}
								className={sidebarNavItemClassName(false, {
									active: selected,
								})}
								onClick={() => selectFeatureTab(tab.value)}
							>
								<SidebarNavIcon sidebarOpen={false}>
									<Icon size={sidebarFeatureTabIconSize} />
								</SidebarNavIcon>
							</button>
						</TooltipTrigger>
						<TooltipContent
							side="right"
							align="center"
							className="max-w-xs px-3 py-2 text-xs leading-relaxed"
						>
							<p className="font-semibold">{tab.label}</p>
							<p className="mt-1 font-normal text-background/85">
								{tab.description}
							</p>
						</TooltipContent>
					</Tooltip>
				);
			})}
			{targetsError ? (
				<p className="text-xs text-red-600 dark:text-red-400">{targetsError}</p>
			) : null}
		</>
	);

	return (
		<InspectorChromeProvider value={inspectorChrome}>
			<InspectorWindowLayout
				sidebar={sidebarContent}
				footerWorkspace={footerWorkspace}
				onFooterWorkspaceChange={selectFooterWorkspace}
				workspaceModeLabel={inspectorWorkspaceModeLabel(workspaceView)}
				headerActions={headerActions}
			>
				<div className="relative flex h-full min-h-0 flex-col overflow-hidden bg-background">
					<main className="mb-8 flex min-h-0 min-w-0 flex-1 flex-col">
						{workspaceView === "configuration" ? (
							<div className={inspectorWorkspaceContentClassName("pt-3")}>
								<InspectorConfigurationWorkspace
									config={inspectorConfig}
									onConfigChange={(patch) =>
										setInspectorConfig((previous) => ({ ...previous, ...patch }))
									}
								/>
							</div>
						) : workspaceView === "connect" ? (
							<div className={inspectorConnectWorkspaceClassName()}>
								{connectionComposer}
							</div>
						) : !connectedTarget || !sessionConnected ? (
							<div className={inspectorConnectWorkspaceClassName()}>
								{connectionComposer}
							</div>
						) : (
							<>
								<div className="bg-background px-6 py-4">
									<div className="flex flex-wrap items-center gap-2">
										<p className="truncate text-xl font-semibold text-foreground">
											{targetLabel(connectedTarget)}
										</p>
										<Badge
											variant={
												connectedTarget.source === "managed" ? "secondary" : "outline"
											}
										>
											{connectedTarget.source === "managed" ? "Managed" : "Scratch"}
										</Badge>
									</div>
								</div>

								<div className={inspectorWorkspaceContentClassName()}>

									{workspaceView === "inspect" ? (
										<InspectorCapabilityWorkspace
											activeFamily={activeCapabilityFamily}
											selectedItem={selectedCapabilityItem}
											items={activeFamilyState?.items ?? []}
											onSelectItemKey={(key) => {
												if (!activeCapabilityFamily) return;
												handleCapabilitySelectItem(activeCapabilityFamily, key);
											}}
											disabled={capabilityControlsDisabled}
										/>
									) : null}

									{workspaceView === "compatibility" ? (
										<div className="max-w-5xl space-y-4">
											<div className="rounded-md border border-dashed border-border bg-card/40 p-4">
												<div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
													<div className="min-w-0">
														<div className="flex items-center gap-2">
															<ShieldCheck className="h-5 w-5 text-muted-foreground" />
															<p className="text-base font-medium text-foreground">
																Compatibility summary
															</p>
														</div>
														<p className="mt-1 text-sm text-muted-foreground">
															Baseline: {compatibilitySpecVersion}. Summarize view and
															spec-fit hints will render here.
														</p>
													</div>
													<Button
														type="button"
														variant="outline"
														className="h-9 gap-2"
														disabled={snapshotLoading !== null}
														onClick={() => void handleSnapshotLoad("compatibility")}
													>
														{snapshotLoading === "compatibility" ? (
															<RefreshCcw className="h-4 w-4 animate-spin" />
														) : (
															<ShieldCheck className="h-4 w-4" />
														)}
														Run comparison (draft)
													</Button>
												</div>
											</div>
											<div className="rounded-md border border-dashed border-border bg-card/20 p-4">
												<p className="text-sm font-medium text-foreground">
													Spec vs server diff
												</p>
												<p className="mt-1 text-sm text-muted-foreground">
													Git-diff style requirement columns and optional timeline will
													appear here after backend wiring.
												</p>
												{snapshotState?.kind === "compatibility" ? (
													<pre className="mt-4 max-h-96 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-background p-3 font-mono text-xs text-muted-foreground">
														{JSON.stringify(snapshotState.payload, null, 2)}
													</pre>
												) : null}
											</div>
										</div>
									) : null}

									{workspaceView === "package_safety" ? (
										<div className="max-w-5xl space-y-4">
											<div className="rounded-md border border-dashed border-border bg-card/40 p-4">
												<div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
													<div className="min-w-0">
														<div className="flex items-center gap-2">
															<PackageSearch className="h-5 w-5 text-muted-foreground" />
															<p className="text-base font-medium text-foreground">
																Package safety scan
															</p>
														</div>
														<p className="mt-1 text-sm text-muted-foreground">
															Source: {packageSafetyFactSource.replace("_", " ")} ·
															Database: {packageSafetyDatabase} · Depth:{" "}
															{packageSafetyScanDepth}
														</p>
													</div>
													<Button
														type="button"
														variant="outline"
														className="h-9 gap-2"
														disabled={snapshotLoading !== null}
														onClick={() => void handleSnapshotLoad("package_safety")}
													>
														{snapshotLoading === "package_safety" ? (
															<RefreshCcw className="h-4 w-4 animate-spin" />
														) : (
															<PackageSearch className="h-4 w-4" />
														)}
														Start scan (draft)
													</Button>
												</div>
											</div>
											<div className="rounded-md border border-dashed border-border bg-card/20 p-4">
												<p className="text-sm font-medium text-foreground">
													Scan progress and findings
												</p>
												<p className="mt-1 text-sm text-muted-foreground">
													Structured results or embedded report views will render here.
												</p>
												{snapshotState?.kind === "package_safety" ? (
													<pre className="mt-4 max-h-96 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-background p-3 font-mono text-xs text-muted-foreground">
														{JSON.stringify(snapshotState.payload, null, 2)}
													</pre>
												) : null}
											</div>
										</div>
									) : null}

									{workspaceView === "llm_evaluation" ? (
										<div className="max-w-5xl space-y-4">
											<div className="rounded-md border border-dashed border-border bg-card/40 p-4">
												<div className="flex flex-col gap-4">
													<div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
														<div className="min-w-0">
															<div className="flex items-center gap-2">
																<MessageSquareText className="h-5 w-5 text-muted-foreground" />
																<p className="text-base font-medium text-foreground">
																	LLM evaluation
																</p>
															</div>
															<p className="mt-1 text-sm text-muted-foreground">
																Focus: {llmEvaluationFocus.join(", ") || "none"} ·
																Provider: {evaluationProviderIdValue || "default"}
															</p>
														</div>
														<Button
															type="button"
															className="h-9 gap-2"
															disabled={evaluationLoading}
															onClick={() => void handleEvaluationRun()}
														>
															{evaluationLoading ? (
																<RefreshCcw className="h-4 w-4 animate-spin" />
															) : (
																<MessageSquareText className="h-4 w-4" />
															)}
															Run evaluation (draft)
														</Button>
													</div>
													<div className="space-y-2">
														<Label htmlFor={evaluationScenarioId}>Scenario</Label>
														<Textarea
															id={evaluationScenarioId}
															value={evaluationScenario}
															onChange={(event) =>
																setEvaluationScenario(event.target.value)
															}
															placeholder="Ask the model to review this server using the selected focus dimensions."
															className="min-h-28 text-sm"
														/>
													</div>
												</div>
											</div>
											<div className="rounded-md border border-dashed border-border bg-card/20 p-4">
												<p className="text-sm font-medium text-foreground">
													Analysis and recommendations
												</p>
												<p className="mt-1 text-sm text-muted-foreground">
													Factual scan outputs feed into the model; recommendations list
													here. Follow-up chat comes later.
												</p>
												{evaluationState ? (
													<pre className="mt-4 max-h-96 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-background p-3 font-mono text-xs text-muted-foreground">
														{JSON.stringify(evaluationState.evaluation, null, 2)}
													</pre>
												) : null}
											</div>
										</div>
									) : null}
								</div>
							</>
						)}
					</main>

					<InspectorBottomPanel
						expanded={activityPanelExpanded}
						onExpandedChange={setActivityPanelExpanded}
						height={activityPanelHeight}
						onHeightChange={setActivityPanelHeight}
						title={`Activity · ${activityEvents.length}`}
						search={{
							value: activitySearch,
							onChange: setActivitySearch,
							open: activitySearchOpen,
							onOpenChange: setActivitySearchOpen,
							placeholder: "Search activity",
							ariaLabel: "Search activity",
							clearAriaLabel: "Clear activity search",
						}}
						pinned={activityPanelPinned}
						onPinnedChange={setActivityPanelPinned}
						headerActions={
							<button
								type="button"
								className={INSPECTOR_BOTTOM_BAR_ICON_BUTTON_CLASSNAME}
								disabled={activityEvents.length === 0}
								aria-label="Clear activity"
								onClick={clearEvents}
							>
								<BrushCleaning className="h-3.5 w-3.5" aria-hidden />
							</button>
						}
					>
						<ActivityLogTable
							rows={activityRows}
							headers={{
								expandColumn: "Details",
								timestamp: "Time",
								action: "Action",
								category: "Category",
								status: "Status",
								target: "Target",
								duration: "Duration",
							}}
							emptyState={
								<div className="p-6 text-sm text-muted-foreground">
									No Inspector activity for the selected target.
								</div>
							}
							size="small"
							fillContainer
							interactiveColumns
							onRowClick={(row) => {
								if (row.eventId) {
									setSelectedActivityEntryId(row.eventId);
								}
							}}
						/>
					</InspectorBottomPanel>

					<InspectorEventDetailDrawer
						open={selectedActivityEntry !== null}
						onOpenChange={(open) => {
							if (!open) {
								setSelectedActivityEntryId(null);
							}
						}}
						entry={selectedActivityEntry}
					/>

				</div>
			</InspectorWindowLayout>
		</InspectorChromeProvider>
	);
}
