import { RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";
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
	onRefresh,
	isLoading,
}: MarketSearchProps) {
	const { t } = useTranslation();
	usePageTranslations("market");
	return (
		<div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-end">
			<div className="flex-1 sm:flex-none">
				<Input
					value={search}
					onChange={(event) => onSearchChange(event.target.value)}
					placeholder={t("market:search.placeholder", {
						defaultValue: "Search by server name",
					})}
					className="h-9 w-full rounded-md border border-slate-200 bg-white px-4 py-2 text-sm placeholder:text-slate-500 focus:outline-none focus:ring-2 focus:ring-slate-300 dark:border-slate-700 dark:bg-slate-900 dark:placeholder:text-slate-400 dark:focus:ring-slate-600 sm:w-64"
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
				onClick={onRefresh}
				disabled={isLoading}
				title={t("market:buttons.refresh", { defaultValue: "Refresh" })}
			>
				<RefreshCw className={`h-4 w-4 ${isLoading ? "animate-spin" : ""}`} />
			</Button>
		</div>
	);
}
