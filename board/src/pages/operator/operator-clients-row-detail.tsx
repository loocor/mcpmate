import { useMutation, useQueryClient } from "@tanstack/react-query";
import React from "react";
import { useTranslation } from "react-i18next";
import { clientsApi } from "../../lib/api";
import { notifyError, notifySuccess } from "../../lib/notify";
import type { ClientInfo } from "../../lib/types";
import {
	OperatorActionChip,
	OperatorAvatarStackToggle,
	OperatorBoardLinkChip,
	OperatorChipVisual,
	OperatorHorizontalStrip,
	OperatorMoreButton,
	OperatorRowDetailFrame,
	OperatorRowDetailMessage,
	partitionAttentionFirst,
	splitClearForDisplay,
	toOperatorChipTitleCase,
} from "./operator-row-detail-shared";

function clientDisplayName(client: ClientInfo): string {
	const raw = client.display_name?.trim() || client.identifier;
	return toOperatorChipTitleCase(raw);
}

function clientInitial(client: ClientInfo): string {
	return clientDisplayName(client).charAt(0).toUpperCase() || "C";
}

export function isOperatorClientAttention(client: ClientInfo): boolean {
	const status = String(client.approval_status || "").toLowerCase();
	return status === "pending" || status === "suspended";
}

function isOperatorClientMissingProfile(client: ClientInfo): boolean {
	return client.custom_profile_missing === true;
}

function isOperatorClientNeedsApprove(client: ClientInfo): boolean {
	const status = String(client.approval_status || "").toLowerCase();
	return status === "pending" || status === "suspended";
}

function clientVisual(client: ClientInfo): OperatorChipVisual {
	const status = String(client.approval_status || "").toLowerCase();
	if (status === "suspended") {
		return "error";
	}
	if (isOperatorClientAttention(client) || isOperatorClientMissingProfile(client)) {
		return "attention";
	}
	if (status === "approved") {
		return "active";
	}
	return "neutral";
}

function ClientAvatar({ client }: { client: ClientInfo }) {
	if (client.logo_url) {
		return (
			<img
				src={client.logo_url}
				alt=""
				className="h-full w-full object-cover"
				draggable={false}
			/>
		);
	}
	return <span aria-hidden>{clientInitial(client)}</span>;
}

function sortClients(clients: ClientInfo[]): ClientInfo[] {
	return [...clients].sort((left, right) =>
		clientDisplayName(left).localeCompare(clientDisplayName(right)),
	);
}

export function OperatorClientsRowDetail({
	clients,
	detailId,
	isError,
	isLoading,
	isTauriShell,
	onOpenClient,
	onOpenClientsBoard,
}: {
	clients: ClientInfo[];
	detailId: string;
	isError: boolean;
	isLoading: boolean;
	isTauriShell: boolean;
	onOpenClient: (identifier: string) => void;
	onOpenClientsBoard: () => void;
}) {
	const { t } = useTranslation();
	const queryClient = useQueryClient();
	const [clearExpanded, setClearExpanded] = React.useState(false);
	const [approvingIdentifier, setApprovingIdentifier] = React.useState<string | null>(
		null,
	);

	const sortedClients = React.useMemo(() => sortClients(clients), [clients]);
	const { attention, clear } = React.useMemo(
		() => partitionAttentionFirst(sortedClients, isOperatorClientAttention),
		[sortedClients],
	);
	const { visible: visibleClear, stacked: stackedClear } = React.useMemo(
		() => splitClearForDisplay(clear, clearExpanded),
		[clear, clearExpanded],
	);
	const expandedStack = clearExpanded ? stackedClear : [];

	React.useEffect(() => {
		setClearExpanded(false);
	}, [clients.length]);

	const approveMutation = useMutation({
		mutationFn: (identifier: string) => clientsApi.approveRecord({ identifier }),
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["operator", "clients"] });
			await queryClient.invalidateQueries({ queryKey: ["clients"] });
			notifySuccess(
				t("operator:detail.clients.approveSuccessTitle", {
					defaultValue: "Client approved",
				}),
				t("operator:detail.clients.approveSuccessMessage", {
					defaultValue: "The client can now access configured profiles.",
				}),
			);
		},
		onError: (error) => {
			notifyError(
				t("operator:detail.clients.approveFailedTitle", {
					defaultValue: "Approval failed",
				}),
				error instanceof Error ? error.message : String(error),
			);
		},
		onSettled: () => {
			setApprovingIdentifier(null);
		},
	});

	const handleApproveClient = React.useCallback(
		(client: ClientInfo) => {
			if (approveMutation.isPending) {
				return;
			}
			setApprovingIdentifier(client.identifier);
			approveMutation.mutate(client.identifier);
		},
		[approveMutation],
	);

	const moreLabel = t("operator:detail.clients.more", { defaultValue: "More..." });
	const openClientsLabel = t("operator:detail.clients.openClients", {
		defaultValue: "Open Clients in Full Board",
	});
	const expandStackLabel = t("operator:detail.clients.expandStack", {
		count: stackedClear.length,
		defaultValue: "{{count}} more",
	});
	const collapseStackLabel = t("operator:detail.clients.collapseStack", {
		defaultValue: "Show less",
	});

	const renderClientChip = (client: ClientInfo) => {
		const name = clientDisplayName(client);
		const visual = clientVisual(client);
		const avatar = <ClientAvatar client={client} />;
		const needsApprove = isOperatorClientNeedsApprove(client);
		const isBusy = approvingIdentifier === client.identifier;

		if (needsApprove) {
			const approveLabel = t("operator:detail.clients.approve", {
				name,
				defaultValue: "Approve {{name}}",
			});

			return (
				<OperatorActionChip
					key={client.identifier}
					ariaLabel={approveLabel}
					avatar={avatar}
					disabled={isBusy}
					displayName={name}
					onAction={() => handleApproveClient(client)}
					visual={visual}
				/>
			);
		}

		const href = `/clients/${encodeURIComponent(client.identifier)}`;
		const openInBoardLabel = t("operator:detail.clients.openClient", {
			name,
			defaultValue: "Open {{name}} in Full Board",
		});

		return (
			<OperatorBoardLinkChip
				key={client.identifier}
				ariaLabel={openInBoardLabel}
				avatar={avatar}
				displayName={name}
				href={href}
				isTauriShell={isTauriShell}
				onOpenBoard={() => onOpenClient(client.identifier)}
				visual={visual}
			/>
		);
	};

	return (
		<OperatorRowDetailFrame detailId={detailId}>
			{isLoading ? (
				<OperatorRowDetailMessage>
					{t("operator:rows.clients.loading", { defaultValue: "Loading clients" })}
				</OperatorRowDetailMessage>
			) : isError ? (
				<OperatorRowDetailMessage tone="error">
					{t("operator:rows.clients.error", { defaultValue: "Clients are unavailable" })}
				</OperatorRowDetailMessage>
			) : sortedClients.length === 0 ? (
				<OperatorRowDetailMessage>
					{t("operator:detail.clients.empty", {
						defaultValue: "Detect local clients in Full Board.",
					})}
				</OperatorRowDetailMessage>
			) : (
				<OperatorHorizontalStrip>
					{attention.map(renderClientChip)}
					{visibleClear.map(renderClientChip)}
					{stackedClear.length > 0 ? (
						<OperatorAvatarStackToggle
							collapseLabel={collapseStackLabel}
							expandLabel={expandStackLabel}
							expanded={clearExpanded}
							items={stackedClear.map((client) => ({
								id: client.identifier,
								avatar: <ClientAvatar client={client} />,
								visual: clientVisual(client),
							}))}
							onToggle={() => setClearExpanded((current) => !current)}
						/>
					) : null}
					{expandedStack.map(renderClientChip)}
					<OperatorMoreButton
						href="/clients"
						isTauriShell={isTauriShell}
						moreLabel={moreLabel}
						onOpenBoard={onOpenClientsBoard}
						openLabel={openClientsLabel}
					/>
				</OperatorHorizontalStrip>
			)}
		</OperatorRowDetailFrame>
	);
}
