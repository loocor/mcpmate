import {
	AlertTriangle,
	ChevronDown,
	ChevronRight,
	Loader2,
	RefreshCw,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type {
	InstallSource,
	ServerInstallDraft,
	useServerInstallPipeline,
} from "../hooks/use-server-install-pipeline";
import { Alert, AlertDescription, AlertTitle } from "./ui/alert";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import {
	Drawer,
	DrawerContent,
	DrawerDescription,
	DrawerFooter,
	DrawerHeader,
	DrawerTitle,
} from "./ui/drawer";
import { ScrollArea } from "./ui/scroll-area";

interface ServerInstallDrawerProps {
	pipeline: ReturnType<typeof useServerInstallPipeline>;
	onBack?: (drafts: ServerInstallDraft[], source: InstallSource | null) => void;
}

type PreviewItem = {
	name?: string;
	tools?: { items?: unknown[] } | null;
	resources?: { items?: unknown[] } | null;
	resource_templates?: { items?: unknown[] } | null;
	prompts?: { items?: unknown[] } | null;
	error?: unknown;
	ok?: boolean;
	[key: string]: unknown;
};

const sourceLabel: Record<InstallSource, string> = {
	manual: "Manual form",
	ingest: "Uni-Import",
	market: "Marketplace",
};

export function ServerInstallDrawer({
	pipeline,
	onBack,
}: ServerInstallDrawerProps) {
	const {
		state: {
			drafts,
			source,
			previewState,
			previewError,
			isPreviewLoading,
			isImporting,
			open,
		},
		confirmImport,
		close,
	} = pipeline;

	const previewItemsByName = useMemo(() => {
		const map = new Map<string, PreviewItem>();
		const items = previewState?.data?.items;
		if (Array.isArray(items)) {
			for (const entry of items) {
				if (entry && typeof entry === "object" && "name" in entry) {
					const name = (entry as { name?: unknown }).name;
					if (typeof name === "string") {
						map.set(name, entry as PreviewItem);
					}
				}
			}
		}
		return map;
	}, [previewState]);

	// Track preview timestamp for "Last preview" text
	const [previewAt, setPreviewAt] = useState<number | null>(null);
	useEffect(() => {
		if (previewState && !isPreviewLoading) setPreviewAt(Date.now());
	}, [previewState, isPreviewLoading]);

	// Expand/collapse details per draft
	const [expanded, setExpanded] = useState<Record<string, boolean>>({});
	const toggleExpanded = (name: string) =>
		setExpanded((e) => ({ ...e, [name]: !e[name] }));

	type CapabilityKindKey = "tools" | "resources" | "prompts" | "templates";

	const capabilityBadge: Record<CapabilityKindKey, string> = {
		tools: "T",
		resources: "R",
		prompts: "P",
		templates: "Tm",
	};

	const capabilityLabels: Record<CapabilityKindKey, string> = {
		tools: "Tools",
		resources: "Resources",
		prompts: "Prompts",
		templates: "Templates",
	};

	const getCapabilityTitle = (
		kind: CapabilityKindKey,
		item: Record<string, unknown>,
	): string => {
		if (kind === "tools") {
			return (
				(item.tool_name as string | undefined) ||
				(item.name as string | undefined) ||
				(item.unique_name as string | undefined) ||
				(item.id as string | undefined) ||
				"Tool"
			);
		}
		if (kind === "resources") {
			return (
				(item.resource_uri as string | undefined) ||
				(item.uri as string | undefined) ||
				(item.name as string | undefined) ||
				"Resource"
			);
		}
		if (kind === "prompts") {
			return (
				(item.prompt_name as string | undefined) ||
				(item.name as string | undefined) ||
				"Prompt"
			);
		}
		return (
			(item.uri_template as string | undefined) ||
			(item.name as string | undefined) ||
			"Template"
		);
	};

	const getCapabilityDescription = (
		item: Record<string, unknown>,
	): string | undefined => {
		const description =
			(item.description as string | undefined) ||
			(item.short_description as string | undefined) ||
			(item.summary as string | undefined) ||
			undefined;
		if (typeof description !== "string") return undefined;
		return description.trim() ? description : undefined;
	};

	const asRecordList = (value: unknown): Array<Record<string, unknown>> => {
		if (!Array.isArray(value)) return [];
		return value.filter(
			(entry): entry is Record<string, unknown> =>
				entry !== null && typeof entry === "object",
		);
	};

	const renderCapabilitySection = (
		kind: CapabilityKindKey,
		items: Record<string, unknown>[],
	) => {
		if (!items.length) return null;
		const badgeText = capabilityBadge[kind];
		const label = capabilityLabels[kind];
		return (
			<div key={kind} className="space-y-2">
				<div className="text-xs font-semibold uppercase tracking-wide text-slate-500">
					{label}
					<span className="ml-1 text-[11px] font-normal text-slate-400">
						({items.length})
					</span>
				</div>
				<div className="grid gap-2">
					{items.map((item, idx) => {
						const title = getCapabilityTitle(kind, item);
						const description = getCapabilityDescription(item);
						return (
							<div
								key={`${kind}-${idx}-${title}`}
								className="flex items-start gap-3 rounded border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-900/40"
							>
								<div className="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-slate-200 text-[11px] font-semibold text-slate-600 dark:bg-slate-800 dark:text-slate-200">
									{badgeText}
								</div>
								<div className="min-w-0 flex-1">
									<div className="font-medium text-slate-800 dark:text-slate-100">
										{title}
									</div>
									{description ? (
										<div className="mt-1 text-xs text-slate-500 dark:text-slate-300">
											{description}
										</div>
									) : null}
								</div>
							</div>
						);
					})}
				</div>
			</div>
		);
	};

	return (
		<Drawer open={open} onOpenChange={(value) => (value ? undefined : close())}>
			<DrawerContent className="h-full flex flex-col">
				<DrawerHeader>
					<div className="flex items-start justify-between gap-2">
						<div>
							<DrawerTitle className="flex items-center gap-2">
								Add MCP Servers
							</DrawerTitle>
							<DrawerDescription>
								Confirm installation for {drafts.length} server
								{drafts.length === 1 ? "" : "s"}
								{source ? ` • Source: ${sourceLabel[source]}` : null}
							</DrawerDescription>
						</div>
						<Button
							variant="ghost"
							size="icon"
							aria-label="Retry preview"
							title="Retry preview"
							disabled={isPreviewLoading || isImporting}
							onClick={() => {
								// Re-run preview with current drafts/source
								pipeline.setPreviewState(null);
								pipeline.begin(drafts, source ?? "manual");
							}}
						>
							{isPreviewLoading ? (
								<Loader2 className="h-4 w-4 animate-spin" />
							) : (
								<RefreshCw className="h-4 w-4" />
							)}
						</Button>
					</div>
				</DrawerHeader>

				<div className="flex-1 px-4 pb-4 space-y-3 flex flex-col min-h-0">
					{previewError ? (
						<Alert variant="destructive" className="mb-4">
							<AlertTriangle className="h-4 w-4" />
							<AlertTitle>Preview failed</AlertTitle>
							<AlertDescription>{previewError}</AlertDescription>
						</Alert>
					) : null}

					{isPreviewLoading ? (
						<div className="flex items-center justify-center gap-2 rounded border border-slate-200 bg-white px-3 py-2 text-sm text-slate-500 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-300">
							<Loader2 className="h-4 w-4 animate-spin" /> Generating capability
							preview…
						</div>
					) : null}

					{previewState?.success === false && previewState?.error ? (
						<Alert variant="default" className="mb-4">
							<AlertTriangle className="h-4 w-4" />
							<AlertTitle>Preview reported issues</AlertTitle>
							<AlertDescription>
								Some servers could not be contacted during preview. You can
								still proceed—the proxy will retry after installation.
							</AlertDescription>
						</Alert>
					) : null}

					<ScrollArea className="flex-1 pr-3">
						<div className="space-y-3 pb-2">
							{drafts.map((draft) => {
								const item = previewItemsByName.get(draft.name);
								const ok = item?.ok !== false;
								const tools = asRecordList(item?.tools?.items);
								const resources = asRecordList(item?.resources?.items);
								const templates = asRecordList(item?.resource_templates?.items);
								const prompts = asRecordList(item?.prompts?.items);
								const summaryParts = [] as string[];
								if (tools.length)
									summaryParts.push(
										`${tools.length} ${tools.length === 1 ? "tool" : "tools"}`,
									);
								if (resources.length)
									summaryParts.push(
										`${resources.length} ${resources.length === 1 ? "resource" : "resources"}`,
									);
								if (templates.length)
									summaryParts.push(
										`${templates.length} ${templates.length === 1 ? "template" : "templates"}`,
									);
								if (prompts.length)
									summaryParts.push(
										`${prompts.length} ${prompts.length === 1 ? "prompt" : "prompts"}`,
									);
								const summaryText = summaryParts.join(" · ");
								const isOpen = !!expanded[draft.name];
								return (
									<div key={draft.name} className="rounded border px-3 py-2">
										<div className="flex items-center justify-between gap-2">
											<div className="flex items-center gap-2 min-w-0">
												<div
													role="button"
													className="p-0 text-slate-500 hover:text-slate-800 dark:hover:text-slate-200"
													onClick={() => toggleExpanded(draft.name)}
													aria-label={isOpen ? "Collapse" : "Expand"}
												>
													{isOpen ? (
														<ChevronDown className="h-4 w-4" />
													) : (
														<ChevronRight className="h-4 w-4" />
													)}
												</div>
												<div
													className="font-medium text-sm truncate"
													title={draft.name}
												>
													{draft.name}
													{summaryText ? (
														<span className="ml-2 text-xs text-slate-500">
															{summaryText}
														</span>
													) : null}
												</div>
											</div>
											<Badge variant="secondary" className="text-xs">
												{draft.kind}
											</Badge>
										</div>

										{!ok && item?.error ? (
											<div className="mt-2 text-xs text-red-500">
												{String(item.error)}
											</div>
										) : null}

										{/* Details */}
										{isOpen ? (
											<div className="mt-3 space-y-4">
												{renderCapabilitySection("tools", tools)}
												{renderCapabilitySection("resources", resources)}
												{renderCapabilitySection("templates", templates)}
												{renderCapabilitySection("prompts", prompts)}
											</div>
										) : null}
									</div>
								);
							})}
						</div>
					</ScrollArea>
				</div>

				<DrawerFooter className="border-t bg-slate-50 dark:bg-slate-900/20">
					<div className="flex w-full items-center justify-between gap-3">
						{onBack && drafts.length === 1 ? (
							<Button
								variant="ghost"
								onClick={() => onBack?.(drafts, source)}
								disabled={isImporting || isPreviewLoading}
							>
								Back
							</Button>
						) : (
							<div />
						)}
						<Button
							onClick={confirmImport}
							disabled={isImporting || isPreviewLoading}
						>
							{(isImporting || isPreviewLoading) && (
								<Loader2 className="mr-2 h-4 w-4 animate-spin" />
							)}
							Install
						</Button>
					</div>
				</DrawerFooter>
			</DrawerContent>
		</Drawer>
	);
}
