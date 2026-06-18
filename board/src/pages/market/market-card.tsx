import { ExternalLink, EyeOff, Plug, Download, ShieldCheck } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { MouseEvent, KeyboardEvent } from "react";
import { Avatar, AvatarFallback } from "../../components/ui/avatar";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
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
	hasUnsupportedRegistryPackageOption,
	hasPreviewableOption,
} from "./utils";

export const MARKET_CARD_GRID_CLASS = "grid gap-4 md:grid-cols-2 xl:grid-cols-3";

const MARKET_CARD_SHELL_CLASS =
	"group flex h-full flex-col overflow-hidden border border-slate-200 dark:border-slate-700";

const MARKET_CARD_SHELL_INTERACTIVE_CLASS =
	"cursor-pointer transition-all duration-200 hover:border-primary/40 hover:shadow-xl hover:-translate-y-0.5";

const MARKET_CARD_DESCRIPTION_MIN_HEIGHT_CLASS = "min-h-[3.75rem]";

const SKELETON_BLOCK_CLASS = "rounded bg-secondary animate-pulse dark:bg-muted";

export { SKELETON_BLOCK_CLASS };

export function MarketCardSkeleton({ enableBlacklist = false }: { enableBlacklist?: boolean }) {
	return (
		<Card className={MARKET_CARD_SHELL_CLASS} aria-hidden="true">
			<CardHeader className="p-4">
				<div className="flex items-start gap-3">
					<div className={cn("h-12 w-12 shrink-0 rounded-[10px]", SKELETON_BLOCK_CLASS)} />
					<div className="min-w-0 flex-1 space-y-1">
						<div className={cn("h-12 w-full rounded-lg", SKELETON_BLOCK_CLASS)} />
						<div
							className={cn(
								MARKET_CARD_DESCRIPTION_MIN_HEIGHT_CLASS,
								"w-full rounded-lg",
								SKELETON_BLOCK_CLASS,
							)}
						/>
					</div>
				</div>
			</CardHeader>
			<CardFooter className="mt-auto flex items-center justify-between gap-2 px-4 pb-4 pt-0">
				<div className="flex items-center gap-3">
					<div className="w-12" />
					{enableBlacklist ? <div className="h-5 w-5 shrink-0" aria-hidden="true" /> : null}
				</div>
				<div className={cn("h-7 w-28 rounded-md", SKELETON_BLOCK_CLASS)} />
			</CardFooter>
		</Card>
	);
}

export function MarketCard({
	server,
	isInstalled,
	onPreview,
	onInstall,
	onHide,
	enableBlacklist,
}: MarketCardProps) {
	const { t } = useTranslation("market");
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
	const hasUnsupportedPackageOption = useMemo(
		() => hasUnsupportedRegistryPackageOption(server),
		[server],
	);
	const canOpenDetail = supportsPreview || hasUnsupportedPackageOption;
	const installDisabled = isInstalled || !supportsPreview;

	const handleCardClick = () => {
		if (!canOpenDetail) {
			notifyInfo(
				t("notifications.previewUnavailable", { defaultValue: "Preview unavailable" }),
				t("notifications.noPreviewableTransport", { defaultValue: "This registry entry does not expose a previewable transport option." }),
			);
			return;
		}
		onPreview(server);
	};

	const handleInstall = (e: MouseEvent | KeyboardEvent) => {
		e.stopPropagation();
		e.preventDefault();
		onInstall(server);
	};

	return (
		<Card
			role="button"
			tabIndex={canOpenDetail ? 0 : -1}
			onClick={handleCardClick}
			onKeyDown={(event) => {
				if (event.key === "Enter" || event.key === " ") {
					event.preventDefault();
					handleCardClick();
				}
			}}
			className={cn(
				MARKET_CARD_SHELL_CLASS,
				MARKET_CARD_SHELL_INTERACTIVE_CLASS,
				canOpenDetail ? "cursor-pointer" : "cursor-not-allowed opacity-95",
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
						<div className="flex min-w-0 items-start justify-between gap-3">
							<CardTitle
								className="min-w-0 flex-1 truncate text-lg font-semibold leading-tight"
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
						<div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-xs text-slate-500 dark:text-slate-400">
							<span className="break-words">
								{t("server.version", { defaultValue: "Version {{version}}", version: server.version })}
							</span>
							{absoluteTimestamp && (
								<>
									<span>•</span>
									<span>{t("server.updated", { defaultValue: "Updated {{time}}", time: absoluteTimestamp })}</span>
								</>
							)}
						</div>

						{/* 描述 - 与标题左对齐 */}
						<div className={cn(MARKET_CARD_DESCRIPTION_MIN_HEIGHT_CLASS, "flex items-start")}>
							<CardDescription className="line-clamp-3 text-sm leading-5 text-slate-500">
								{truncate(server.description, 320) || t("server.na", { defaultValue: "N/A" })}
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
				<div className="flex items-center gap-2">
					<Button
						variant={isInstalled ? "secondary" : "outline"}
						size="sm"
						className={cn(
							"h-7 px-2 text-xs",
							isInstalled && "bg-green-50 text-green-700 hover:bg-green-50 hover:text-green-700 border-transparent dark:bg-green-950/30 dark:text-green-400 dark:hover:bg-green-950/30 cursor-default opacity-100"
						)}
						onClick={installDisabled ? (e) => { e.stopPropagation(); e.preventDefault(); } : handleInstall}
						disabled={installDisabled}
					>
						{isInstalled ? (
							<ShieldCheck className="h-3 w-3 mr-1" />
						) : (
							<Download className="h-3 w-3 mr-1" />
						)}
						{isInstalled
							? t("buttons.installed", { defaultValue: "Installed" })
							: !supportsPreview && hasUnsupportedPackageOption
								? t("buttons.unsupported", { defaultValue: "Unsupported" })
								: t("buttons.install", { defaultValue: "Install" })}
					</Button>
					<button
						type="button"
						onClick={(event) => {
							event.stopPropagation();
							const target = server.repository?.url ?? server.websiteUrl;
							if (target) {
								window.open(target, "_blank", "noopener,noreferrer");
							} else {
								notifyInfo(
									t("notifications.registryEntry", { defaultValue: "Registry entry" }),
									t("notifications.noProjectUrl", { defaultValue: "No external project URL provided for this server." }),
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
										t("notifications.registryEntry", { defaultValue: "Registry entry" }),
										t("notifications.noProjectUrl", { defaultValue: "No external project URL provided for this server." }),
									);
								}
							}
						}}
						className={cn(
							"inline-flex items-center justify-center rounded-full border border-transparent bg-transparent h-7 w-7 text-primary transition hover:text-primary/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50",
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
