import { ExternalLink, Plug, Download, Code, Globe, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Avatar, AvatarFallback, AvatarImage } from "../../components/ui/avatar";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import {
	Drawer,
	DrawerContent,
	DrawerHeader,
	DrawerTitle,
	DrawerDescription,
	DrawerFooter,
} from "../../components/ui/drawer";
import type { RegistryServerEntry } from "../../lib/types";
import { formatServerName, getRemoteTypeLabel } from "./utils";
import { getOfficialMeta } from "../../lib/registry";
import { formatLocalDateTime } from "../../lib/utils";

interface MarketDetailDrawerProps {
	server: RegistryServerEntry | null;
	isOpen: boolean;
	onClose: () => void;
	onInstall: (server: RegistryServerEntry) => void;
}

export function MarketDetailDrawer({
	server,
	isOpen,
	onClose,
	onInstall,
}: MarketDetailDrawerProps) {
	const { t } = useTranslation("market");
	
	if (!server) return null;

	const official = getOfficialMeta(server);
	const displayName = formatServerName(server.name);
	const publishedLabel = official?.updatedAt ?? official?.publishedAt;
	const absoluteTimestamp = publishedLabel
		? formatLocalDateTime(publishedLabel)
		: null;

	const transportBadges = Array.from(
		new Set(
			[
				...(server.remotes ?? []).map((r) => r.type),
				...(server.packages ?? []).map((p) => p.transport?.type),
			].filter((t): t is string => Boolean(t))
		)
	);

	const primaryIconSrc = server.icons?.[0]?.src;
	const websiteUrl = server.websiteUrl;
	const repoUrl = server.repository?.url;

	return (
		<Drawer open={isOpen} onOpenChange={(v) => !v && onClose()}>
			<DrawerContent className="max-w-2xl mx-auto h-[85vh]">
				<DrawerHeader className="text-left border-b pb-4">
					<div className="flex items-start gap-4">
						<Avatar className="h-16 w-16 bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-200 text-xl font-medium flex-shrink-0">
							{primaryIconSrc ? (
								<AvatarImage src={primaryIconSrc} alt={displayName} />
							) : null}
							<AvatarFallback>
								{displayName.charAt(0).toUpperCase()}
							</AvatarFallback>
						</Avatar>
						<div className="flex-1 space-y-1">
							<DrawerTitle className="text-2xl font-bold">{displayName}</DrawerTitle>
							<DrawerDescription className="text-sm">
								{server.name} • v{server.version}
							</DrawerDescription>
							{absoluteTimestamp && (
								<div className="text-xs text-slate-500">
									{t("detail.updatedAt", { defaultValue: "Updated" })} {absoluteTimestamp}
								</div>
							)}
							{official && (
								<Badge variant="outline" className="mt-2 text-xs font-medium bg-emerald-50 text-emerald-700 border-emerald-200 dark:bg-emerald-950/30 dark:text-emerald-400 dark:border-emerald-800">
									<ShieldCheck className="w-3 h-3 mr-1" />
									{t("detail.officialRegistry", { defaultValue: "Official Registry" })}
								</Badge>
							)}
						</div>
					</div>
				</DrawerHeader>

				<div className="flex-1 overflow-y-auto p-6 space-y-8">
					<div className="space-y-3">
						<h3 className="font-semibold text-sm text-slate-900 dark:text-slate-100">
							{t("detail.description", { defaultValue: "Description" })}
						</h3>
						<p className="text-sm text-slate-600 dark:text-slate-300 leading-relaxed whitespace-pre-wrap">
							{server.description || t("detail.noDescription", { defaultValue: "No description provided." })}
						</p>
					</div>

					<div className="space-y-3">
						<h3 className="font-semibold text-sm text-slate-900 dark:text-slate-100">
							{t("detail.links", { defaultValue: "Links" })}
						</h3>
						<div className="flex flex-wrap gap-3">
							{websiteUrl && (
								<a
									href={websiteUrl}
									target="_blank"
									rel="noopener noreferrer"
									className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline"
								>
									<Globe className="w-4 h-4" />
									{t("detail.website", { defaultValue: "Website" })}
								</a>
							)}
							{repoUrl && (
								<a
									href={repoUrl}
									target="_blank"
									rel="noopener noreferrer"
									className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline"
								>
									<Code className="w-4 h-4" />
									{t("detail.repository", { defaultValue: "Repository" })}
								</a>
							)}
							{!websiteUrl && !repoUrl && (
								<span className="text-sm text-slate-500">
									{t("detail.noLinks", { defaultValue: "No external links available." })}
								</span>
							)}
						</div>
					</div>

					<div className="space-y-3">
						<h3 className="font-semibold text-sm text-slate-900 dark:text-slate-100">
							{t("detail.transports", { defaultValue: "Transports" })}
						</h3>
						<div className="flex flex-wrap gap-2">
							{transportBadges.map((type) => (
								<Badge
									key={type}
									variant="secondary"
									className="rounded-full px-3 py-1 text-xs font-medium"
								>
									<Plug className="mr-1.5 h-3.5 w-3.5" />
									{getRemoteTypeLabel(type)}
								</Badge>
							))}
							{transportBadges.length === 0 && (
								<span className="text-sm text-slate-500">
									{t("detail.noTransports", { defaultValue: "No known transports." })}
								</span>
							)}
						</div>
					</div>
				</div>

				<DrawerFooter className="border-t flex flex-row justify-end gap-3 pt-4">
					<Button variant="outline" onClick={onClose}>
						{t("buttons.close", { defaultValue: "Close" })}
					</Button>
					<Button onClick={() => onInstall(server)}>
						<Download className="w-4 h-4 mr-2" />
						{t("buttons.install", { defaultValue: "Install" })}
					</Button>
				</DrawerFooter>
			</DrawerContent>
		</Drawer>
	);
}
