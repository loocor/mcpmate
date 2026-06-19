import { RefreshCw } from "lucide-react";
import {
	useEffect,
	useRef,
	useState,
	type ChangeEvent,
	type CompositionEvent,
} from "react";
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
	onSync,
	isSyncing,
}: MarketSearchProps) {
	const { t } = useTranslation();
	usePageTranslations("market");
	const [inputValue, setInputValue] = useState(search);
	const isComposingRef = useRef(false);

	useEffect(() => {
		if (!isComposingRef.current) {
			setInputValue(search);
		}
	}, [search]);

	function handleSearchInputChange(event: ChangeEvent<HTMLInputElement>) {
		const nextValue = event.target.value;
		setInputValue(nextValue);
		if (!isComposingRef.current) {
			onSearchChange(nextValue);
		}
	}

	function handleSearchCompositionStart() {
		isComposingRef.current = true;
	}

	function handleSearchCompositionEnd(
		event: CompositionEvent<HTMLInputElement>,
	) {
		isComposingRef.current = false;
		const nextValue = event.currentTarget.value;
		setInputValue(nextValue);
		onSearchChange(nextValue);
	}

	return (
		<div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-end">
			<div className="relative flex-1 overflow-visible sm:flex-none">
				<Input
					value={inputValue}
					onChange={handleSearchInputChange}
					onCompositionStart={handleSearchCompositionStart}
					onCompositionEnd={handleSearchCompositionEnd}
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
