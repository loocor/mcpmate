import {
	FileText,
	LayoutTemplate,
	Loader2,
	MessageSquareText,
	Microscope,
	PackageSearch,
	PencilLine,
	Plus,
	RefreshCcw,
	RefreshCw,
	ShieldCheck,
	Trash2,
	Wrench,
} from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import InspectorDrawer from "../../components/inspector-drawer";
import { ServerInstallManualForm } from "../../components/server-install";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { ButtonGroup } from "../../components/ui/button-group";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../../components/ui/tabs";
import { Textarea } from "../../components/ui/textarea";
import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";
import { inspectorApi, serversApi } from "../../lib/api";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifySuccess, stringifyError } from "../../lib/notify";
import type { ServerSummary } from "../../lib/types";
import type { CapabilityRecord } from "../../types/capabilities";

type InspectorKind = "tool" | "resource" | "prompt" | "template";
type InspectorSnapshotKind = "compatibility" | "package_safety";
type InspectorFeatureTab =
	| "inspect"
	| "compatibility"
	| "package_safety"
	| "patcher"
	| "llm_evaluation";
type InspectorCapabilityPatchKind =
	| "tools"
	| "prompts"
	| "resources"
	| "resource_templates";

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
};

type InspectorScratchTarget = {
	source: "scratch";
	id: string;
	name: string;
	config: Record<string, unknown>;
};

type InspectorTarget = InspectorManagedTarget | InspectorScratchTarget;

type InspectorSnapshotState = {
	kind: InspectorSnapshotKind;
	payload: Record<string, unknown>;
	loadedAt: string;
};

type InspectorPatchState = {
	record: Record<string, unknown>;
	loadedAt: string;
};

type InspectorEvaluationState = {
	evaluation: Record<string, unknown>;
	loadedAt: string;
};

const INSPECTOR_KINDS: Array<{
	value: InspectorKind;
	icon: typeof Wrench;
	labelKey: string;
	defaultLabel: string;
}> = [
	{
		value: "tool",
		icon: Wrench,
		labelKey: "modes.toolCall",
		defaultLabel: "Tool",
	},
	{
		value: "prompt",
		icon: MessageSquareText,
		labelKey: "modes.getPrompt",
		defaultLabel: "Prompt",
	},
	{
		value: "resource",
		icon: FileText,
		labelKey: "modes.readResource",
		defaultLabel: "Resource",
	},
	{
		value: "template",
		icon: LayoutTemplate,
		labelKey: "modes.getTemplate",
		defaultLabel: "Template",
	},
];

const FEATURE_TABS: Array<{
	value: InspectorFeatureTab;
	icon: typeof Microscope;
	label: string;
}> = [
	{ value: "inspect", icon: Microscope, label: "Inspect" },
	{ value: "compatibility", icon: ShieldCheck, label: "Compatibility" },
	{ value: "package_safety", icon: PackageSearch, label: "Package Safety" },
	{ value: "patcher", icon: PencilLine, label: "Patcher" },
	{ value: "llm_evaluation", icon: MessageSquareText, label: "LLM Evaluation" },
];

const CAPABILITY_PATCH_KINDS: Array<{
	value: InspectorCapabilityPatchKind;
	label: string;
}> = [
	{ value: "tools", label: "Tools" },
	{ value: "prompts", label: "Prompts" },
	{ value: "resources", label: "Resources" },
	{ value: "resource_templates", label: "Templates" },
];

function targetKey(target: InspectorTarget): string {
	return `${target.source}:${target.id}`;
}

function targetLabel(target: InspectorTarget | null): string {
	if (!target) return "No target selected";
	return target.source === "managed"
		? target.name || target.id
		: `Scratch: ${target.name || target.id}`;
}

function parseInspectorKind(value: string | null): InspectorKind {
	return value === "tool" ||
		value === "prompt" ||
		value === "resource" ||
		value === "template"
		? value
		: "tool";
}

function segmentedButtonClass(index: number, total: number): string {
	return [
		"h-9 px-3",
		index === 0 ? "rounded-r-none" : "",
		index === total - 1 ? "rounded-l-none" : "",
		index > 0 && index < total - 1 ? "rounded-none" : "",
	].join(" ");
}

function snapshotTitle(kind: InspectorSnapshotKind): string {
	return kind === "compatibility"
		? "Compatibility snapshot"
		: "Package safety snapshot";
}

function capabilityRecordFromKey(
	kind: InspectorKind,
	key: string,
): CapabilityRecord | null {
	const trimmed = key.trim();
	if (!trimmed) return null;
	if (kind === "tool") {
		return {
			name: trimmed,
			tool_name: trimmed,
			unique_name: trimmed,
		} as CapabilityRecord;
	}
	if (kind === "prompt") {
		return {
			name: trimmed,
			prompt_name: trimmed,
			unique_name: trimmed,
		} as CapabilityRecord;
	}
	if (kind === "resource") {
		return {
			name: trimmed,
			uri: trimmed,
			resource_uri: trimmed,
		} as CapabilityRecord;
	}
	return {
		name: trimmed,
		uriTemplate: trimmed,
		uri_template: trimmed,
	} as CapabilityRecord;
}

function hasEntries(value: Record<string, string> | undefined): boolean {
	return Boolean(value && Object.keys(value).length > 0);
}

function buildScratchConfigFromDraft(
	draft: ServerInstallDraft,
): Record<string, unknown> {
	const config: Record<string, unknown> = {
		type: draft.kind,
	};

	if (draft.kind === "stdio") {
		config.command = draft.command;
		if (draft.args?.length) {
			config.args = draft.args;
		}
		if (hasEntries(draft.env)) {
			config.env = draft.env;
		}
		return config;
	}

	config.url = draft.url;
	if (hasEntries(draft.headers)) {
		config.headers = draft.headers;
	}
	return config;
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
		}))
		.sort((left, right) => left.name.localeCompare(right.name));
}

export function InspectorPage() {
	const { t } = useTranslation("inspector");
	usePageTranslations("inspector");
	const [searchParams] = useSearchParams();
	const initialServerId = searchParams.get("server_id");
	const initialServerName = searchParams.get("server_name");
	const initialCapabilityKey = searchParams.get("capability_key") ?? "";
	const patchKeyId = useId();
	const patchJsonId = useId();
	const evaluationScenarioId = useId();
	const evaluationProviderId = useId();
	const capabilityKeyId = useId();
	const [featureTab, setFeatureTab] = useState<InspectorFeatureTab>("inspect");
	const [kind, setKind] = useState<InspectorKind>(() =>
		parseInspectorKind(searchParams.get("kind")),
	);
	const [drawerOpen, setDrawerOpen] = useState(Boolean(initialServerId || initialServerName));
	const [capabilityKey, setCapabilityKey] = useState(initialCapabilityKey);
	const [managedTargets, setManagedTargets] = useState<InspectorManagedTarget[]>([]);
	const [scratchTargets, setScratchTargets] = useState<InspectorScratchTarget[]>([]);
	const [targetsLoading, setTargetsLoading] = useState(false);
	const [targetsError, setTargetsError] = useState<string | null>(null);
	const [selectedTargetKey, setSelectedTargetKey] = useState<string | null>(null);
	const [scratchImportOpen, setScratchImportOpen] = useState(false);
	const [scratchImportSaving, setScratchImportSaving] = useState(false);
	const [scratchDeletingId, setScratchDeletingId] = useState<string | null>(null);
	const [snapshotLoading, setSnapshotLoading] =
		useState<InspectorSnapshotKind | null>(null);
	const [snapshotState, setSnapshotState] =
		useState<InspectorSnapshotState | null>(null);
	const [patchKind, setPatchKind] =
		useState<InspectorCapabilityPatchKind>("tools");
	const [patchCapabilityKey, setPatchCapabilityKey] = useState(initialCapabilityKey);
	const [patchJson, setPatchJson] = useState("");
	const [patchLoading, setPatchLoading] = useState(false);
	const [patchState, setPatchState] = useState<InspectorPatchState | null>(
		null,
	);
	const [evaluationScenario, setEvaluationScenario] = useState("");
	const [evaluationProviderIdValue, setEvaluationProviderIdValue] =
		useState("");
	const [evaluationLoading, setEvaluationLoading] = useState(false);
	const [evaluationState, setEvaluationState] =
		useState<InspectorEvaluationState | null>(null);

	const targets = useMemo(
		() => [...managedTargets, ...scratchTargets],
		[managedTargets, scratchTargets],
	);
	const selectedTarget = useMemo(
		() => targets.find((target) => targetKey(target) === selectedTargetKey) ?? null,
		[selectedTargetKey, targets],
	);
	const activeKind = useMemo(
		() =>
			INSPECTOR_KINDS.find((entry) => entry.value === kind) ??
			INSPECTOR_KINDS[0],
		[kind],
	);
	const ActiveIcon = activeKind.icon;
	const selectedCapabilityItem = useMemo(
		() => capabilityRecordFromKey(kind, capabilityKey),
		[capabilityKey, kind],
	);
	const targetRequest = useMemo(() => {
		if (!selectedTarget) return null;
		return selectedTarget.source === "managed"
			? { mode: "native" as const, server_id: selectedTarget.id }
			: { mode: "native" as const, scratch_id: selectedTarget.id };
	}, [selectedTarget]);

	const refreshTargets = useCallback(
		async (preferredTargetKey?: string) => {
			setTargetsLoading(true);
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
			} finally {
				setTargetsLoading(false);
			}
		},
		[t],
	);

	useEffect(() => {
		void refreshTargets();
	}, [refreshTargets]);

	useEffect(() => {
		if (selectedTargetKey && targets.some((target) => targetKey(target) === selectedTargetKey)) {
			return;
		}

		const requestedManaged = initialServerId
			? managedTargets.find((target) => target.id === initialServerId)
			: initialServerName
				? managedTargets.find((target) => target.name === initialServerName)
				: null;
		const nextTarget = requestedManaged ?? targets[0] ?? null;
		setSelectedTargetKey(nextTarget ? targetKey(nextTarget) : null);
	}, [
		initialServerId,
		initialServerName,
		managedTargets,
		selectedTargetKey,
		targets,
	]);

	useEffect(() => {
		setSnapshotState(null);
		setPatchState(null);
		setEvaluationState(null);
	}, [selectedTargetKey]);

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

	const createScratchFromDraft = useCallback(
		async (draft: ServerInstallDraft): Promise<InspectorScratchServerRecord> => {
			const response = await inspectorApi.scratchServerCreate({
				name: draft.name,
				config: buildScratchConfigFromDraft(draft),
				origin: "standalone_inspector",
			});
			if (!response?.success || !response.data?.record) {
				throw new Error(
					response?.error
						? String(response.error)
						: "Failed to create Inspector scratch server",
				);
			}
			return response.data.record as InspectorScratchServerRecord;
		},
		[],
	);

	const handleScratchImport = useCallback(
		async (draft: ServerInstallDraft) => {
			setScratchImportSaving(true);
			try {
				const record = await createScratchFromDraft(draft);
				await refreshTargets(`scratch:${record.id}`);
				setScratchImportOpen(false);
				notifySuccess(
					t("scratch.notifications.created", {
						defaultValue: "Scratch server saved",
					}),
					record.name,
				);
			} catch (error) {
				notifyError(
					t("scratch.notifications.failed", {
						defaultValue: "Scratch server import failed",
					}),
					stringifyError(error),
				);
			} finally {
				setScratchImportSaving(false);
			}
		},
		[createScratchFromDraft, refreshTargets, t],
	);

	const handleScratchImportMultiple = useCallback(
		async (drafts: ServerInstallDraft[]) => {
			setScratchImportSaving(true);
			try {
				let lastRecord: InspectorScratchServerRecord | null = null;
				for (const draft of drafts) {
					lastRecord = await createScratchFromDraft(draft);
				}
				await refreshTargets(lastRecord ? `scratch:${lastRecord.id}` : undefined);
				setScratchImportOpen(false);
				notifySuccess(
					t("scratch.notifications.created", {
						defaultValue: "Scratch server saved",
					}),
					lastRecord?.name,
				);
			} catch (error) {
				notifyError(
					t("scratch.notifications.failed", {
						defaultValue: "Scratch server import failed",
					}),
					stringifyError(error),
				);
			} finally {
				setScratchImportSaving(false);
			}
		},
		[createScratchFromDraft, refreshTargets, t],
	);

	const handleScratchDelete = useCallback(
		async (recordId: string) => {
			setScratchDeletingId(recordId);
			try {
				const response = await inspectorApi.scratchServerDelete({
					record_id: recordId,
				});
				if (!response?.success || !response.data?.deleted) {
					throw new Error(
						response?.error
							? String(response.error)
							: "Failed to delete Inspector scratch server",
					);
				}
				await refreshTargets();
				notifySuccess(
					t("scratch.notifications.deleted", {
						defaultValue: "Scratch server deleted",
					}),
					recordId,
				);
			} catch (error) {
				notifyError(
					t("scratch.notifications.deleteFailed", {
						defaultValue: "Scratch server delete failed",
					}),
					stringifyError(error),
				);
			} finally {
				setScratchDeletingId(null);
			}
		},
		[refreshTargets, t],
	);

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

	const handlePatchApply = useCallback(async () => {
		const requestTarget = requireTargetRequest("Capability patcher");
		if (!requestTarget) return;

		const nextCapabilityKey = patchCapabilityKey.trim();
		if (!nextCapabilityKey) {
			notifyError(
				t("standalone.patchKeyRequiredTitle", {
					defaultValue: "Capability key is required",
				}),
			);
			return;
		}

		let patch: Record<string, unknown>;
		try {
			const parsed = JSON.parse(patchJson);
			if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
				throw new Error("Patch must be a JSON object");
			}
			patch = parsed as Record<string, unknown>;
		} catch (error) {
			notifyError(
				t("standalone.patchJsonInvalidTitle", {
					defaultValue: "Invalid patch JSON",
				}),
				stringifyError(error),
			);
			return;
		}

		setPatchLoading(true);
		try {
			const response = await inspectorApi.capabilityPatchUpsert({
				...requestTarget,
				capability_kind: patchKind,
				capability_key: nextCapabilityKey,
				patch,
			});

			if (!response.success || !response.data?.record) {
				throw new Error(
					typeof response.error === "string"
						? response.error
						: "Inspector capability patch request failed",
				);
			}

			setPatchState({
				record: response.data.record,
				loadedAt: new Date().toLocaleTimeString(),
			});
			notifySuccess(
				t("standalone.patchSavedTitle", {
					defaultValue: "Capability patch saved",
				}),
			);
		} catch (error) {
			notifyError(
				t("standalone.patchFailedTitle", {
					defaultValue: "Patch failed",
				}),
				stringifyError(error),
			);
		} finally {
			setPatchLoading(false);
		}
	}, [
		patchCapabilityKey,
		patchJson,
		patchKind,
		requireTargetRequest,
		t,
	]);

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

	const renderTargetButton = (target: InspectorTarget) => {
		const selected = selectedTargetKey === targetKey(target);
		return (
			<div
				key={targetKey(target)}
				className={`w-full rounded-md border px-3 py-2 text-left text-sm transition ${
					selected
						? "border-primary bg-primary/10 text-foreground"
						: "border-transparent hover:border-border hover:bg-muted/70"
				}`}
			>
				<button
					type="button"
					className="w-full text-left"
					onClick={() => setSelectedTargetKey(targetKey(target))}
				>
					<div className="flex items-start justify-between gap-2">
						<div className="min-w-0">
							<p className="truncate font-medium">{target.name || target.id}</p>
							<p className="mt-1 truncate text-xs text-muted-foreground">
								{target.source === "managed"
									? target.serverType || "managed server"
									: target.id}
							</p>
						</div>
						<div className="flex shrink-0 items-center gap-1">
							<Badge
								variant={target.source === "managed" ? "secondary" : "outline"}
							>
								{target.source === "managed" ? "Managed" : "Scratch"}
							</Badge>
							{target.source === "managed" ? (
								<Badge variant={target.enabled ? "default" : "outline"}>
									{target.enabled ? "Enabled" : "Disabled"}
								</Badge>
							) : null}
						</div>
					</div>
				</button>
				{target.source === "scratch" ? (
					<div className="mt-2 flex justify-end">
						<Button
							type="button"
							variant="ghost"
							size="sm"
							className="h-7 gap-1 px-2 text-red-600 hover:text-red-700"
							disabled={scratchDeletingId === target.id}
							onClick={() => void handleScratchDelete(target.id)}
						>
							{scratchDeletingId === target.id ? (
								<Loader2 className="h-3.5 w-3.5 animate-spin" />
							) : (
								<Trash2 className="h-3.5 w-3.5" />
							)}
							Delete
						</Button>
					</div>
				) : null}
			</div>
		);
	};

	return (
		<div className="flex min-h-full bg-background">
			<aside className="flex w-[320px] shrink-0 flex-col border-r border-border bg-card/50">
				<div className="border-b border-border p-4">
					<div className="flex items-center justify-between gap-2">
						<div className="min-w-0">
							<div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
								<Microscope className="h-4 w-4" />
								<span>{t("standalone.eyebrow", { defaultValue: "Inspector" })}</span>
							</div>
							<h1 className="mt-1 truncate text-lg font-semibold text-foreground">
								{t("standalone.title", { defaultValue: "Inspector Workbench" })}
							</h1>
						</div>
						<div className="flex shrink-0 gap-1">
							<Button
								type="button"
								variant="outline"
								size="icon"
								className="h-8 w-8"
								disabled={targetsLoading}
								onClick={() => void refreshTargets()}
								aria-label="Refresh targets"
							>
								{targetsLoading ? (
									<Loader2 className="h-4 w-4 animate-spin" />
								) : (
									<RefreshCw className="h-4 w-4" />
								)}
							</Button>
							<Button
								type="button"
								size="icon"
								className="h-8 w-8"
								onClick={() => setScratchImportOpen(true)}
								aria-label="Add scratch server"
							>
								<Plus className="h-4 w-4" />
							</Button>
						</div>
					</div>
					{targetsError ? (
						<p className="mt-3 text-xs text-red-600 dark:text-red-400">
							{targetsError}
						</p>
					) : null}
				</div>
				<div className="min-h-0 flex-1 overflow-y-auto p-3">
					<div className="space-y-4">
						<section className="space-y-2">
							<div className="flex items-center justify-between px-1">
								<p className="text-xs font-semibold uppercase tracking-normal text-muted-foreground">
									Managed Registry
								</p>
								<Badge variant="secondary">{managedTargets.length}</Badge>
							</div>
							<div className="space-y-1">
								{managedTargets.length
									? managedTargets.map(renderTargetButton)
									: (
										<p className="rounded-md border border-dashed border-border p-3 text-sm text-muted-foreground">
											No managed servers.
										</p>
									)}
							</div>
						</section>
						<section className="space-y-2">
							<div className="flex items-center justify-between px-1">
								<p className="text-xs font-semibold uppercase tracking-normal text-muted-foreground">
									Scratch Workspace
								</p>
								<Badge variant="outline">{scratchTargets.length}</Badge>
							</div>
							<div className="space-y-1">
								{scratchTargets.length
									? scratchTargets.map(renderTargetButton)
									: (
										<p className="rounded-md border border-dashed border-border p-3 text-sm text-muted-foreground">
											Add a scratch server with the plus button.
										</p>
									)}
							</div>
						</section>
					</div>
				</div>
			</aside>

			<main className="flex min-w-0 flex-1 flex-col">
				<div className="border-b border-border bg-background px-6 py-4">
					<div className="flex flex-col gap-4 xl:flex-row xl:items-center xl:justify-between">
						<div className="min-w-0">
							<p className="text-sm text-muted-foreground">Target</p>
							<div className="mt-1 flex flex-wrap items-center gap-2">
								<p className="truncate text-xl font-semibold text-foreground">
									{targetLabel(selectedTarget)}
								</p>
								{selectedTarget ? (
									<Badge
										variant={
											selectedTarget.source === "managed" ? "secondary" : "outline"
										}
									>
										{selectedTarget.source === "managed"
											? "Managed"
											: "Scratch"}
									</Badge>
								) : null}
							</div>
						</div>
						<Tabs
							value={featureTab}
							onValueChange={(value) => setFeatureTab(value as InspectorFeatureTab)}
						>
							<TabsList className="flex h-auto flex-wrap justify-start">
								{FEATURE_TABS.map((tab) => {
									const Icon = tab.icon;
									return (
										<TabsTrigger
											key={tab.value}
											value={tab.value}
											className="gap-2"
										>
											<Icon className="h-4 w-4" />
											{tab.label}
										</TabsTrigger>
									);
								})}
							</TabsList>
						</Tabs>
					</div>
				</div>

				<Tabs
					value={featureTab}
					onValueChange={(value) => setFeatureTab(value as InspectorFeatureTab)}
					className="min-h-0 flex-1"
				>
					<div className="min-h-0 flex-1 overflow-y-auto px-6 py-6">
						<TabsContent value="inspect" className="m-0 max-w-4xl space-y-6">
							<div className="rounded-md border border-border bg-card/60 p-4">
								<div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
									<div className="min-w-0 space-y-3">
										<div className="flex items-center gap-2">
											<ActiveIcon className="h-5 w-5 text-muted-foreground" />
											<div>
												<p className="text-base font-medium text-foreground">
													{t(activeKind.labelKey, {
														defaultValue: activeKind.defaultLabel,
													})}
												</p>
												<p className="mt-1 text-sm text-muted-foreground">
													{targetLabel(selectedTarget)}
												</p>
											</div>
										</div>
										<ButtonGroup className="flex flex-wrap">
											{INSPECTOR_KINDS.map((entry, index) => {
												const Icon = entry.icon;
												return (
													<Button
														key={entry.value}
														type="button"
														variant={kind === entry.value ? "default" : "outline"}
														className={`gap-2 ${segmentedButtonClass(
															index,
															INSPECTOR_KINDS.length,
														)}`}
														onClick={() => setKind(entry.value)}
													>
														<Icon className="h-4 w-4" />
														{t(entry.labelKey, {
															defaultValue: entry.defaultLabel,
														})}
													</Button>
												);
											})}
										</ButtonGroup>
									</div>
									<Button
										type="button"
										className="h-9 gap-2"
										disabled={!selectedTarget}
										onClick={() => setDrawerOpen(true)}
									>
										<ActiveIcon className="h-4 w-4" />
										{t("standalone.open", { defaultValue: "Open Inspector" })}
									</Button>
								</div>
								<div className="mt-4 space-y-2">
									<Label htmlFor={capabilityKeyId}>Capability key</Label>
									<Input
										id={capabilityKeyId}
										value={capabilityKey}
										onChange={(event) => setCapabilityKey(event.target.value)}
										placeholder="Optional tool, prompt, resource, or template key"
										className="font-mono text-sm"
									/>
								</div>
							</div>
						</TabsContent>

						<TabsContent value="compatibility" className="m-0 max-w-4xl">
							<div className="rounded-md border border-border bg-card/60 p-4">
								<div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
									<div className="min-w-0">
										<div className="flex items-center gap-2">
											<ShieldCheck className="h-5 w-5 text-muted-foreground" />
											<p className="text-base font-medium text-foreground">
												Compatibility
											</p>
										</div>
										<p className="mt-1 text-sm text-muted-foreground">
											{snapshotState?.kind === "compatibility"
												? `${snapshotTitle(snapshotState.kind)} - ${snapshotState.loadedAt}`
												: "No compatibility snapshot loaded."}
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
										Load snapshot
									</Button>
								</div>
								{snapshotState?.kind === "compatibility" ? (
									<pre className="mt-4 max-h-96 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-background p-3 font-mono text-xs text-muted-foreground">
										{JSON.stringify(snapshotState.payload, null, 2)}
									</pre>
								) : null}
							</div>
						</TabsContent>

						<TabsContent value="package_safety" className="m-0 max-w-4xl">
							<div className="rounded-md border border-border bg-card/60 p-4">
								<div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
									<div className="min-w-0">
										<div className="flex items-center gap-2">
											<PackageSearch className="h-5 w-5 text-muted-foreground" />
											<p className="text-base font-medium text-foreground">
												Package Safety
											</p>
										</div>
										<p className="mt-1 text-sm text-muted-foreground">
											{snapshotState?.kind === "package_safety"
												? `${snapshotTitle(snapshotState.kind)} - ${snapshotState.loadedAt}`
												: "No package safety snapshot loaded."}
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
										Load snapshot
									</Button>
								</div>
								{snapshotState?.kind === "package_safety" ? (
									<pre className="mt-4 max-h-96 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-background p-3 font-mono text-xs text-muted-foreground">
										{JSON.stringify(snapshotState.payload, null, 2)}
									</pre>
								) : null}
							</div>
						</TabsContent>

						<TabsContent value="llm_evaluation" className="m-0 max-w-4xl">
							<div className="rounded-md border border-border bg-card/60 p-4">
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
												{evaluationState
													? `Evaluation loaded - ${evaluationState.loadedAt}`
													: "No evaluation loaded."}
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
											Run evaluation
										</Button>
									</div>
									<div className="grid gap-4 lg:grid-cols-[280px_minmax(0,1fr)]">
										<div className="space-y-2">
											<Label htmlFor={evaluationProviderId}>Provider ID</Label>
											<Input
												id={evaluationProviderId}
												value={evaluationProviderIdValue}
												onChange={(event) =>
													setEvaluationProviderIdValue(event.target.value)
												}
												placeholder="default provider"
												className="font-mono text-sm"
											/>
										</div>
										<div className="space-y-2">
											<Label htmlFor={evaluationScenarioId}>Scenario</Label>
											<Textarea
												id={evaluationScenarioId}
												value={evaluationScenario}
												onChange={(event) =>
													setEvaluationScenario(event.target.value)
												}
												placeholder="Convert 9 AM in Singapore to New York time."
												className="min-h-28 text-sm"
											/>
										</div>
									</div>
									{evaluationState ? (
										<pre className="max-h-96 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-background p-3 font-mono text-xs text-muted-foreground">
											{JSON.stringify(evaluationState.evaluation, null, 2)}
										</pre>
									) : null}
								</div>
							</div>
						</TabsContent>

						<TabsContent value="patcher" className="m-0 max-w-4xl">
							<div className="rounded-md border border-border bg-card/60 p-4">
								<div className="flex flex-col gap-4">
									<div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
										<div className="min-w-0">
											<div className="flex items-center gap-2">
												<PencilLine className="h-5 w-5 text-muted-foreground" />
												<p className="text-base font-medium text-foreground">
													Capability patcher
												</p>
											</div>
											<p className="mt-1 text-sm text-muted-foreground">
												{patchState
													? `Patch saved - ${patchState.loadedAt}`
													: "No patch saved."}
											</p>
										</div>
										<Button
											type="button"
											className="h-9 gap-2"
											disabled={patchLoading}
											onClick={() => void handlePatchApply()}
										>
											{patchLoading ? (
												<RefreshCcw className="h-4 w-4 animate-spin" />
											) : (
												<PencilLine className="h-4 w-4" />
											)}
											Save patch
										</Button>
									</div>
									<div className="space-y-2">
										<Label>Capability kind</Label>
										<ButtonGroup className="flex flex-wrap">
											{CAPABILITY_PATCH_KINDS.map((entry, index) => (
												<Button
													key={entry.value}
													type="button"
													variant={patchKind === entry.value ? "default" : "outline"}
													className={segmentedButtonClass(
														index,
														CAPABILITY_PATCH_KINDS.length,
													)}
													onClick={() => setPatchKind(entry.value)}
												>
													{entry.label}
												</Button>
											))}
										</ButtonGroup>
									</div>
									<div className="grid gap-4 lg:grid-cols-[280px_minmax(0,1fr)]">
										<div className="space-y-2">
											<Label htmlFor={patchKeyId}>Capability key</Label>
											<Input
												id={patchKeyId}
												value={patchCapabilityKey}
												onChange={(event) =>
													setPatchCapabilityKey(event.target.value)
												}
												placeholder="time_convert_time"
												className="font-mono text-sm"
											/>
										</div>
										<div className="space-y-2">
											<Label htmlFor={patchJsonId}>Patch JSON</Label>
											<Textarea
												id={patchJsonId}
												value={patchJson}
												onChange={(event) => setPatchJson(event.target.value)}
												placeholder={'{\n  "description": "..."\n}'}
												className="min-h-32 font-mono text-xs"
												spellCheck={false}
											/>
										</div>
									</div>
									{patchState ? (
										<pre className="max-h-80 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-background p-3 font-mono text-xs text-muted-foreground">
											{JSON.stringify(patchState.record, null, 2)}
										</pre>
									) : null}
								</div>
							</div>
						</TabsContent>
					</div>
				</Tabs>
			</main>

			<InspectorDrawer
				open={drawerOpen}
				onOpenChange={setDrawerOpen}
				serverId={selectedTarget?.source === "managed" ? selectedTarget.id : undefined}
				serverName={selectedTarget?.name}
				scratchId={selectedTarget?.source === "scratch" ? selectedTarget.id : undefined}
				showStandaloneButton={false}
				kind={kind}
				item={selectedCapabilityItem}
			/>

			<ServerInstallManualForm
				isOpen={scratchImportOpen}
				onClose={() => {
					if (!scratchImportSaving) {
						setScratchImportOpen(false);
					}
				}}
				onSubmit={handleScratchImport}
				onSubmitMultiple={handleScratchImportMultiple}
				drawerDirection="left"
			/>
		</div>
	);
}
