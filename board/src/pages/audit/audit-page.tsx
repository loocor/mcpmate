import { useQuery } from "@tanstack/react-query";
import { RefreshCw, Search, Filter, AlertCircle, Inbox } from "lucide-react";
import { Fragment, useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import { Pagination } from "../../components/pagination";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
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
	"capability_control",
	"rest_api",
];

const STATUS_OPTIONS: AuditStatus[] = ["success", "failed", "cancelled"];

function EventRowSkeleton() {
	return (
		<tr className="border-b">
			<td className="py-2 pr-4">
				<div className="h-4 w-32 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
			</td>
			<td className="py-2 pr-4">
				<div className="h-4 w-20 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
			</td>
			<td className="py-2 pr-4">
				<div className="h-4 w-24 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
			</td>
			<td className="py-2 pr-4">
				<div className="h-5 w-16 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
			</td>
			<td className="py-2 pr-4">
				<div className="h-4 w-40 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
			</td>
			<td className="py-2 pr-4">
				<div className="h-4 w-12 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
			</td>
			<td className="py-2">
				<div className="h-6 w-16 bg-slate-200 dark:bg-slate-800 animate-pulse rounded" />
			</td>
		</tr>
	);
}

function EventsTableSkeleton() {
	return (
		<table className="w-full text-sm">
			<thead className="sticky top-0 z-[1] bg-white dark:bg-slate-900">
				<tr className="border-b border-slate-200 text-left text-muted-foreground dark:border-slate-700">
					<th className="py-2 pr-4">Timestamp</th>
					<th className="py-2 pr-4">Action</th>
					<th className="py-2 pr-4">Category</th>
					<th className="py-2 pr-4">Status</th>
					<th className="py-2 pr-4">Target</th>
					<th className="py-2 pr-4">Duration</th>
					<th className="py-2" />
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
	const [expandedRows, setExpandedRows] = useState<Record<string, boolean>>({});
	const [connectionState, setConnectionState] = useState<"live" | "disconnected">("disconnected");
	const [isPaginationActionLoading, setIsPaginationActionLoading] = useState(false);

	const currentCursor = pageCursors[currentPageIndex];

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
		setExpandedRows({});
	}, [category, status, pageSize]);

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
			const haystacks = [
				event.target,
				event.route,
				event.server_id,
				event.profile_id,
				event.client_id,
				event.session_id,
				event.action,
				event.mcp_method,
			]
				.filter(Boolean)
				.map((value) => value?.toLowerCase() ?? "");

			return haystacks.some((value) => value.includes(keyword));
		});
	}, [displayEvents, search]);

	const toggleExpanded = useCallback((key: string) => {
		setExpandedRows((current) => ({ ...current, [key]: !current[key] }));
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
			setExpandedRows({});
		}
	}, [query.data, currentPageIndex]);

	const handlePrevPage = useCallback(() => {
		if (currentPageIndex > 0) {
			setCurrentPageIndex((prev) => prev - 1);
			setExpandedRows({});
		}
	}, [currentPageIndex]);

	const handleFirstPage = useCallback(() => {
		setCurrentPageIndex(0);
		setExpandedRows({});
	}, []);

	const handleLastPage = useCallback(async () => {
		if (!query.data?.next_cursor) {
			return;
		}

		setIsPaginationActionLoading(true);
		try {
			let nextCursor: string | undefined = query.data.next_cursor;
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
				nextCursor = page.next_cursor;
			}

			setPageCursors(nextPageCursors);
			setCurrentPageIndex(targetPageIndex);
			setExpandedRows({});
		} finally {
			setIsPaginationActionLoading(false);
		}
	}, [category, currentPageIndex, pageCursors, pageSize, query.data?.next_cursor, status]);

	const renderStatusBadge = useCallback(
		(value: AuditStatus) => {
			const variant =
				value === "success"
					? "success"
					: value === "failed"
						? "destructive"
						: "warning";
			return (
				<Badge variant={variant}>
					{t(`audit:statusValues.${value}`, { defaultValue: value })}
				</Badge>
			);
		},
		[t],
	);

	return (
		<div className="flex h-full min-h-0 flex-col gap-4 overflow-hidden">
			<div className="flex shrink-0 items-center gap-2 min-w-0">
				<p className="flex-1 min-w-0 truncate whitespace-nowrap text-base text-muted-foreground">
					{t("audit:title", {
						defaultValue: "Inspect audit events across REST and MCP flows",
					})}
				</p>
				<div className="flex items-center gap-2 flex-shrink-0">
					<Button variant="outline" size="sm" onClick={() => query.refetch()} disabled={query.isFetching}>
						<RefreshCw className={`mr-2 h-4 w-4 ${query.isFetching ? "animate-spin" : ""}`} />
						{t("audit:buttons.refresh", { defaultValue: "Refresh" })}
					</Button>
				</div>
			</div>

			<Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
				<CardHeader className="shrink-0">
					<div className="flex items-center justify-between gap-2">
						<CardTitle className="flex items-center gap-2">
							<Filter className="h-5 w-5" />
							{t("audit:filters.title", { defaultValue: "Audit Events" })}
						</CardTitle>
						<span className="text-xs text-muted-foreground">
							{t("audit:labels.liveStatus", { defaultValue: "Connection" })}:{" "}
							{connectionState === "live"
								? t("audit:states.live", { defaultValue: "Live" })
								: t("audit:states.disconnected", { defaultValue: "Disconnected" })}
						</span>
					</div>
					<div className="flex flex-col gap-3 pt-2 md:flex-row">
						<Input
							value={search}
							onChange={(event) => setSearch(event.target.value)}
							placeholder={t("audit:filters.search", {
								defaultValue: "Search target, route, server, profile, or client",
							})}
							className="md:max-w-sm"
						/>
						<Select value={category} onValueChange={setCategory}>
							<SelectTrigger className="md:w-[200px]">
								<SelectValue placeholder={t("audit:filters.allCategories", { defaultValue: "All categories" })} />
							</SelectTrigger>
							<SelectContent>
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
							<SelectTrigger className="md:w-[180px]">
								<SelectValue placeholder={t("audit:filters.allStatuses", { defaultValue: "All statuses" })} />
							</SelectTrigger>
							<SelectContent>
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
					</div>
				</CardHeader>
				<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden">
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
							<table className="w-full text-sm">
								<thead className="sticky top-0 z-[1] bg-white dark:bg-slate-900">
									<tr className="border-b border-slate-200 text-left text-muted-foreground dark:border-slate-700">
										<th className="py-2 pr-4">{t("audit:headers.timestamp", { defaultValue: "Timestamp" })}</th>
										<th className="py-2 pr-4">{t("audit:headers.action", { defaultValue: "Action" })}</th>
										<th className="py-2 pr-4">{t("audit:headers.category", { defaultValue: "Category" })}</th>
										<th className="py-2 pr-4">{t("audit:headers.status", { defaultValue: "Status" })}</th>
										<th className="py-2 pr-4">{t("audit:headers.target", { defaultValue: "Target" })}</th>
										<th className="py-2 pr-4">{t("audit:headers.duration", { defaultValue: "Duration" })}</th>
										<th className="py-2" />
									</tr>
								</thead>
								<tbody>
									{filteredEvents.map((event) => {
										const rowKey = toEventKey(event);
										const expanded = expandedRows[rowKey] ?? false;
										return (
											<Fragment key={rowKey}>
												<tr className="border-b align-middle">
													<td className="py-2 pr-4 whitespace-nowrap">
														{formatLocalDateTime(new Date(event.occurred_at_ms).toISOString(), i18n.language)}
													</td>
													<td className="py-2 pr-4">{t(`audit:actionValues.${event.action}`, { defaultValue: event.action })}</td>
													<td className="py-2 pr-4">
														{t(`audit:categoryValues.${event.category}`, { defaultValue: event.category })}
													</td>
													<td className="py-2 pr-4">{renderStatusBadge(event.status)}</td>
													<td className="py-2 pr-4 max-w-[260px] truncate">
														{event.target ?? event.route ?? event.server_id ?? event.profile_id ?? event.client_id ?? "—"}
													</td>
													<td className="py-2 pr-4">{event.duration_ms != null ? `${event.duration_ms}ms` : "—"}</td>
													<td className="py-2 text-right">
														<Button variant="ghost" size="sm" onClick={() => toggleExpanded(rowKey)}>
															{t("audit:labels.details", { defaultValue: "Details" })}
														</Button>
													</td>
												</tr>
												{expanded ? (
													<tr className="border-b bg-slate-50 dark:bg-slate-900/50 last:border-0">
														<td colSpan={7} className="p-4">
															<div className="grid gap-2 text-xs text-muted-foreground md:grid-cols-2">
																<div><strong>{t("audit:details.route", { defaultValue: "Route" })}:</strong> {event.route ?? "—"}</div>
																<div><strong>{t("audit:details.mcpMethod", { defaultValue: "MCP Method" })}:</strong> {event.mcp_method ?? "—"}</div>
																<div><strong>{t("audit:details.clientId", { defaultValue: "Client ID" })}:</strong> {event.client_id ?? "—"}</div>
																<div><strong>{t("audit:details.profileId", { defaultValue: "Profile ID" })}:</strong> {event.profile_id ?? "—"}</div>
																<div><strong>{t("audit:details.serverId", { defaultValue: "Server ID" })}:</strong> {event.server_id ?? "—"}</div>
																<div><strong>{t("audit:details.sessionId", { defaultValue: "Session ID" })}:</strong> {event.session_id ?? "—"}</div>
																<div><strong>{t("audit:details.requestId", { defaultValue: "Request ID" })}:</strong> {event.request_id ?? "—"}</div>
																<div><strong>{t("audit:details.protocol", { defaultValue: "Protocol" })}:</strong> {event.protocol_version ?? "—"}</div>
																<div className="md:col-span-2"><strong>{t("audit:details.error", { defaultValue: "Error" })}:</strong> {event.error_message ?? "—"}</div>
																<div className="md:col-span-2">
																	<strong>{t("audit:details.data", { defaultValue: "Data" })}:</strong>
																	<pre className="mt-2 overflow-x-auto rounded-md bg-background p-3 text-[11px] leading-5 text-foreground">{event.data ? JSON.stringify(event.data, null, 2) : "—"}</pre>
																</div>
															</div>
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
						onItemsPerPageChange={setPageSize}
						onPreviousPage={handlePrevPage}
						onFirstPage={handleFirstPage}
						onNextPage={handleNextPage}
						onLastPage={handleLastPage}
						pageSizeOptions={[...PAGE_SIZE_OPTIONS]}
						className="mt-4 shrink-0 border-t border-slate-200 pt-4 dark:border-slate-700"
					/>
				</CardContent>
			</Card>
		</div>
	);
}

function toEventKey(event: AuditEventRecord): string {
	return String(event.id ?? `${event.occurred_at_ms}-${event.action}-${event.target ?? ""}`);
}
