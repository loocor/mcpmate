import { ShieldCheck } from "lucide-react";
import { memo, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { SecretLifecycleState } from "../../lib/secret-lifecycle";
import type { SecretMetadata } from "../../lib/types";
import { EntityCard } from "../entity-card";
import { EntityListItem } from "../entity-list-item";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";

export interface SecretCatalogDisplay {
	title: string;
	secondary: string | null;
}

export type SecretCatalogStatsLabels = {
	provider: string;
	usage: string;
	history: string;
	version: string;
};

type SecretCatalogEntryBaseProps = {
	secret: SecretMetadata;
	display: SecretCatalogDisplay;
	kindLabel: string;
	lifecycleState: SecretLifecycleState;
	providerLabel: string;
	providerNeedsAttention?: boolean;
	statsLabels: SecretCatalogStatsLabels;
	onOpen: (alias: string) => void;
};

export type SecretCatalogListEntryProps = SecretCatalogEntryBaseProps & {
	variant: "list";
	viewUsageLabel: string;
	onViewUsage: (alias: string) => void;
};

export type SecretCatalogGridEntryProps = SecretCatalogEntryBaseProps & {
	variant: "grid";
};

export type SecretCatalogEntryProps =
	| SecretCatalogListEntryProps
	| SecretCatalogGridEntryProps;

const providerAttentionClassName = "text-amber-700 dark:text-amber-300";

function lifecycleBadgeVariant(
	state: SecretLifecycleState,
): "secondary" | "success" | "warning" | "outline" | "info" {
	switch (state) {
		case "active":
			return "success";
		case "oauth_managed":
			return "info";
		case "unused":
			return "outline";
	}
}

function buildStats(
	secret: SecretMetadata,
	providerLabel: string,
	statsLabels: SecretCatalogStatsLabels,
	providerNeedsAttention: boolean,
) {
	return [
		{
			label: statsLabels.provider,
			value: providerLabel,
			valueTitle: secret.provider_kind,
			labelClassName: providerNeedsAttention
				? providerAttentionClassName
				: undefined,
			valueClassName: providerNeedsAttention
				? providerAttentionClassName
				: undefined,
		},
		{
			label: statsLabels.usage,
			value: secret.used_by_count,
		},
		{
			label: statsLabels.history,
			value: secret.historical_usage_count,
		},
		{
			label: statsLabels.version,
			value: secret.version,
		},
	];
}

function SecretCatalogEntryComponent(props: SecretCatalogEntryProps) {
	const { t } = useTranslation("secrets");
	const {
		secret,
		display,
		kindLabel,
		lifecycleState,
		providerLabel,
		providerNeedsAttention = false,
		statsLabels,
		onOpen,
	} = props;

	const lifecycleLabel = t(`lifecycle.state.${lifecycleState}`, {
		defaultValue: lifecycleState.replace(/_/g, " "),
	});
	const lifecycleDescription = t(`lifecycle.description.${lifecycleState}`, {
		defaultValue: lifecycleState.replace(/_/g, " "),
	});

	const handleOpen = useCallback(() => {
		onOpen(secret.alias);
	}, [onOpen, secret.alias]);

	const onViewUsage =
		props.variant === "list" ? props.onViewUsage : undefined;
	const handleViewUsage = useCallback(() => {
		onViewUsage?.(secret.alias);
	}, [onViewUsage, secret.alias]);

	const description = useMemo(
		() => (
			<div className="min-w-0">
				{display.secondary ? (
					<div className="truncate font-mono text-xs">{display.secondary}</div>
				) : null}
				<div className="truncate font-mono text-xs text-muted-foreground">
					{secret.placeholder}
				</div>
			</div>
		),
		[display.secondary, secret.placeholder],
	);

	const lifecycleBadge = useMemo(
		() => (
			<Badge
				variant={lifecycleBadgeVariant(lifecycleState)}
				title={lifecycleDescription}
			>
				{lifecycleLabel}
			</Badge>
		),
		[lifecycleDescription, lifecycleLabel, lifecycleState],
	);

	const stats = useMemo(
		() => buildStats(secret, providerLabel, statsLabels, providerNeedsAttention),
		[providerLabel, providerNeedsAttention, secret, statsLabels],
	);

	if (props.variant === "list") {
		return (
			<EntityListItem
				id={secret.alias}
				title={display.title}
				description={description}
				avatar={{
					fallback: secret.alias.slice(0, 2).toUpperCase(),
				}}
				titleBadges={[
					<Badge key="kind" variant="secondary">
						{kindLabel}
					</Badge>,
					<span key="lifecycle">{lifecycleBadge}</span>,
				]}
				stats={stats}
				actionButtons={[
					<Button
						key="usage"
						type="button"
						variant="ghost"
						size="sm"
						className="h-9 px-2"
						onClick={handleViewUsage}
						aria-label={props.viewUsageLabel}
					>
						<ShieldCheck className="mr-2 h-4 w-4" />
						{secret.used_by_count}
					</Button>,
				]}
				onClick={handleOpen}
			/>
		);
	}

	return (
		<EntityCard
			id={secret.alias}
			title={display.title}
			description={description}
			avatar={{
				fallback: secret.alias.slice(0, 2).toUpperCase(),
			}}
			avatarShape="rounded"
			topRightBadge={
				<>
					{lifecycleBadge}
					<Badge variant="secondary">{kindLabel}</Badge>
				</>
			}
			stats={stats}
			onClick={handleOpen}
		/>
	);
}

export const SecretCatalogEntry = memo(SecretCatalogEntryComponent);
