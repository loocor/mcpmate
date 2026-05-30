import React from "react";
import { useTranslation } from "react-i18next";
import {
	canIngestFromDataTransfer,
} from "../../lib/server-uni-import-transfer";
import type { ServerSummary } from "../../lib/types";
import {
	OperatorAvatarStackToggle,
	OperatorBoardLinkChip,
	OperatorChipVisual,
	OperatorHorizontalStrip,
	OperatorImportDropChip,
	OperatorRowDetailFrame,
	OperatorRowDetailMessage,
	partitionAttentionFirst,
	splitClearForDisplay,
	toOperatorChipTitleCase,
} from "./operator-row-detail-shared";

function serverDisplayName(server: ServerSummary): string {
	const raw = server.name?.trim() || server.id;
	return toOperatorChipTitleCase(raw);
}

function serverInitial(server: ServerSummary): string {
	return serverDisplayName(server).charAt(0).toUpperCase() || "S";
}

export function isOperatorServerAttention(server: ServerSummary): boolean {
	const status = String(server.status || "").toLowerCase();
	return status === "error" || status === "disconnected" || server.enabled === false;
}

function serverVisual(server: ServerSummary): OperatorChipVisual {
	if (!server.enabled || String(server.status).toLowerCase() === "error") {
		return "error";
	}
	if (isOperatorServerAttention(server)) {
		return "attention";
	}
	if (String(server.status).toLowerCase() === "connected") {
		return "active";
	}
	return "neutral";
}

function sortServers(servers: ServerSummary[]): ServerSummary[] {
	return [...servers].sort((left, right) =>
		serverDisplayName(left).localeCompare(serverDisplayName(right)),
	);
}

function OperatorServerImportActions({
	chipLabel,
	dropLabel,
	dropTip,
	onImportDrop,
}: {
	chipLabel: string;
	dropLabel: string;
	dropTip: string;
	onImportDrop: (dataTransfer: DataTransfer) => void | Promise<void>;
}) {
	const [dragActive, setDragActive] = React.useState(false);

	const handleDragEnter = React.useCallback(
		(event: React.DragEvent<HTMLButtonElement>) => {
			if (!canIngestFromDataTransfer(event.dataTransfer)) return;
			event.preventDefault();
			event.stopPropagation();
			setDragActive(true);
		},
		[],
	);

	const handleDragOver = React.useCallback(
		(event: React.DragEvent<HTMLButtonElement>) => {
			if (!canIngestFromDataTransfer(event.dataTransfer)) return;
			event.preventDefault();
			event.stopPropagation();
			if (event.dataTransfer) {
				event.dataTransfer.dropEffect = "copy";
			}
			if (!dragActive) {
				setDragActive(true);
			}
		},
		[dragActive],
	);

	const handleDragLeave = React.useCallback(
		(event: React.DragEvent<HTMLButtonElement>) => {
			event.preventDefault();
			event.stopPropagation();
			const nextTarget = event.relatedTarget as Node | null;
			if (nextTarget && event.currentTarget.contains(nextTarget)) {
				return;
			}
			setDragActive(false);
		},
		[],
	);

	const handleDrop = React.useCallback(
		async (event: React.DragEvent<HTMLButtonElement>) => {
			event.preventDefault();
			event.stopPropagation();
			setDragActive(false);
			const dataTransfer = event.dataTransfer;
			if (!dataTransfer) {
				return;
			}
			await onImportDrop(dataTransfer);
		},
		[onImportDrop],
	);

	return (
		<OperatorImportDropChip
			chipLabel={chipLabel}
			dragActive={dragActive}
			dropLabel={dropLabel}
			dropTip={dropTip}
			onDragEnter={handleDragEnter}
			onDragLeave={handleDragLeave}
			onDragOver={handleDragOver}
			onDrop={handleDrop}
		/>
	);
}

export function OperatorServersRowDetail({
	detailId,
	isError,
	isLoading,
	isTauriShell,
	onImportDrop,
	onOpenServer,
	servers,
}: {
	detailId: string;
	isError: boolean;
	isLoading: boolean;
	isTauriShell: boolean;
	onImportDrop: (dataTransfer: DataTransfer) => void | Promise<void>;
	onOpenServer: (serverId: string) => void;
	servers: ServerSummary[];
}) {
	const { t } = useTranslation();
	const [clearExpanded, setClearExpanded] = React.useState(false);

	const sortedServers = React.useMemo(() => sortServers(servers), [servers]);
	const { attention, clear } = React.useMemo(
		() => partitionAttentionFirst(sortedServers, isOperatorServerAttention),
		[sortedServers],
	);
	const { visible: visibleClear, stacked: stackedClear } = React.useMemo(
		() => splitClearForDisplay(clear, clearExpanded),
		[clear, clearExpanded],
	);
	const expandedStack = clearExpanded ? stackedClear : [];

	React.useEffect(() => {
		setClearExpanded(false);
	}, [servers.length]);

	const chipLabel = t("operator:detail.servers.drop", { defaultValue: "Drop-in" });
	const dropLabel = t("operator:detail.servers.dropRelease", {
		defaultValue: "Release to import",
	});
	const dropTip = t("operator:detail.servers.dropTip", {
		defaultValue:
			"Drag an MCP server JSON snippet, config file, or URL here.",
	});
	const expandStackLabel = t("operator:detail.servers.expandStack", {
		count: stackedClear.length,
		defaultValue: "{{count}} more",
	});
	const collapseStackLabel = t("operator:detail.servers.collapseStack", {
		defaultValue: "Show less",
	});

	const importActions = (
		<OperatorServerImportActions
			chipLabel={chipLabel}
			dropLabel={dropLabel}
			dropTip={dropTip}
			onImportDrop={onImportDrop}
		/>
	);

	const renderServerChip = (server: ServerSummary) => {
		const name = serverDisplayName(server);
		const visual = serverVisual(server);
		const href = `/servers/${encodeURIComponent(server.id)}`;
		const openLabel = t("operator:detail.servers.openServer", {
			name,
			defaultValue: "Open {{name}} in Full Board",
		});

		return (
			<OperatorBoardLinkChip
				key={server.id}
				ariaLabel={openLabel}
				avatar={<span aria-hidden>{serverInitial(server)}</span>}
				displayName={name}
				href={href}
				isTauriShell={isTauriShell}
				onOpenBoard={() => onOpenServer(server.id)}
				visual={visual}
			/>
		);
	};

	return (
		<OperatorRowDetailFrame detailId={detailId}>
			{isLoading ? (
				<OperatorRowDetailMessage>
					{t("operator:rows.servers.loading", { defaultValue: "Loading servers" })}
				</OperatorRowDetailMessage>
			) : isError ? (
				<OperatorRowDetailMessage tone="error">
					{t("operator:rows.servers.error", { defaultValue: "Servers are unavailable" })}
				</OperatorRowDetailMessage>
			) : sortedServers.length === 0 ? (
				<OperatorHorizontalStrip>{importActions}</OperatorHorizontalStrip>
			) : (
				<OperatorHorizontalStrip>
					{attention.map(renderServerChip)}
					{visibleClear.map(renderServerChip)}
					{stackedClear.length > 0 ? (
						<OperatorAvatarStackToggle
							collapseLabel={collapseStackLabel}
							expandLabel={expandStackLabel}
							expanded={clearExpanded}
							items={stackedClear.map((server) => ({
								id: server.id,
								avatar: <span aria-hidden>{serverInitial(server)}</span>,
								visual: serverVisual(server),
							}))}
							onToggle={() => setClearExpanded((current) => !current)}
						/>
					) : null}
					{expandedStack.map(renderServerChip)}
					{importActions}
				</OperatorHorizontalStrip>
			)}
		</OperatorRowDetailFrame>
	);
}
