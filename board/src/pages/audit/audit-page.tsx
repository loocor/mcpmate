import { useQuery } from "@tanstack/react-query";
import { RefreshCw, Search, AlertCircle, Inbox, ChevronRight, ChevronDown, X } from "lucide-react";
import { Fragment, useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import { Pagination } from "../../components/pagination";
import { Button } from "../../components/ui/button";
import { Card, CardContent } from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import { auditApi } from "../../lib/api";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError } from "../../lib/notify";
import type { AuditCategory, AuditEventRecord, AuditStatus } from "../../lib/types";
import { formatLocalDateTime } from "../../lib/utils";
import { AuditEventDetailDrawer } from "./components/audit-event-detail-drawer";
import { AuditEventDetails } from "./components/audit-event-details";

const DEFAULT_PAGE_SIZE = 10;
const PAGE_SIZE_OPTIONS = [10, 20, 50, 100] as const;
const ALL_CATEGORIES = "all";
const ALL_STATUSES = "all";

const CATEGORY_OPTIONS: AuditCategory[] = [
	"mcp_request",
	"server_config",
	"profile_config",
	"client_config",
	"runtime_control",
	"management",
];

const STATUS_OPTIONS: AuditStatus[] = ["success", "failed", "cancelled"];

/** Fixed-width gutter so ChevronRight/ChevronDown swaps do not shift the layout */
const EXPAND_COL_BASE = "box-border w-10 min-w-10 max-w-10 px-0 pl-1 pr-2";
const EXPAND_COL_TH_CLASS = `${EXPAND_COL_BASE} py-2 align-middle`;
const EXPAND_COL_TD_CLASS = `${EXPAND_COL_BASE} py-3 align-middle`;
const EXPAND_COL_SPACER_CLASS = `${EXPAND_COL_BASE} border-b-0 p-0 align-middle`;

/** Distributes width so Target absorbs remainder; ellipsis relies on min-w-0 + truncate inside cells */
function AuditEventsTableColgroup() {
	return (
		<colgroup>
			<col className="w-10" />
			<col className="w-[11rem]" />
			<col className="w-[9.5rem]" />
			<col className="w-[9.5rem]" />
			<col className="w-[6.5rem]" />
			<col />
			<col className="w-[9rem]" />
		</colgroup>
	);
}

function EventRowSkeleton() {
	return (
		<tr className="border-b align-middle">
			<td className={EXPAND_COL_TD_CLASS}>
				<span className="inline-flex h-8 w-8 shrink-0 items-center justify-center">
					<div className="h-4 w-4 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
				</span>
			</td>
			<td className="py-3 pr-4 align-middle">
				<div className="h-4 w-32 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
			</td>
			<td className="py-3 pr-4 align-middle">
				<div className="h-4 w-20 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
			</td>
			<td className="py-3 pr-4 align-middle">
				<div className="h-4 w-24 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
			</td>
			<td className="py-3 pr-4 align-middle">
				<div className="h-5 w-16 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
			</td>
			<td className="min-w-0 max-w-0 py-3 pr-4 align-middle">
				<div className="h-4 w-full max-w-full bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
			</td>
			<td className="py-3 pl-2 pr-4 align-middle text-right tabular-nums">
				<div className="ml-auto h-4 w-12 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
			</td>
		</tr>
	);
}

function EventsTableSkeleton() {
	const { t } = useTranslation("audit");
	return (
		<table className="w-full table-fixed text-sm">
			<AuditEventsTableColgroup />
			<thead className="sticky top-0 z-[1] bg-white dark:bg-slate-900">
				<tr className="border-b border-slate-200 text-left text-muted-foreground dark:border-slate-700">
					<th scope="col" className={`${EXPAND_COL_TH_CLASS} font-normal`}>
						<span className="sr-only">
							{t("audit:headers.expandColumn", { defaultValue: "Expand row" })}
						</span>
					</th>
					<th className="py-2 pr-4 whitespace-nowrap">
						{t("audit:headers.timestamp", { defaultValue: "Timestamp" })}
					</th>
					<th className="py-2 pr-4 whitespace-nowrap">
						{t("audit:headers.action", { defaultValue: "Action" })}
					</th>
					<th className="py-2 pr-4 whitespace-nowrap">
						{t("audit:headers.category", { defaultValue: "Category" })}
					</th>
					<th className="py-2 pr-4 whitespace-nowrap">
						{t("audit:headers.status", { defaultValue: "Status" })}
					</th>
					<th className="min-w-0 py-2 pr-4">
						{t("audit:headers.target", { defaultValue: "Target" })}
					</th>
					<th className="whitespace-nowrap py-2 pl-2 pr-4 text-right font-normal">
						{t("audit:headers.duration", { defaultValue: "Duration (ms)" })}
					</th>
				</tr>
			</thead>
			<tbody>
				{Array.from({ length: 8 }).map((_, i) => (
					<EventRowSkeleton key={i} />
				))}
			</tbody>
		</table>
	);
}

function EmptyState({ hasFilters }: { hasFilters: boolean }) {
	const { t } = useTranslation("audit");

	return (
		<div className="flex flex-col items-center justify-center py-12 text-center">
			{hasFilters ? (
				<>
					<Search className="h-12 w-12 text-muted-foreground/50 mb-4" />
					<p className="text-base font-medium text-muted-foreground">
						{t("audit:states.noMatches", { defaultValue: "No events match your filters" })}
					</p>
					<p className="text-sm text-muted-foreground/70 mt-1">
						{t("audit:states.noMatchesHint", { defaultValue: "Try adjusting your search or filter criteria" })}
					</p>
				</>
			) : (
				<>
					<Inbox className="h-12 w-12 text-muted-foreground/50 mb-4" />
					<p className="text-base font-medium text-muted-foreground">
						{t("audit:states.empty", { defaultValue: "No audit events recorded yet" })}
					</p>
					<p className="text-sm text-muted-foreground/70 mt-1">
						{t("audit:states.emptyHint", { defaultValue: "Events will appear here as you interact with MCP servers and profiles" })}
					</p>
				</>
			)}
		</div>
	);
}

export function AuditPage() {
	usePageTranslations("audit");
	const { t, i18n } = useTranslation("audit");
	const [search, setSearch] = useState("");
	const [category, setCategory] = useState<string>(ALL_CATEGORIES);
	const [status, setStatus] = useState<string>(ALL_STATUSES);
	const [pageSize, setPageSize] = useState<number>(DEFAULT_PAGE_SIZE);
	const [pageCursors, setPageCursors] = useState<string[]>([]);
	const [currentPageIndex, setCurrentPageIndex] = useState(0);
	const [liveEvents, setLiveEvents] = useState<AuditEventRecord[]>([]);
	const [expandedRowKey, setExpandedRowKey] = useState<string | null>(null);
	const [drawerEventId, setDrawerEventId] = useState<number | null>(null);
	const [drawerOpen, setDrawerOpen] = useState(false);
	const [connectionState, setConnectionState] = useState<"live" | "disconnected">("disconnected");
	const [isPaginationActionLoading, setIsPaginationActionLoading] = useState(false);

	const currentCursor = pageCursors[currentPageIndex];

	/** REST loads one cursor page per request (refresh included); smaller page sizes reduce SQLite work. */
	const query = useQuery({
		queryKey: ["audit", "events", currentCursor, category, status, pageSize],
		queryFn: async () =>
			auditApi.list({
				limit: pageSize,
				cursor: currentCursor,
				category: category !== ALL_CATEGORIES ? category : undefined,
				status: status !== ALL_STATUSES ? status : undefined,
			}),
		refetchOnWindowFocus: false,
		retry: false,
	});

	useEffect(() => {
		if (query.isError) {
			notifyError(
				t("audit:errors.loadFailed", { defaultValue: "Failed to load audit events" }),
				query.error?.message ?? String(query.error)
			);
		}
	}, [query.isError, query.error, t]);

	const hasActiveFilters = search.trim() !== "" || category !== ALL_CATEGORIES || status !== ALL_STATUSES;

	useEffect(() => {
		setPageCursors([]);
		setCurrentPageIndex(0);
		setExpandedRowKey(null);
	}, [category, status, pageSize]);

	/** WS sends one JSON event per message (incremental), not a full list. */
	useEffect(() => {
		const socket = new WebSocket(auditApi.eventsWsUrl());
		socket.onopen = () => setConnectionState("live");
		socket.onclose = () => setConnectionState("disconnected");
		socket.onerror = () => setConnectionState("disconnected");
		socket.onmessage = (event) => {
			try {
				const parsed = JSON.parse(event.data) as AuditEventRecord;
				setLiveEvents((current) => [parsed, ...current].slice(0, pageSize));
			} catch {
				setConnectionState("disconnected");
			}
		};

		return () => socket.close();
	}, [pageSize]);

	const currentLoadedEvents = query.data?.events ?? [];
	const displayEvents = useMemo<AuditEventRecord[]>(() => {
		if (currentPageIndex === 0) {
			const seen = new Set<string>();
			return [...liveEvents, ...currentLoadedEvents].filter((event) => {
				const key = toEventKey(event);
				if (seen.has(key)) {
					return false;
				}
				seen.add(key);
				return true;
			});
		}
		return currentLoadedEvents;
	}, [liveEvents, currentLoadedEvents, currentPageIndex]);

	const filteredEvents = useMemo(() => {
		const keyword = search.trim().toLowerCase();
		if (!keyword) {
			return displayEvents;
		}

		return displayEvents.filter((event) => {
			const categoryKey =
				event.category === ("capability_control" as AuditCategory)
					? "profile_config"
					: event.category;
			const actionLabel = t(`audit:actionValues.${event.action}`, {
				defaultValue: event.action,
			}).toLowerCase();
			const categoryLabel = t(`audit:categoryValues.${categoryKey}`, {
				defaultValue: categoryKey,
			}).toLowerCase();

			const haystacks = [
				event.target,
				event.route,
				event.server_id,
				event.profile_id,
				event.client_id,
				event.session_id,
				event.action,
				actionLabel,
				categoryLabel,
				event.mcp_method,
			]
				.filter(Boolean)
				.map((value) => value?.toLowerCase() ?? "");

			return haystacks.some((value) => value.includes(keyword));
		});
		// i18n.language: recomputed translated action/category labels must participate in filtering after locale switch.
		// eslint-disable-next-line react-hooks/exhaustive-deps -- i18n.language is intentional (see board/AGENTS.md i18n hook deps)
	}, [displayEvents, i18n.language, search, t]);

	const toggleExpanded = useCallback((key: string) => {
		setExpandedRowKey((current) => (current === key ? null : key));
	}, []);

	const handleRowToggle = useCallback(
		(event: React.MouseEvent<HTMLTableRowElement>, rowKey: string) => {
			const target = event.target as HTMLElement;
			if (target.closest("a, button, summary")) {
				return;
			}
			toggleExpanded(rowKey);
		},
		[toggleExpanded],
	);

	const openRawDetails = useCallback((eventId: number) => {
		setDrawerEventId(eventId);
		setDrawerOpen(true);
	}, []);

	const handleNextPage = useCallback(() => {
		if (query.data?.next_cursor) {
			const nextCursor = query.data.next_cursor;
			setPageCursors((prev) => {
				const next = [...prev];
				next[currentPageIndex + 1] = nextCursor;
				return next;
			});
			setCurrentPageIndex((prev) => prev + 1);
			setExpandedRowKey(null);
		}
	}, [query.data, currentPageIndex]);

	const handlePrevPage = useCallback(() => {
		if (currentPageIndex > 0) {
			setCurrentPageIndex((prev) => prev - 1);
			setExpandedRowKey(null);
		}
	}, [currentPageIndex]);

	const handleFirstPage = useCallback(() => {
		setCurrentPageIndex(0);
		setExpandedRowKey(null);
	}, []);

	const handleLastPage = useCallback(async () => {
		if (!query.data?.next_cursor) {
			return;
		}

		setIsPaginationActionLoading(true);
		try {
			let nextCursor: string | undefined = query.data.next_cursor ?? undefined;
			let targetPageIndex = currentPageIndex;
			const nextPageCursors = [...pageCursors];

			while (nextCursor) {
				targetPageIndex += 1;
				nextPageCursors[targetPageIndex] = nextCursor;
				const page = await auditApi.list({
					limit: pageSize,
					cursor: nextCursor,
					category: category !== ALL_CATEGORIES ? category : undefined,
					status: status !== ALL_STATUSES ? status : undefined,
				});
				nextCursor = page.next_cursor ?? undefined;
			}

			setPageCursors(nextPageCursors);
			setCurrentPageIndex(targetPageIndex);
			setExpandedRowKey(null);
		} finally {
			setIsPaginationActionLoading(false);
		}
	}, [category, currentPageIndex, pageCursors, pageSize, query.data?.next_cursor, status]);

	const auditTotalPages = useMemo(() => {
		if (query.data?.next_cursor) {
			return null;
		}
		return currentPageIndex + 1;
	}, [currentPageIndex, query.data?.next_cursor]);

	const handleGoToPage = useCallback(
		async (targetPage: number) => {
			const p = Math.max(1, Math.floor(Number(targetPage)));
			const targetIndex = p - 1;
			if (targetIndex === currentPageIndex) {
				return;
			}
			if (targetIndex === 0) {
				handleFirstPage();
				return;
			}
			if (targetIndex < currentPageIndex) {
				if (targetIndex > 0 && pageCursors[targetIndex] === undefined) {
					handleFirstPage();
					return;
				}
				setCurrentPageIndex(targetIndex);
				setExpandedRowKey(null);
				return;
			}
			if (pageCursors[targetIndex] !== undefined) {
				setCurrentPageIndex(targetIndex);
				setExpandedRowKey(null);
				return;
			}
			let nextCursor: string | undefined = query.data?.next_cursor ?? undefined;
			if (!nextCursor) {
				return;
			}
			setIsPaginationActionLoading(true);
			try {
				const nextCursors = [...pageCursors];
				let idx = currentPageIndex;
				while (idx < targetIndex && nextCursor) {
					idx += 1;
					nextCursors[idx] = nextCursor;
					const page = await auditApi.list({
						limit: pageSize,
						cursor: nextCursor,
						category: category !== ALL_CATEGORIES ? category : undefined,
						status: status !== ALL_STATUSES ? status : undefined,
					});
					nextCursor = page.next_cursor ?? undefined;
				}
				setPageCursors(nextCursors);
				if (nextCursors[targetIndex] !== undefined && idx === targetIndex) {
					setCurrentPageIndex(targetIndex);
				} else if (idx > currentPageIndex && nextCursors[idx] !== undefined) {
					setCurrentPageIndex(idx);
				}
				setExpandedRowKey(null);
			} finally {
				setIsPaginationActionLoading(false);
			}
		},
		[
			category,
			currentPageIndex,
			handleFirstPage,
			pageCursors,
			pageSize,
			query.data?.next_cursor,
			status,
		],
	);

	const renderStatusBadge = useCallback(
		(value: AuditStatus) => {
			let variant: "success" | "destructive" | "warning";
			switch (value) {
				case "success":
					variant = "success";
					break;
				case "failed":
					variant = "destructive";
					break;
				default:
					variant = "warning";
			}
			return (
				<Badge variant={variant}>
					{t(`audit:statusValues.${value}`, { defaultValue: value })}
				</Badge>
			);
		},
		[t],
	);

	const renderCategoryCell = useCallback(
		(event: AuditEventRecord) => {
			const categoryKey =
				event.category === ("capability_control" as AuditCategory)
					? "profile_config"
					: event.category;
			const primary = t(`audit:categoryValues.${categoryKey}`, {
				defaultValue: categoryKey,
			});
			return <span className="block min-w-0 truncate whitespace-nowrap">{primary}</span>;
		},
		[t],
	);

	return (
		<div className="flex h-full min-h-0 flex-col gap-4 overflow-hidden">
			<div className="sticky top-0 z-10 -mx-1 rounded-b-xl px-1 backdrop-blur">
				<div className="flex items-center gap-2 min-w-0">
					<p className="flex-1 min-w-0 truncate whitespace-nowrap text-base text-muted-foreground">
						{t("audit:description", {
							defaultValue: "Inspect audit events across REST and MCP flows",
						})}
					</p>
					<div className="flex min-w-0 shrink-0 flex-col gap-2 sm:flex-row sm:flex-wrap sm:items-center sm:justify-end">
						<div className="relative min-w-0 w-full sm:w-56 sm:shrink-0">
							<Input
								value={search}
								onChange={(event) => setSearch(event.target.value)}
								placeholder={t("audit:filters.search", {
									defaultValue: "Search target, route, server, profile, or client",
								})}
								className="h-9 w-full pr-10"
							/>
							{search.trim().length > 0 ? (
								<button
									type="button"
									className="absolute right-1.5 top-1/2 z-[1] flex h-7 w-7 -translate-y-1/2 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
									onClick={() => setSearch("")}
									aria-label={t("audit:filters.clearSearch", { defaultValue: "Clear search" })}
								>
									<X className="h-4 w-4 shrink-0" aria-hidden />
								</button>
							) : null}
						</div>
						<Select value={category} onValueChange={setCategory}>
							<SelectTrigger className="h-9 w-full sm:w-[160px]">
								<SelectValue placeholder={t("audit:filters.allCategories", { defaultValue: "All categories" })} />
							</SelectTrigger>
							<SelectContent align="end">
								<SelectItem value={ALL_CATEGORIES}>
									{t("audit:filters.allCategories", { defaultValue: "All categories" })}
								</SelectItem>
								{CATEGORY_OPTIONS.map((option) => (
									<SelectItem key={option} value={option}>
										{t(`audit:categoryValues.${option}`, { defaultValue: option })}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
						<Select value={status} onValueChange={setStatus}>
							<SelectTrigger className="h-9 w-full sm:w-[120px]">
								<SelectValue placeholder={t("audit:filters.allStatuses", { defaultValue: "All statuses" })} />
							</SelectTrigger>
							<SelectContent align="end">
								<SelectItem value={ALL_STATUSES}>
									{t("audit:filters.allStatuses", { defaultValue: "All statuses" })}
								</SelectItem>
								{STATUS_OPTIONS.map((option) => (
									<SelectItem key={option} value={option}>
										{t(`audit:statusValues.${option}`, { defaultValue: option })}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
						<Button
							type="button"
							variant="outline"
							size="sm"
							className="h-9 w-9 shrink-0 p-0"
							onClick={() => query.refetch()}
							disabled={query.isFetching}
							title={t("audit:buttons.refresh", { defaultValue: "Refresh" })}
						>
							<RefreshCw className={`h-4 w-4 ${query.isFetching ? "animate-spin" : ""}`} />
						</Button>
					</div>
				</div>
			</div>

			<Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
				<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-4">
					{query.isLoading && displayEvents.length === 0 ? (
						<div className="min-h-0 flex-1 overflow-auto overscroll-contain">
							<EventsTableSkeleton />
						</div>
					) : query.isError && displayEvents.length === 0 ? (
						<div className="flex min-h-0 flex-1 flex-col items-center justify-center py-12 text-center">
							<AlertCircle className="h-12 w-12 text-destructive/50 mb-4" />
							<p className="text-base font-medium text-muted-foreground">
								{t("audit:states.error", { defaultValue: "Failed to load audit events" })}
							</p>
							<Button variant="outline" size="sm" className="mt-4" onClick={() => query.refetch()}>
								{t("audit:buttons.retry", { defaultValue: "Retry" })}
							</Button>
						</div>
					) : filteredEvents.length === 0 ? (
						<div className="flex min-h-0 flex-1 flex-col items-center justify-center">
							<EmptyState hasFilters={hasActiveFilters} />
						</div>
					) : (
						<div className="min-h-0 flex-1 overflow-auto overscroll-contain">
							<table className="w-full table-fixed text-sm">
								<AuditEventsTableColgroup />
								<thead className="sticky top-0 z-[1] bg-white dark:bg-slate-900">
									<tr className="border-b border-slate-200 text-left text-muted-foreground dark:border-slate-700">
										<th scope="col" className={`${EXPAND_COL_TH_CLASS} font-normal`}>
											<span className="sr-only">
												{t("audit:headers.expandColumn", { defaultValue: "Expand row" })}
											</span>
										</th>
										<th className="py-2 pr-4 whitespace-nowrap">
											{t("audit:headers.timestamp", { defaultValue: "Timestamp" })}
										</th>
										<th className="py-2 pr-4 whitespace-nowrap">
											{t("audit:headers.action", { defaultValue: "Action" })}
										</th>
										<th className="py-2 pr-4 whitespace-nowrap">
											{t("audit:headers.category", { defaultValue: "Category" })}
										</th>
										<th className="py-2 pr-4 whitespace-nowrap">
											{t("audit:headers.status", { defaultValue: "Status" })}
										</th>
										<th className="min-w-0 py-2 pr-4">
											{t("audit:headers.target", { defaultValue: "Target" })}
										</th>
										<th className="whitespace-nowrap py-2 pl-2 pr-4 text-right font-normal">
											{t("audit:headers.duration", { defaultValue: "Duration (ms)" })}
										</th>
									</tr>
								</thead>
								<tbody>
									{filteredEvents.map((event) => {
										const rowKey = toEventKey(event);
										const expanded = expandedRowKey === rowKey;
										return (
											<Fragment key={rowKey}>
												<tr
													className="border-b align-middle odd:bg-background even:bg-slate-50/50 hover:bg-slate-100/70 dark:even:bg-slate-900/40 dark:hover:bg-slate-800/60 cursor-pointer"
													onClick={(clickEvent) => handleRowToggle(clickEvent, rowKey)}
												>
													<td className={`${EXPAND_COL_TD_CLASS} text-muted-foreground`}>
														<span className="inline-flex h-8 w-8 shrink-0 items-center justify-center">
															{expanded ? (
																<ChevronDown className="h-4 w-4 shrink-0" aria-hidden />
															) : (
																<ChevronRight className="h-4 w-4 shrink-0" aria-hidden />
															)}
														</span>
													</td>
													<td className="py-3 pr-4 align-middle whitespace-nowrap">
														{formatLocalDateTime(event.occurred_at_ms)}
													</td>
													<td className="overflow-hidden py-3 pr-4 align-middle">
														<span className="block min-w-0 truncate whitespace-nowrap">
															{t(`audit:actionValues.${event.action}`, { defaultValue: event.action })}
														</span>
													</td>
													<td className="overflow-hidden py-3 pr-4 align-middle">{renderCategoryCell(event)}</td>
													<td className="py-3 pr-4 align-middle whitespace-nowrap">{renderStatusBadge(event.status)}</td>
													<td className="min-w-0 max-w-0 py-3 pr-4 align-middle">
														<div className="truncate">
															{event.target ?? event.route ?? event.server_id ?? event.profile_id ?? event.client_id ?? "—"}
														</div>
													</td>
													<td className="py-3 pl-2 pr-4 align-middle text-right tabular-nums">
														{event.duration_ms != null ? String(event.duration_ms) : "—"}
													</td>
												</tr>
												{expanded ? (
													<tr className="border-b bg-slate-100/80 dark:bg-slate-900/70 last:border-0">
														<td className={EXPAND_COL_SPACER_CLASS} />
														<td colSpan={6} className="p-4">
															<AuditEventDetails event={event} t={t} onOpenRawDetails={openRawDetails} />
														</td>
													</tr>
												) : null}
											</Fragment>
										);
									})}
								</tbody>
							</table>
						</div>
					)}

					<Pagination
						currentPage={currentPageIndex + 1}
						hasPreviousPage={currentPageIndex > 0}
						hasNextPage={!!query.data?.next_cursor}
						isLoading={query.isFetching || isPaginationActionLoading}
						itemsPerPage={pageSize}
						currentPageItemCount={filteredEvents.length}
						totalPages={auditTotalPages}
						disableLastPageWhenTotalUnknown
						onGoToPage={handleGoToPage}
						onItemsPerPageChange={setPageSize}
						onPreviousPage={handlePrevPage}
						onFirstPage={handleFirstPage}
						onNextPage={handleNextPage}
						onLastPage={handleLastPage}
						pageSizeOptions={[...PAGE_SIZE_OPTIONS]}
						className="mt-4 shrink-0 border-t border-slate-200 pt-4 dark:border-slate-700"
						centerSlot={
							<span className="whitespace-nowrap">
								{t("audit:labels.liveStatus", { defaultValue: "Connection" })}:{" "}
								{connectionState === "live"
									? t("audit:states.live", { defaultValue: "Live" })
									: t("audit:states.disconnected", { defaultValue: "Disconnected" })}
							</span>
						}
					/>
				</CardContent>
			</Card>
			<AuditEventDetailDrawer open={drawerOpen} onOpenChange={setDrawerOpen} eventId={drawerEventId} />
		</div>
	);
}

function toEventKey(event: AuditEventRecord): string {
	return String(event.id ?? `${event.occurred_at_ms}-${event.action}-${event.target ?? ""}`);
}
