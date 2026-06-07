import { RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
import { toolbarSearchInputClassName } from "../../components/ui/page-toolbar";
import { cn } from "../../lib/utils";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import type { MarketSearchProps } from "./types";

export function MarketSearch({
	search,
	onSearchChange,
	sort,
	onSortChange,
	isLoading,
	lastSyncedAt,
	onSync,
	isSyncing,
}: MarketSearchProps) {
	const { t } = useTranslation();
	usePageTranslations("market");
	return (
		<div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-end">
			{lastSyncedAt ? (
				<div className="text-xs text-slate-500 mr-2">
					{t("market:search.lastSyncedAt", {
						defaultValue: "Last synced: {{time}}",
						time: new Date(lastSyncedAt).toLocaleString(),
					})}
				</div>
			) : null}
			<div className="relative flex-1 overflow-visible sm:flex-none">
				<Input
					value={search}
					onChange={(event) => onSearchChange(event.target.value)}
					placeholder={t("market:search.placeholder", {
						defaultValue: "Search by server name",
					})}
					className={cn(toolbarSearchInputClassName, "sm:w-64")}
				/>
			</div>
			<Select value={sort} onValueChange={onSortChange}>
				<SelectTrigger className="h-9 w-full sm:w-[160px]">
					<SelectValue
						placeholder={t("market:search.sort", { defaultValue: "Sort" })}
					/>
				</SelectTrigger>
				<SelectContent align="end">
					<SelectItem value="recent">
						{t("market:search.recentlyUpdated", {
							defaultValue: "Recently updated",
						})}
					</SelectItem>
					<SelectItem value="name">
						{t("market:search.alphabetical", { defaultValue: "Alphabetical" })}
					</SelectItem>
				</SelectContent>
			</Select>

			<Button
				type="button"
				variant="outline"
				size="sm"
				className="h-9 w-9 shrink-0 p-0"
				onClick={() => {
					void onSync();
				}}
				disabled={isLoading || isSyncing}
				title={t("market:buttons.refresh", { defaultValue: "Refresh & Sync" })}
			>
				<RefreshCw className={`h-4 w-4 ${isSyncing ? "animate-spin" : ""}`} />
			</Button>
		</div>
	);
}
