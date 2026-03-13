import { ExternalLink, EyeOff, Plug } from "lucide-react";
import { useMemo } from "react";
import { Avatar, AvatarFallback } from "../../components/ui/avatar";
import { Badge } from "../../components/ui/badge";
import {
	Card,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { notifyInfo } from "../../lib/notify";
import { getOfficialMeta } from "../../lib/registry";
import { cn, formatLocalDateTime, truncate } from "../../lib/utils";
import type { MarketCardProps } from "./types";
import {
	formatServerName,
	getRemoteTypeLabel,
	hasPreviewableOption,
} from "./utils";

export function MarketCard({
	server,
	onPreview,
	onHide,
	enableBlacklist,
}: MarketCardProps) {
	const official = getOfficialMeta(server);
	const transportBadges = useMemo(() => {
		const set = new Set(
			(server.remotes ?? [])
				.map((remote) => remote.type)
				.filter((type): type is string => Boolean(type)),
		);
		if (server.packages) {
			for (const pkg of server.packages) {
				if (pkg.transport?.type) {
					set.add(pkg.transport.type);
				}
			}
		}
		return Array.from(set);
	}, [server.packages, server.remotes]);

	const publishedLabel = official?.updatedAt ?? official?.publishedAt;
    const absoluteTimestamp = publishedLabel
        ? formatLocalDateTime(publishedLabel)
        : null;
	const displayName = formatServerName(server.name);

	const supportsPreview = useMemo(() => hasPreviewableOption(server), [server]);

	const handleCardClick = () => {
		if (!supportsPreview) {
			notifyInfo(
				"Preview unavailable",
				"This registry entry does not expose a previewable transport option.",
			);
			return;
		}
		onPreview(server);
	};

	return (
		<Card
			role="button"
			tabIndex={supportsPreview ? 0 : -1}
			onClick={handleCardClick}
			onKeyDown={(event) => {
				if (event.key === "Enter" || event.key === " ") {
					event.preventDefault();
					handleCardClick();
				}
			}}
			className={cn(
				"group flex h-full cursor-pointer flex-col overflow-hidden border border-slate-200 transition-all duration-200 hover:border-primary/40 hover:shadow-xl hover:-translate-y-0.5 dark:border-slate-800",
				supportsPreview ? "cursor-pointer" : "cursor-not-allowed opacity-95",
			)}
		>
			<CardHeader className="p-4">
				<div className="flex items-start gap-3">
					<Avatar className="bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-200 text-sm font-medium flex-shrink-0">
						<AvatarFallback>
							{displayName.charAt(0).toUpperCase()}
						</AvatarFallback>
					</Avatar>

					<div className="flex-1 min-w-0 space-y-1">
						{/* 标题和传输类型标签在同一行 */}
						<div className="flex items-start justify-between gap-3">
							<CardTitle
								className="text-lg font-semibold leading-tight truncate"
								title={displayName}
							>
								{displayName}
							</CardTitle>

							{/* 右上角传输类型标签 */}
							{transportBadges.length > 0 && (
								<div className="flex justify-end items-start flex-shrink-0">
									<div className="flex flex-row-reverse gap-1 flex-nowrap">
										{transportBadges.map((type) => (
											<Badge
												key={type}
												variant="outline"
												className="rounded-full border-primary/40 bg-primary/5 px-2 py-0 text-[11px] font-medium text-primary"
											>
												<Plug className="mr-1 h-3 w-3" />
												{getRemoteTypeLabel(type)}
											</Badge>
										))}
									</div>
								</div>
							)}
						</div>

						{/* 版本和更新时间 - 与标题左对齐 */}
						<div className="flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
							<span>{`Version ${server.version}`}</span>
                        {absoluteTimestamp && (
                            <>
                                <span>•</span>
                                <span>Updated {absoluteTimestamp}</span>
                            </>
                        )}
						</div>

						{/* 描述 - 与标题左对齐 */}
						<div className="h-15 flex items-start">
							<CardDescription className="text-sm text-slate-500 line-clamp-3 leading-5">
								{truncate(server.description, 320) || "N/A"}
							</CardDescription>
						</div>
					</div>
				</div>
			</CardHeader>

			<CardFooter className="flex items-center justify-between gap-2 px-4 pb-4 pt-0 mt-auto">
				<div className="flex items-center gap-3">
					<div className="w-12"></div>
					{enableBlacklist && (
						<div className="flex items-center gap-2">
							<button
								type="button"
								onClick={(event) => {
									event.stopPropagation();
									onHide(server);
								}}
								onKeyDown={(event) => {
									if (event.key === "Enter" || event.key === " ") {
										event.preventDefault();
										event.stopPropagation();
										onHide(server);
									}
								}}
								className="inline-flex items-center justify-center rounded-full border border-transparent bg-transparent h-5 w-5 text-slate-400 transition hover:text-slate-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50 dark:text-slate-500 dark:hover:text-slate-300"
								title="Hide this server"
							>
								<EyeOff className="h-4 w-4" />
								<span className="sr-only">Hide server</span>
							</button>
						</div>
					)}
				</div>
				<div className="flex items-center gap-3">
					<button
						type="button"
						onClick={(event) => {
							event.stopPropagation();
							const target = server.repository?.url ?? server.websiteUrl;
							if (target) {
								window.open(target, "_blank", "noopener,noreferrer");
							} else {
								notifyInfo(
									"Registry entry",
									"No external project URL provided for this server.",
								);
							}
						}}
						onKeyDown={(event) => {
							if (event.key === "Enter" || event.key === " ") {
								event.preventDefault();
								event.stopPropagation();
								const target = server.repository?.url ?? server.websiteUrl;
								if (target) {
									window.open(target, "_blank", "noopener,noreferrer");
								} else {
									notifyInfo(
										"Registry entry",
										"No external project URL provided for this server.",
									);
								}
							}
						}}
						className={cn(
							"inline-flex items-center justify-center rounded-full border border-transparent bg-transparent h-5 w-5 text-primary transition hover:text-primary/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50",
							!server.repository?.url && !server.websiteUrl
								? "cursor-not-allowed opacity-60"
								: "",
						)}
						disabled={!server.repository?.url && !server.websiteUrl}
						title="Open project details"
					>
						<ExternalLink className="h-4 w-4" />
						<span className="sr-only">Open project site</span>
					</button>
				</div>
			</CardFooter>
		</Card>
	);
}
