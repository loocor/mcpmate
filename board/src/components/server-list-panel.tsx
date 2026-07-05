import { useMemo, useState } from "react";
import { Eye } from "lucide-react";
import { useTranslation } from "react-i18next";
import { CachedAvatar } from "./cached-avatar";
import {
	BulkSelectionCheckbox,
	BulkSelectionHeader,
	useBulkSelection,
	useBulkSelectionLabels,
	useEnableDisableBulkActions,
} from "./bulk-selection";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "./capsule-stripe-list";
import {
	CapsuleStripeLeadCircle,
	CapsuleStripeRowBody,
} from "./capsule-stripe-row";
import { CardListScrollBody } from "./card-list-scroll-body";
import { Badge } from "./ui/badge";
import { Input } from "./ui/input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./ui/select";
import { Switch } from "./ui/switch";
import { toolbarSearchInputClassName } from "./ui/page-toolbar";
import { cn } from "../lib/utils";

const ALL_SERVERS_ID = "__all_servers__";

const compactSelectTriggerClass =
	"relative h-9 w-full min-w-9 px-2 pr-8 [&>span]:min-w-0 [&>span]:truncate [&>svg]:pointer-events-none [&>svg]:absolute [&>svg]:right-2.5 [&>svg]:top-1/2 [&>svg]:-translate-y-1/2";

export type ServerListPanelItem = {
	id: string;
	name: string;
	enabled?: boolean;
	serverType?: string | null;
	endpointSummary?: string | null;
	iconSrc?: string | null;
	source?: string;
	capabilityCount?: string;
};

/** Height of one CapsuleStripeListItem row: p-2 (16px) + content (~40px) = 56px. */
const SERVER_ROW_HEIGHT_PX = 56;

type ServerListPanelProps = {
	servers: ServerListPanelItem[];
	selectedId: string | null;
	onSelect: (id: string) => void;
	onToggleEnabled?: (id: string, enabled: boolean) => void;
	onBrowseServer?: (id: string) => void;
	toggleDisabled?: boolean;
	isLoading?: boolean;
	showSearch?: boolean;
	showStatusFilter?: boolean;
	showSwitch?: boolean;
	showBulkMode?: boolean;
	showSelectionIndicator?: boolean;
	showAllServersRow?: boolean;
	allServersTotalCount?: number;
	allServersCapabilitySummary?: string;
	onBulkAction?: (action: "enable" | "disable", ids: string[]) => void;
	/** Constrain the scroll area to show at most this many rows before scrolling. */
	maxVisibleItems?: number;
	className?: string;
};

function serverSubtitle(server: ServerListPanelItem): string {
	const parts: string[] = [];
	if (server.source) parts.push(server.source.charAt(0).toUpperCase() + server.source.slice(1));
	if (server.endpointSummary) parts.push(server.endpointSummary);
	return parts.join(" · ");
}

export function ServerListPanel({
	servers,
	selectedId,
	onSelect,
	onToggleEnabled,
	onBrowseServer,
	toggleDisabled = false,
	isLoading = false,
	showSearch = true,
	showStatusFilter = false,
	showSwitch = false,
	showBulkMode = false,
	showSelectionIndicator = false,
	showAllServersRow = false,
	allServersTotalCount,
	allServersCapabilitySummary,
	onBulkAction,
	maxVisibleItems,
	className,
}: ServerListPanelProps) {
	const { t, i18n } = useTranslation();
	const [searchQuery, setSearchQuery] = useState("");
	const [statusFilter, setStatusFilter] = useState<
		"all" | "enabled" | "disabled"
	>("all");
	const { bulkModeDescription } = useBulkSelectionLabels();
	const serverBulk = useBulkSelection<string>();

	const filteredServers = useMemo(
		() =>
			servers.filter((s) => {
				const queryPass =
					searchQuery.trim() === "" ||
					s.name.toLowerCase().includes(searchQuery.toLowerCase());
				const statusPass =
					statusFilter === "all" ||
					(statusFilter === "enabled" ? s.enabled : !s.enabled);
				return queryPass && statusPass;
			}),
		[searchQuery, statusFilter, servers],
	);

	const statusLabel = useMemo(() => {
		if (statusFilter === "enabled") {
			return t("servers:detail.filters.status.enabled", {
				defaultValue: "Enabled",
			});
		}
		if (statusFilter === "disabled") {
			return t("servers:detail.filters.status.disabled", {
				defaultValue: "Disabled",
			});
		}
		return t("servers:detail.filters.status.all", {
			defaultValue: "All",
		});
	}, [i18n.language, statusFilter, t]);

	const bulkActions = useEnableDisableBulkActions(
		serverBulk,
		filteredServers.map((s) => s.id),
		onBulkAction
			? {
				mutate: ({ ids, enable }: { ids: string[]; enable: boolean }) =>
					onBulkAction(enable ? "enable" : "disable", ids),
				isPending: false,
			}
			: { mutate: () => { }, isPending: false },
	);

	const isAllServersSelected = selectedId === ALL_SERVERS_ID;
	const showRowSelectionCircle = showSelectionIndicator && !serverBulk.isBulkMode;
	const showBulkSelectionControl = showBulkMode;
	const showLeadSelectionSpace = showBulkSelectionControl && serverBulk.isBulkMode;

	return (
		<div className={cn("flex min-h-0 flex-col", className)}>
			{(showSearch || showStatusFilter || showBulkMode) && (
				<div className="shrink-0 pb-3">
					{showBulkMode && (
						<BulkSelectionHeader
							className="mb-3"
							title={t("profiles:detail.labels.servers", {
								defaultValue: "Servers",
							})}
							description={
								serverBulk.isBulkMode
									? bulkModeDescription(serverBulk.selectedCount)
									: t("profiles:detail.descriptions.capabilityServers", {
										defaultValue:
											"Select a server to manage its profile capabilities.",
									})
							}
							isBulkMode={serverBulk.isBulkMode}
							onToggleBulkMode={serverBulk.toggleMode}
							actions={bulkActions}
						/>
					)}
					{(showSearch || showStatusFilter) && (
						<div
							className={cn(
								"grid min-w-0 gap-2 overflow-visible",
								showSearch && showStatusFilter
									? "grid-cols-[minmax(0,3fr)_minmax(2.25rem,1fr)]"
									: "grid-cols-1",
							)}
						>
							{showSearch && (
								<Input
									placeholder={t(
										"profiles:detail.placeholders.searchServers",
										{ defaultValue: "Search servers..." },
									)}
									value={searchQuery}
									onChange={(e) => setSearchQuery(e.target.value)}
									className={cn(toolbarSearchInputClassName, "min-w-0")}
								/>
							)}
							{showStatusFilter && (
								<Select
									value={statusFilter}
									onValueChange={(v) =>
										setStatusFilter(v as "all" | "enabled" | "disabled")
									}
								>
									<SelectTrigger
										title={statusLabel}
										className={compactSelectTriggerClass}
									>
										<SelectValue
											placeholder={t(
												"profiles:detail.placeholders.status",
												{ defaultValue: "Status" },
											)}
										/>
									</SelectTrigger>
									<SelectContent>
										<SelectItem value="all">
											{t("servers:detail.filters.status.all", {
												defaultValue: "All",
											})}
										</SelectItem>
										<SelectItem value="enabled">
											{t("servers:detail.filters.status.enabled", {
												defaultValue: "Enabled",
											})}
										</SelectItem>
										<SelectItem value="disabled">
											{t("servers:detail.filters.status.disabled", {
												defaultValue: "Disabled",
											})}
										</SelectItem>
									</SelectContent>
								</Select>
							)}
						</div>
					)}
				</div>
			)}
			<CardListScrollBody
				style={maxVisibleItems != null ? { maxHeight: maxVisibleItems * SERVER_ROW_HEIGHT_PX } : undefined}
			>
				{isLoading ? (
					<div className="space-y-3">
						{["s1", "s2", "s3"].map((id) => (
							<div
								key={`server-list-skel-${id}`}
								className="h-16 animate-pulse rounded-md bg-slate-200 dark:bg-slate-800"
							/>
						))}
					</div>
				) : filteredServers.length > 0 ? (
					<CapsuleStripeList className="overflow-visible rounded-none border-0">
						{showAllServersRow && (
							<CapsuleStripeListItem
								key={ALL_SERVERS_ID}
								interactive
								className={`group relative px-3 transition-colors ${isAllServersSelected ? "bg-primary/10" : ""
									}`}
								onClick={() => onSelect(ALL_SERVERS_ID)}
								onKeyDown={(event) => {
									if (event.key === "Enter" || event.key === " ") {
										event.preventDefault();
										onSelect(ALL_SERVERS_ID);
									}
								}}
							>
								<CapsuleStripeRowBody
									lead={
										<div
											className={cn(
												"flex items-center transition-[gap] duration-200",
												showLeadSelectionSpace || showRowSelectionCircle
													? "gap-3"
													: "gap-0",
											)}
										>
											{showRowSelectionCircle ? (
												<CapsuleStripeLeadCircle
													variant="toggle"
													selected={isAllServersSelected}
												/>
											) : null}
											<div className="flex h-9 w-9 items-center justify-center rounded-md border border-slate-200 bg-white text-[10px] font-semibold uppercase text-slate-600 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-300">
												{t("profiles:detail.labels.allServersShort", {
													defaultValue: "All",
												})}
											</div>
										</div>
									}
									trailing={
										allServersTotalCount != null ? (
											<Badge variant="outline">
												{allServersTotalCount}
											</Badge>
										) : undefined
									}
								>
									<div className="min-w-0">
										<div
											className="truncate font-medium capitalize text-slate-900 dark:text-slate-100"
											title={t("profiles:detail.labels.allServers", {
												defaultValue: "All servers",
											})}
										>
											{t("profiles:detail.labels.allServers", {
												defaultValue: "All servers",
											})}
										</div>
										{allServersCapabilitySummary && (
											<div
												className="mt-1 truncate text-xs text-slate-500"
												title={allServersCapabilitySummary}
											>
												{allServersCapabilitySummary}
											</div>
										)}
									</div>
								</CapsuleStripeRowBody>
							</CapsuleStripeListItem>
						)}
						{filteredServers.map((server) => {
							const avatarFallback = (server.name || server.id || "S")
								.slice(0, 1)
								.toUpperCase();
							const isSelected = selectedId === server.id;
							const bulkSelected =
								serverBulk.isBulkMode &&
								serverBulk.selectedIdSet.has(server.id);
							let serverItemStateClass = "";
							if (isSelected) {
								serverItemStateClass = "bg-primary/10";
							} else if (bulkSelected) {
								serverItemStateClass = "bg-accent/40";
							}
							const serverLeadClassName = cn(
								"flex items-center transition-[gap] duration-200",
								showLeadSelectionSpace || showRowSelectionCircle
									? "gap-3"
									: "gap-0",
							);

							return (
								<CapsuleStripeListItem
									key={server.id}
									interactive
									className={`group relative px-3 transition-colors ${serverItemStateClass}`}
									onClick={() => onSelect(server.id)}
									onKeyDown={(event) => {
										if (event.key === "Enter" || event.key === " ") {
											event.preventDefault();
											onSelect(server.id);
										}
									}}
								>
									<CapsuleStripeRowBody
										lead={
											<div className={serverLeadClassName}>
												{showBulkMode ? (
													<BulkSelectionCheckbox
														visible={serverBulk.isBulkMode}
														checked={bulkSelected}
														onToggle={() =>
															serverBulk.toggleItem(server.id)
														}
														ariaLabel={t(
															"profiles:detail.bulk.selectItem",
															{
																name: server.name,
																defaultValue: "Select {{name}}",
															},
														)}
													/>
												) : showRowSelectionCircle ? (
													<CapsuleStripeLeadCircle
														variant="toggle"
														selected={isSelected}
													/>
												) : null}
												<CachedAvatar
													src={server.iconSrc ?? undefined}
													alt={
														server.name
															? `${server.name} icon`
															: undefined
													}
													fallback={avatarFallback}
													size="sm"
													shape="rounded"
													className="border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-900/40"
												/>
											</div>
										}
										trailing={
											(showSwitch || onBrowseServer) ? (
												<div className="flex w-[4.25rem] shrink-0 items-center justify-end gap-1">
													{onBrowseServer && !serverBulk.isBulkMode && (
														<button
															type="button"
															className="flex h-7 w-7 shrink-0 items-center justify-center border-0 bg-transparent p-0 text-muted-foreground opacity-0 shadow-none transition-[color,opacity] hover:bg-transparent hover:text-foreground focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-accent/60 group-hover:opacity-100"
															onClick={(event) => {
																event.stopPropagation();
																onBrowseServer(server.id);
															}}
															aria-label={t(
																"profiles:detail.labels.browseServer",
																{ defaultValue: "Browse server" },
															)}
														>
															<Eye className="h-4 w-4" />
														</button>
													)}
													{showSwitch && onToggleEnabled && (
														<Switch
															checked={server.enabled ?? false}
															onClick={(e) => e.stopPropagation()}
															onCheckedChange={(enabled) =>
																onToggleEnabled(server.id, enabled)
															}
															disabled={toggleDisabled}
														/>
													)}
												</div>
											) : undefined
										}
									>
										<div className="min-w-0">
											<div
												className="truncate font-medium capitalize text-slate-900 dark:text-slate-100"
												title={server.name}
											>
												{server.name}
											</div>
											{(server.capabilityCount || server.source || server.endpointSummary) && (
												<div
													className="mt-1 truncate text-xs text-slate-500"
													title={server.capabilityCount ?? serverSubtitle(server)}
												>
													{server.capabilityCount ?? serverSubtitle(server)}
												</div>
											)}
										</div>
									</CapsuleStripeRowBody>
								</CapsuleStripeListItem>
							);
						})}
					</CapsuleStripeList>
				) : (
					<div className="flex min-h-full items-center justify-center px-4 py-8 text-center text-sm text-muted-foreground">
						{t("profiles:detail.emptyStates.noServers", {
							defaultValue: "No servers found in this profile",
						})}
					</div>
				)}
			</CardListScrollBody>
		</div>
	);
}
