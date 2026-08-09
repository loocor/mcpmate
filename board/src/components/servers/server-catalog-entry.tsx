import { Plug } from "lucide-react";
import { memo, useCallback, useMemo } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";

import { resolveServerOAuthReadiness } from "../../lib/oauth-readiness";
import {
	formatCapabilityLifecycle,
	hasCapabilityAuthenticationFailure,
	type CapabilityLifecycleLabels,
} from "../../lib/capability-lifecycle";
import {
	formatServerEndpoint,
	getServerDisplayName,
} from "../../lib/server-display";
import {
	classifyServerTransport,
	type ClassifiedServerTransport,
} from "../../lib/server-transport";
import type { ServerSummary } from "../../lib/types";
import { EntityCard } from "../entity-card";
import { EntityListItem } from "../entity-list-item";
import {
	resolveElevatedServerWarningLabel,
	resolveServerAuthBadgeDisplay,
	ServerAuthBadge,
	ServerWarningBadge,
} from "../server-auth-badge";
import { StatusBadge } from "../status-badge";
import { Badge } from "../ui/badge";
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

function buildCapabilityStats(
	server: ServerSummary,
	statsLabels: ServerCatalogStatsLabels,
	lifecycleLabels: CapabilityLifecycleLabels,
) {
	return [
		{
			label: statsLabels.tools,
			value: formatCapabilityLifecycle(server.capability, "tools", lifecycleLabels),
		},
		{
			label: statsLabels.prompts,
			value: formatCapabilityLifecycle(server.capability, "prompts", lifecycleLabels),
		},
		{
			label: statsLabels.resources,
			value: formatCapabilityLifecycle(server.capability, "resources", lifecycleLabels),
		},
		{
			label: statsLabels.templates,
			value: formatCapabilityLifecycle(
				server.capability,
				"resourceTemplates",
				lifecycleLabels,
			),
		},
	].filter(
		(
			item,
		): item is {
			label: string;
			value: string;
		} => item.value != null,
	);
}

function resolveConnectionTypeLabel(
	transport: ClassifiedServerTransport,
	unrecognized: boolean,
	t: TFunction<"servers">,
): string {
	if (unrecognized) {
		return t("entity.connectionTags.unknown", { defaultValue: "Unknown" });
	}

	switch (transport) {
		case "stdio":
			return t("entity.connectionTags.stdio", { defaultValue: "STDIO" });
		case "sse":
		case "streamable_http":
			return t("entity.connectionTags.streamableHttp", {
				defaultValue: "Streamable HTTP",
			});
		case "http":
		case "unknown":
			return t("entity.connectionTags.http", { defaultValue: "HTTP" });
	}
}

function ServerCatalogEntryComponent(props: ServerCatalogEntryProps) {
	const { t } = useTranslation("servers");
	const {
		server,
		statsLabels,
		onOpen,
		onToggle,
		isToggleDisabled,
	} = props;
	const displayName = getServerDisplayName(server);
	const requiresTransportRepair =
		server.transport_validity?.draft?.kind === "unrecognized";
	const classifiedTransport = classifyServerTransport(server.server_type);
	const lifecycleLabels: CapabilityLifecycleLabels = {
		unavailable: t("capabilityLifecycle.capabilityUnavailable"),
		unsupported: t("capabilityLifecycle.capabilityUnsupported"),
		unknown: t("capabilityLifecycle.capabilityUnknown"),
		empty: t("capabilityLifecycle.capabilityEmpty"),
		ready: t("capabilityLifecycle.capabilityReady"),
	};

	const serverInitial = (displayName || server.id || "S")
		.slice(0, 1)
		.toUpperCase();
	const iconSrc = server.icons?.[0]?.src;
	const iconAlt = displayName
		? t("entity.iconAlt.named", {
			name: displayName,
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

	const unifyEligibilityTag = server.unify_direct_exposure_eligible ? (
		<Badge
			variant="secondary"
			className="border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-200"
		>
			{t("entity.tags.unifyEligible", { defaultValue: "Unify Direct" })}
		</Badge>
	) : null;

	const connectionTypeTag = (
		<span className="flex items-center gap-1 text-xs" data-decorative>
			<Plug className="h-3 w-3" />
			{resolveConnectionTypeLabel(
				classifiedTransport,
				requiresTransportRepair,
				t,
			)}
		</span>
	);

	const oauthReadiness = resolveServerOAuthReadiness(server);
	const authDisplay = resolveServerAuthBadgeDisplay({
		authMode: server.auth_mode,
		oauthStatus: server.oauth_status,
		readiness: oauthReadiness,
		t,
	});
	const authWarningLabel =
		authDisplay.kind === "warning" ? authDisplay.label : null;
	const primaryWarningLabel = resolveElevatedServerWarningLabel({
		requiresTransportRepair,
		authWarningLabel,
		t,
	});

	const authBadge =
		authDisplay.kind === "none" || authDisplay.kind === "warning" ? null : (
			<ServerAuthBadge
				display={authDisplay}
				showLabel={props.variant === "list"}
			/>
		);

	const statusBadge = (() => {
		if (primaryWarningLabel) {
			return <ServerWarningBadge label={primaryWarningLabel} />;
		}

		const namespaceIssue = server.namespace_issue;
		const namespaceIssueLabel = namespaceIssue
			? t(
				namespaceIssue.code === "capability_collision" ||
					namespaceIssue.conflicts?.length
					? "detail.namespaceIssue.statusConflict"
					: "detail.namespaceIssue.statusInvalid",
			)
			: undefined;

		return (
			<StatusBadge
				status={namespaceIssue ? "pending" : server.status}
				statusLabel={namespaceIssueLabel}
				instances={namespaceIssue ? [] : server.instances}
				blinkOnError={
					!namespaceIssue &&
					["error", "unhealthy", "stopped", "failed"].includes(
						(server.status || "").toLowerCase(),
					)
				}
				isServerEnabled={namespaceIssue ? false : server.enabled}
			/>
		);
	})();

	const stats = hasCapabilityAuthenticationFailure(server.capability)
		? []
		: buildCapabilityStats(server, statsLabels, lifecycleLabels);

	const gridDescription = useMemo(() => {
		const serverTypeRaw = server.server_type || "";
		const fallbackServerLabel = t("entity.description.serverLabel", {
			name: server.name || server.id,
			defaultValue: "Server: {{name}}",
		});

		let technicalLine = fallbackServerLabel;
		if (!requiresTransportRepair) {
			switch (classifiedTransport) {
				case "stdio":
					technicalLine = `stdio://${server.name || server.id}`;
					break;
				case "sse":
				case "streamable_http":
				case "http":
					technicalLine =
						formatServerEndpoint(server.url) ?? fallbackServerLabel;
					break;
				case "unknown":
					break;
			}
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
	}, [
		classifiedTransport,
		requiresTransportRepair,
		server.id,
		server.meta?.description,
		server.name,
		server.server_type,
		server.url,
		t,
	]);

	const listDescription = (
		<div className="flex items-center gap-2">
			{connectionTypeTag}
			{unifyEligibilityTag}
			{authBadge}
		</div>
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
				title={displayName}
				description={listDescription}
				avatar={avatar}
				titleBadges={unifyEligibilityTag ? [unifyEligibilityTag] : []}
				stats={stats}
				statusBadge={statusBadge}
				enableSwitch={{
					checked: server.enabled || false,
					onChange: handleToggle,
					disabled: isToggleDisabled,
				}}
				onClick={handleOpen}
			/>
		);
	}

	return (
		<EntityCard
			id={server.id}
			title={displayName}
			description={gridDescription}
			avatar={avatar}
			stats={stats}
			topRightBadge={
				<div className="flex items-center gap-2">
					{connectionTypeTag}
					{unifyEligibilityTag}
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
