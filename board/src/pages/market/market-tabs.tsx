import { Plus, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { Button } from "../../components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "../../components/ui/dropdown-menu";
import type { MarketTabsProps } from "./types";

export function MarketTabs({
	tabs,
	activeTab,
	onTabChange,
	onCloseTab,
	onAddOfficial,
	availablePortals,
	onAddPortal,
}: MarketTabsProps) {
	const { t } = useTranslation();
	usePageTranslations("market");
	const activePortalIds = new Set(
		tabs
			.filter((tab) => tab.type === "third-party" && tab.portalId)
			.map((tab) => tab.portalId as string),
	);

	return (
		<div className="mt-4">
			<div className="flex items-center border-b border-slate-200 dark:border-slate-700">
				{/* Tab triggers */}
				<div className="flex items-center flex-1 overflow-x-auto">
					{tabs.map((tab, index) => (
						<div
							key={tab.id}
							className={`group relative flex items-center ${index === 0 ? "pl-0" : "pl-2"}`}
						>
							<button
								type="button"
								onClick={() => onTabChange(tab.id)}
								className={`flex items-center gap-2 py-2 pr-6 text-sm font-medium border-b-2 transition-colors ${
									index === 0 ? "pl-0" : "pl-1"
								} ${
									activeTab === tab.id
										? "border-primary text-primary"
										: "border-transparent text-slate-500 hover:text-slate-700 hover:border-slate-300 dark:text-slate-400 dark:hover:text-slate-300"
								}`}
							>
								{tab.icon ? (
									<img
										src={tab.icon}
										alt=""
										aria-hidden="true"
										className="h-4 w-4 shrink-0 rounded border border-slate-200 object-cover dark:border-slate-700"
										loading="lazy"
										draggable={false}
									/>
								) : null}
								<span className="truncate text-left">{tab.label}</span>
							</button>
							{tab.closable ? (
								<button
									type="button"
									onClick={(e) => {
										e.stopPropagation();
										onCloseTab(tab.id);
									}}
									className="absolute right-0 top-1/2 -translate-y-1/2 rounded p-1 opacity-0 transition-opacity hover:bg-slate-100 focus:opacity-100 focus:outline-none group-hover:opacity-100 dark:hover:bg-slate-800"
									aria-label={`Close ${tab.label}`}
								>
									<X className="h-3 w-3" />
								</button>
							) : null}
						</div>
					))}
				</div>

				{/* Add tab button (right-aligned) */}
				<div className="ml-auto flex-shrink-0 pl-2">
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button variant="ghost" size="sm" className="h-8 w-8 p-0">
								<Plus className="h-4 w-4" />
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent align="end">
							<DropdownMenuItem
								onClick={onAddOfficial}
								className="flex items-center gap-2"
							>
								<span className="flex h-5 w-5 items-center justify-center">
									<img
										src="https://mcpmate.io/logo.svg"
										alt=""
										className="h-4 w-4 rounded border border-slate-200 object-contain dark:border-slate-700 dark:invert"
										loading="lazy"
										draggable={false}
										aria-hidden="true"
									/>
								</span>
								{t("market:officialRegistry", {
									defaultValue: "Official MCP Registry",
								})}
							</DropdownMenuItem>
							{availablePortals.map((portal) => (
								<DropdownMenuItem
									key={portal.id}
									onClick={() => onAddPortal(portal.id)}
									disabled={activePortalIds.has(portal.id)}
									className="flex items-center gap-2"
								>
									{portal.proxyFavicon || portal.favicon ? (
										<span className="flex h-5 w-5 items-center justify-center">
											<img
												src={portal.proxyFavicon ?? portal.favicon ?? ""}
												alt=""
												className="h-4 w-4 rounded border border-slate-200 object-cover dark:border-slate-700"
												loading="lazy"
												draggable={false}
												aria-hidden="true"
											/>
										</span>
									) : (
										<span
											className="flex h-5 w-5 items-center justify-center"
											aria-hidden="true"
										>
											<span className="h-2.5 w-2.5 rounded-full bg-slate-300 dark:bg-slate-600" />
										</span>
									)}
									{portal.label}
								</DropdownMenuItem>
							))}
						</DropdownMenuContent>
					</DropdownMenu>
				</div>
			</div>
		</div>
	);
}
