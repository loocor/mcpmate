import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "../capsule-stripe-list";
import { Badge } from "../ui/badge";
import { secretUsageLabel } from "../../lib/secret-usage-label";
import type { SecretUsage, SecretUsageStatus } from "../../lib/types";

interface SecretUsageListProps {
	usages: SecretUsage[];
	isLoading?: boolean;
	serverNameById?: ReadonlyMap<string, string>;
	onNavigateToServer?: (serverId: string) => void;
}

function resolveServerDisplayName(
	serverId: string,
	serverNameById?: ReadonlyMap<string, string>,
): { title: string; showId: boolean } {
	const resolved = serverNameById?.get(serverId)?.trim();
	const title = resolved && resolved.length > 0 ? resolved : serverId;
	return { title, showId: title !== serverId };
}

function normalizeUsageStatus(usage: SecretUsage): SecretUsageStatus {
	return usage.status === "stale" ? "stale" : "active";
}

function SecretUsageRow({
	usage,
	serverNameById,
	onNavigateToServer,
}: {
	usage: SecretUsage;
	serverNameById?: ReadonlyMap<string, string>;
	onNavigateToServer?: (serverId: string) => void;
}) {
	const { t } = useTranslation("secrets");
	const server = resolveServerDisplayName(usage.server_id, serverNameById);
	const status = normalizeUsageStatus(usage);
	const isStale = status === "stale";
	const canNavigate =
		!isStale &&
		Boolean(onNavigateToServer) &&
		Boolean(serverNameById?.has(usage.server_id));
	const serverTitleClassName = `truncate text-sm font-medium ${isStale ? "text-muted-foreground" : ""
		}`;

	return (
		<CapsuleStripeListItem className="items-start py-3">
			<div className="min-w-0 flex-1">
				<div className="flex items-start justify-between gap-3">
					<div className="min-w-0 flex-1">
						{canNavigate ? (
							<Link
								to={`/servers/${encodeURIComponent(usage.server_id)}`}
								onClick={() => onNavigateToServer?.(usage.server_id)}
								className={`${serverTitleClassName} text-primary underline-offset-4 hover:underline`}
								aria-label={t("usage.actions.openServer", {
									defaultValue: "Open server {{name}}",
									name: server.title,
								})}
							>
								{server.title}
							</Link>
						) : (
							<p className={serverTitleClassName}>{server.title}</p>
						)}
						{server.showId ? (
							<p
								className="mt-0.5 truncate font-mono text-xs text-muted-foreground"
								title={usage.server_id}
							>
								{usage.server_id}
							</p>
						) : null}
					</div>
					<Badge
						variant={isStale ? "outline" : "secondary"}
						className="shrink-0"
					>
						{isStale
							? t("usage.status.stale", { defaultValue: "Stale" })
							: t("usage.status.active", { defaultValue: "Active" })}
					</Badge>
				</div>
				<p className="mt-1 text-sm text-muted-foreground">
					{secretUsageLabel(usage, t)}
				</p>
			</div>
		</CapsuleStripeListItem>
	);
}

function SecretUsageSection({
	title,
	description,
	usages,
	serverNameById,
	onNavigateToServer,
}: {
	title: string;
	description?: string;
	usages: SecretUsage[];
	serverNameById?: ReadonlyMap<string, string>;
	onNavigateToServer?: (serverId: string) => void;
}) {
	if (usages.length === 0) {
		return null;
	}

	return (
		<section className="space-y-3">
			<div className="space-y-1">
				<h3 className="text-sm font-medium">{title}</h3>
				{description ? (
					<p className="text-xs text-muted-foreground">{description}</p>
				) : null}
			</div>
			<CapsuleStripeList>
				{usages.map((usage, index) => (
					<SecretUsageRow
						key={`${usage.server_id}-${usage.status ?? "active"}-${index}`}
						usage={usage}
						serverNameById={serverNameById}
						onNavigateToServer={onNavigateToServer}
					/>
				))}
			</CapsuleStripeList>
		</section>
	);
}

export function SecretUsageList({
	usages,
	isLoading = false,
	serverNameById,
	onNavigateToServer,
}: SecretUsageListProps) {
	const { t } = useTranslation("secrets");

	const { activeUsages, staleUsages } = useMemo(() => {
		const active: SecretUsage[] = [];
		const stale: SecretUsage[] = [];
		for (const usage of usages) {
			if (normalizeUsageStatus(usage) === "stale") {
				stale.push(usage);
			} else {
				active.push(usage);
			}
		}
		return { activeUsages: active, staleUsages: stale };
	}, [usages]);

	if (isLoading) {
		return (
			<p className="py-8 text-center text-sm text-muted-foreground">
				{t("usage.loading", { defaultValue: "Loading usages" })}
			</p>
		);
	}

	if (usages.length === 0) {
		return (
			<div className="rounded-lg border border-dashed p-6 text-center">
				<p className="text-sm font-medium">
					{t("usage.empty", { defaultValue: "No server usage recorded" })}
				</p>
				<p className="mt-1 text-xs text-muted-foreground">
					{t("usage.emptyDescription", {
						defaultValue:
							"This secret has no active or historical server binding.",
					})}
				</p>
			</div>
		);
	}

	return (
		<div className="space-y-6">
			<SecretUsageSection
				title={t("usage.sections.active", {
					defaultValue: "Active bindings",
				})}
				description={t("usage.sections.activeDescription", {
					defaultValue: "Servers that currently reference this secret in runtime config.",
				})}
				usages={activeUsages}
				serverNameById={serverNameById}
				onNavigateToServer={onNavigateToServer}
			/>
			<SecretUsageSection
				title={t("usage.sections.stale", {
					defaultValue: "Historical bindings",
				})}
				description={t("usage.sections.staleDescription", {
					defaultValue:
						"Former references left after a server was deleted or the secret was removed from config.",
				})}
				usages={staleUsages}
				serverNameById={serverNameById}
			/>
		</div>
	);
}
