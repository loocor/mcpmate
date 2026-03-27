import type { AuditEventRecord } from "../lib/types";
import { useState } from "react";
import { Pagination } from "./pagination";
import { Button } from "./ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "./ui/card";
import { Input } from "./ui/input";

interface AuditLogsPanelProps {
	title: string;
	description: string;
	searchPlaceholder: string;
	refreshLabel: string;
	loadingLabel: string;
	emptyLabel: string;
	headers: {
		timestamp: string;
		action: string;
		category: string;
		status: string;
		target: string;
	};
	searchValue: string;
	onSearchChange: (value: string) => void;
	onRefresh: () => void;
	rows: AuditEventRecord[];
	isLoading: boolean;
	isFetching: boolean;
	isPaginationActionLoading: boolean;
	currentPage: number;
	hasPreviousPage: boolean;
	hasNextPage: boolean;
	itemsPerPage: number;
	onItemsPerPageChange: (size: number) => void;
	onPreviousPage: () => void;
	onFirstPage: () => void;
	onNextPage: () => void;
	onLastPage: () => void;
	expandLabel: string;
	collapseLabel: string;
	defaultExpanded?: boolean;
	collapsedPreviewRows?: number;
}

export function AuditLogsPanel({
	title,
	description,
	searchPlaceholder,
	refreshLabel,
	loadingLabel,
	emptyLabel,
	headers,
	searchValue,
	onSearchChange,
	onRefresh,
	rows,
	isLoading,
	isFetching,
	isPaginationActionLoading,
	currentPage,
	hasPreviousPage,
	hasNextPage,
	itemsPerPage,
	onItemsPerPageChange,
	onPreviousPage,
	onFirstPage,
	onNextPage,
	onLastPage,
	expandLabel,
	collapseLabel,
	defaultExpanded = false,
	collapsedPreviewRows = 5,
}: AuditLogsPanelProps) {
	const [expanded, setExpanded] = useState(defaultExpanded);
	const visibleRows = expanded ? rows : rows.slice(0, collapsedPreviewRows);

	return (
		<Card>
			<CardHeader className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
				<div>
					<CardTitle>{title}</CardTitle>
					<CardDescription>{description}</CardDescription>
				</div>
				<div className="flex flex-wrap items-center gap-2">
					<Input
						type="search"
						placeholder={searchPlaceholder}
						value={searchValue}
						onChange={(event) => onSearchChange(event.target.value)}
						className="h-8 w-48"
					/>
					<Button
						size="sm"
						variant="outline"
						onClick={onRefresh}
						disabled={isFetching}
					>
						{refreshLabel}
					</Button>
					{rows.length ? (
						<Button
							size="sm"
							variant="outline"
							onClick={() => setExpanded((prev) => !prev)}
						>
							{expanded ? collapseLabel : expandLabel}
						</Button>
					) : null}
				</div>
			</CardHeader>
			<CardContent className="pt-0">
				{isLoading ? (
					<div className="rounded border border-dashed border-slate-200 p-6 text-center text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
						{loadingLabel}
					</div>
				) : rows.length ? (
					<div className="overflow-x-auto">
						<table className="w-full text-sm">
							<thead>
								<tr className="border-b text-left text-muted-foreground">
									<th className="py-2 pr-4">{headers.timestamp}</th>
									<th className="py-2 pr-4">{headers.action}</th>
									<th className="py-2 pr-4">{headers.category}</th>
									<th className="py-2 pr-4">{headers.status}</th>
									<th className="py-2 pr-4">{headers.target}</th>
								</tr>
							</thead>
							<tbody>
								{visibleRows.map((entry) => (
									<tr
										key={String(entry.id ?? `${entry.occurred_at_ms}-${entry.action}`)}
										className="border-b"
									>
										<td className="py-2 pr-4 whitespace-nowrap">
											{new Date(entry.occurred_at_ms).toLocaleString()}
										</td>
										<td className="py-2 pr-4">{entry.action}</td>
										<td className="py-2 pr-4">{entry.category}</td>
										<td className="py-2 pr-4">{entry.status}</td>
										<td className="py-2 pr-4 max-w-[340px] truncate">
											{entry.target ?? entry.route ?? entry.error_message ?? entry.detail ?? "—"}
										</td>
									</tr>
								))}
							</tbody>
						</table>
					</div>
				) : (
					<div className="rounded border border-dashed border-slate-200 p-6 text-center text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
						{emptyLabel}
					</div>
				)}

				{expanded ? (
					<Pagination
						currentPage={currentPage}
						hasPreviousPage={hasPreviousPage}
						hasNextPage={hasNextPage}
						isLoading={isFetching || isPaginationActionLoading}
						itemsPerPage={itemsPerPage}
						currentPageItemCount={rows.length}
						onItemsPerPageChange={onItemsPerPageChange}
						onPreviousPage={onPreviousPage}
						onFirstPage={onFirstPage}
						onNextPage={onNextPage}
						onLastPage={onLastPage}
						pageSizeOptions={[10, 20, 50, 100]}
						className="mt-4"
					/>
				) : null}
			</CardContent>
		</Card>
	);
}
