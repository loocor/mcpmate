import { Bug, Plug } from "lucide-react";
import { memo, useCallback, useMemo, type MouseEvent } from "react";
import { useTranslation } from "react-i18next";

import { resolveServerOAuthReadiness } from "../../lib/oauth-readiness";
import type { ServerSummary } from "../../lib/types";
import { EntityCard } from "../entity-card";
import { EntityListItem } from "../entity-list-item";
import { ServerAuthBadge } from "../server-auth-badge";
import { StatusBadge } from "../status-badge";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Switch } from "../ui/switch";

export type ServerCatalogStatsLabels = {
	tools: string;
	prompts: string;
	resources: string;
	templates: string;
};

type ServerCatalogEntryBaseProps = {
	server: ServerSummary;
	statsLabels: ServerCatalogStatsLabels;
	onOpen: (serverId: string) => void;
	onToggle: (serverId: string, enabled: boolean) => void;
	isToggleDisabled: boolean;
	enableServerDebug?: boolean;
	onOpenDebug?: (serverId: string) => void;
};

export type ServerCatalogListEntryProps = ServerCatalogEntryBaseProps & {
	variant: "list";
};

export type ServerCatalogGridEntryProps = ServerCatalogEntryBaseProps & {
	variant: "grid";
};

export type ServerCatalogEntryProps =
	| ServerCatalogListEntryProps
	| ServerCatalogGridEntryProps;

function getCapabilitySummary(server: ServerSummary) {
	return server.capability ?? server.capabilities ?? undefined;
}

function buildCapabilityStats(
	server: ServerSummary,
	statsLabels: ServerCatalogStatsLabels,
) {
	const capabilitySummary = getCapabilitySummary(server);
	if (!capabilitySummary) {
		return [
			{ label: statsLabels.tools, value: 0 },
			{ label: statsLabels.prompts, value: 0 },
			{ label: statsLabels.resources, value: 0 },
			{ label: statsLabels.templates, value: 0 },
		];
	}

	return [
		{ label: statsLabels.tools, value: capabilitySummary.tools_count },
		{ label: statsLabels.prompts, value: capabilitySummary.prompts_count },
		{ label: statsLabels.resources, value: capabilitySummary.resources_count },
		{
			label: statsLabels.templates,
			value: capabilitySummary.resource_templates_count,
		},
	];
}

function ServerCatalogEntryComponent(props: ServerCatalogEntryProps) {
	const { t } = useTranslation("servers");
	const {
		server,
		statsLabels,
		onOpen,
		onToggle,
		isToggleDisabled,
		enableServerDebug = false,
		onOpenDebug,
	} = props;

	const serverInitial = (server.name || server.id || "S")
		.slice(0, 1)
		.toUpperCase();
	const iconSrc = server.icons?.[0]?.src;
	const iconAlt = server.name
		? t("entity.iconAlt.named", {
				name: server.name,
				defaultValue: "{{name}} icon",
			})
		: t("entity.iconAlt.fallback", { defaultValue: "Server icon" });

	const handleOpen = useCallback(() => {
		onOpen(server.id);
	}, [onOpen, server.id]);

	const handleToggle = useCallback(
		(checked: boolean) => {
			onToggle(server.id, checked);
		},
		[onToggle, server.id],
	);

	const handleOpenDebug = useCallback(
		(event: MouseEvent) => {
			event.stopPropagation();
			onOpenDebug?.(server.id);
		},
		[onOpenDebug, server.id],
	);

	function renderUnifyEligibilityTag() {
		if (!server.unify_direct_exposure_eligible) return null;
		return (
			<Badge
				variant="secondary"
				className="border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-200"
			>
				{t("entity.tags.unifyEligible", { defaultValue: "Unify Direct" })}
			</Badge>
		);
	}

	const connectionTypeTags = useMemo(() => {
		const tags = [];
		const lower = (server.server_type || "").toLowerCase();
		const isStdio = lower.includes("stdio") || lower.includes("process");
		const isStreamable =
			lower.includes("stream") ||
			lower.includes("sse") ||
			lower.includes("streamable");
		const isGenericHttp = lower.includes("http") || lower.includes("rest");

		if (isStdio) {
			tags.push(
				<span
					key="stdio"
					className="flex items-center gap-1 text-xs"
					data-decorative
				>
					<Plug className="h-3 w-3" />
					{t("entity.connectionTags.stdio", { defaultValue: "STDIO" })}
				</span>,
			);
		} else if (isStreamable) {
			tags.push(
				<span
					key="streamable_http"
					className="flex items-center gap-1 text-xs"
					data-decorative
				>
					<Plug className="h-3 w-3" />
					{t("entity.connectionTags.streamableHttp", {
						defaultValue: "Streamable HTTP",
					})}
				</span>,
			);
		} else if (isGenericHttp) {
			tags.push(
				<span
					key="http"
					className="flex items-center gap-1 text-xs"
					data-decorative
				>
					<Plug className="h-3 w-3" />
					{t("entity.connectionTags.http", { defaultValue: "HTTP" })}
				</span>,
			);
		}

		if (tags.length === 0) {
			tags.push(
				<span
					key="default"
					className="flex items-center gap-1 text-xs"
					data-decorative
				>
					<Plug className="h-3 w-3" />
					{t("entity.connectionTags.http", { defaultValue: "HTTP" })}
				</span>,
			);
		}

		return tags;
	}, [server.server_type, t]);

	const unifyEligibilityDescriptionTag = renderUnifyEligibilityTag();
	const unifyEligibilityTitleTag = renderUnifyEligibilityTag();

	const authBadge = useMemo(
		() => (
			<ServerAuthBadge
				authMode={server.auth_mode}
				oauthStatus={server.oauth_status}
				readiness={resolveServerOAuthReadiness(server)}
				showLabel={props.variant === "list"}
			/>
		),
		[props.variant, server],
	);

	const statusBadge = useMemo(
		() => (
			<StatusBadge
				status={server.status}
				instances={server.instances}
				blinkOnError={["error", "unhealthy", "stopped", "failed"].includes(
					(server.status || "").toLowerCase(),
				)}
				isServerEnabled={server.enabled}
			/>
		),
		[server.enabled, server.instances, server.status],
	);

	const stats = useMemo(
		() => buildCapabilityStats(server, statsLabels),
		[server, statsLabels],
	);

	const gridDescription = useMemo(() => {
		const serverTypeRaw = server.server_type || "";
		const serverType = serverTypeRaw.toLowerCase();

		let technicalLine = "";
		if (serverType.includes("stdio") || serverType.includes("process")) {
			technicalLine = `stdio://${server.name || server.id}`;
		} else if (serverType.includes("http") || serverType.includes("sse")) {
			technicalLine = `http://localhost:3000/${server.id}`;
		} else {
			technicalLine = t("entity.description.serverLabel", {
				name: server.name || server.id,
				defaultValue: "Server: {{name}}",
			});
		}

		const metaDescription = server.meta?.description?.trim();
		const firstLine = metaDescription
			? `${metaDescription}${serverTypeRaw ? ` · ${serverTypeRaw}` : ""}`
			: technicalLine;

		return (
			<div
				className="max-w-[200px] truncate text-sm text-slate-500"
				title={firstLine}
			>
				{firstLine}
			</div>
		);
	}, [server.id, server.meta?.description, server.name, server.server_type, t]);

	const listDescription = useMemo(
		() => (
			<div className="flex items-center gap-2">
				{connectionTypeTags}
				{unifyEligibilityDescriptionTag}
				{authBadge}
			</div>
		),
		[authBadge, connectionTypeTags, unifyEligibilityDescriptionTag],
	);

	const avatar = useMemo(
		() => ({
			src: iconSrc,
			alt: iconSrc ? iconAlt : undefined,
			fallback: serverInitial,
		}),
		[iconAlt, iconSrc, serverInitial],
	);

	if (props.variant === "list") {
		return (
			<EntityListItem
				id={server.id}
				title={server.name}
				description={listDescription}
				avatar={avatar}
				titleBadges={
					unifyEligibilityTitleTag ? [unifyEligibilityTitleTag] : []
				}
				stats={stats}
				statusBadge={statusBadge}
				enableSwitch={{
					checked: server.enabled || false,
					onChange: handleToggle,
					disabled: isToggleDisabled,
				}}
				actionButtons={
					enableServerDebug && onOpenDebug
						? [
								<Button
									key="debug"
									size="sm"
									variant="outline"
									className="p-2"
									onClick={handleOpenDebug}
									title={t("actions.debug.open", {
										defaultValue: "Open inspect view",
									})}
								>
									<Bug className="h-4 w-4" />
								</Button>,
							]
						: []
				}
				onClick={handleOpen}
			/>
		);
	}

	return (
		<EntityCard
			id={server.id}
			title={server.name}
			description={gridDescription}
			avatar={avatar}
			stats={stats}
			topRightBadge={
				<div className="flex items-center gap-2">
					{connectionTypeTags}
					{unifyEligibilityDescriptionTag}
					{authBadge}
				</div>
			}
			bottomLeft={statusBadge}
			bottomRight={
				<Switch
					checked={server.enabled || false}
					onCheckedChange={handleToggle}
					disabled={isToggleDisabled}
					onClick={(event) => event.stopPropagation()}
				/>
			}
			onClick={handleOpen}
		/>
	);
}

export const ServerCatalogEntry = memo(ServerCatalogEntryComponent);
