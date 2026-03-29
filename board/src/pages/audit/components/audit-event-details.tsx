import { useMemo } from "react";
import type { TFunction } from "i18next";
import { Button } from "../../../components/ui/button";
import { JsonCodeBlock } from "../../../components/json-code-block";
import { Link } from "react-router-dom";
import type { AuditEventRecord } from "../../../lib/types";

const RAW_DATA_INLINE_CHAR_LIMIT = 1200;

function rawDetailsOpener(
	eventId: number | null | undefined,
	onOpenRawDetails?: (id: number) => void,
): (() => void) | undefined {
	if (eventId == null || onOpenRawDetails == null) {
		return undefined;
	}
	return () => onOpenRawDetails(eventId);
}

function isMcpDrawerAction(action: AuditEventRecord["action"]): boolean {
	switch (action) {
		case "tools_call":
		case "resources_read":
		case "prompts_get":
			return true;
		default:
			return false;
	}
}

function getManagementSubLabelKey(action: AuditEventRecord["action"]):
	| "audit_policy"
	| "system"
	| "developer"
	| null {
	switch (action) {
		case "audit_policy_update":
			return "audit_policy";
		case "runtime_install":
		case "runtime_cache_reset":
			return "developer";
		case "core_source_apply":
		case "local_core_service_start":
		case "local_core_service_restart":
		case "local_core_service_stop":
		case "local_core_service_install":
		case "local_core_service_uninstall":
		case "desktop_managed_core_start":
		case "desktop_managed_core_restart":
		case "desktop_managed_core_stop":
			return "system";
		default:
			return null;
	}
}

function isPresent(value: unknown): boolean {
	if (value == null) return false;
	if (typeof value === "string") return value.trim().length > 0;
	if (Array.isArray(value)) return value.length > 0;
	return true;
}

function asRecord(value: unknown): Record<string, unknown> | null {
	if (!value || typeof value !== "object" || Array.isArray(value)) {
		return null;
	}
	return value as Record<string, unknown>;
}

function DetailField(props: {
	label: string;
	value: unknown;
	className?: string;
}) {
	const { label, value, className } = props;
	if (!isPresent(value)) {
		return null;
	}

	return (
		<div className={className}>
			<strong>{label}:</strong> {typeof value === "string" ? value : String(value)}
		</div>
	);
}

function LinkField(props: {
	label: string;
	idValue?: string | null;
	nameValue?: string | null;
	to?: string | null;
	className?: string;
}) {
	const { label, idValue, nameValue, to, className } = props;
	if (!isPresent(idValue) && !isPresent(nameValue)) {
		return null;
	}

	return (
		<div className={className}>
			<strong>{label}:</strong>{" "}
			{to && isPresent(idValue) ? (
				<Link to={to} className="text-primary underline-offset-4 hover:underline">
					{nameValue ?? idValue}
				</Link>
			) : (
				<span>{nameValue ?? idValue}</span>
			)}
			{isPresent(nameValue) && isPresent(idValue) && nameValue !== idValue ? (
				<span className="ml-1 text-muted-foreground">({idValue})</span>
			) : null}
		</div>
	);
}

function AuditEntityLinkFields(props: {
	t: TFunction;
	clientId?: string | null;
	clientName?: string | null;
	profileId?: string | null;
	profileName?: string | null;
	serverId?: string | null;
	serverName?: string | null;
}) {
	const { t, clientId, clientName, profileId, profileName, serverId, serverName } = props;
	return (
		<>
			<LinkField
				label={t("audit:details.clientId", { defaultValue: "Client ID" })}
				idValue={clientId}
				nameValue={clientName}
				to={clientId ? `/clients/${encodeURIComponent(clientId)}` : null}
			/>
			<LinkField
				label={t("audit:details.profileId", { defaultValue: "Profile ID" })}
				idValue={profileId}
				nameValue={profileName}
				to={profileId ? `/profiles/${encodeURIComponent(profileId)}` : null}
			/>
			<LinkField
				label={t("audit:details.serverId", { defaultValue: "Server ID" })}
				idValue={serverId}
				nameValue={serverName}
				to={serverId ? `/servers/${encodeURIComponent(serverId)}` : null}
			/>
		</>
	);
}

function StructuredDataSection(props: { t: TFunction; data: unknown }) {
	const { t, data } = props;
	const record = asRecord(data);
	if (!record) {
		return null;
	}

	const entries = Object.entries(record).filter(
		([key, value]) => isPresent(value) && key !== "components" && !Array.isArray(value),
	);
	if (entries.length === 0) {
		return null;
	}

	return (
		<div className="md:col-span-2">
			<strong>
				{t("audit:details.structuredData", {
					defaultValue: "Structured fields",
				})}
				:
			</strong>
			<div className="mt-2 grid gap-2 md:grid-cols-2">
				{entries.map(([key, value]) => (
					<div key={key}>
						<strong>{key}:</strong>{" "}
						{typeof value === "string" ? (
							value
						) : (
							JSON.stringify(value)
						)}
					</div>
				))}
			</div>
		</div>
	);
}

function RawDataSection(props: {
	t: TFunction;
	data: unknown;
	onOpenDrawer?: () => void;
}) {
	const { t, data, onOpenDrawer } = props;
	const rawText = useMemo(() => {
		if (!isPresent(data)) {
			return "";
		}
		return JSON.stringify(data, null, 2);
	}, [data]);

	if (!isPresent(data)) {
		return null;
	}

	const useDrawer = rawText.length > RAW_DATA_INLINE_CHAR_LIMIT && !!onOpenDrawer;

	return (
		<div className="md:col-span-2">
			{useDrawer ? (
				<>
					<div className="flex items-center justify-between gap-3">
						<strong>{t("audit:details.rawData", { defaultValue: "Raw data" })}:</strong>
						<Button
							variant="ghost"
							size="sm"
							className="h-7 px-2 text-xs text-muted-foreground hover:text-foreground"
							onClick={onOpenDrawer}
						>
							{t("audit:details.openRawDataDrawer", { defaultValue: "More" })}
						</Button>
					</div>
					<p className="mt-2 text-xs text-muted-foreground">
						{t("audit:details.rawDataMovedToDrawer", {
							defaultValue: "Raw data is large, open it in the detail drawer.",
						})}
					</p>
				</>
			) : (
				<details className="mt-2">
					<summary className="flex cursor-pointer items-center gap-2 text-xs text-muted-foreground hover:text-foreground">
						<strong>{t("audit:details.rawData", { defaultValue: "Raw data" })}:</strong>
					</summary>
					<div className="mt-2 overflow-hidden">
						<JsonCodeBlock code={rawText} />
					</div>
				</details>
			)}
		</div>
	);
}

function ManagementDetails(props: {
	event: AuditEventRecord;
	t: TFunction;
	onOpenRawDetails?: (eventId: number) => void;
}) {
	const { event, t, onOpenRawDetails } = props;
	const managementScopeKey = getManagementSubLabelKey(event.action);
	const managementScopeLabel =
		managementScopeKey != null
			? t(`audit:managementSubLabels.${managementScopeKey}`, {
					defaultValue: managementScopeKey,
				})
			: undefined;

	return (
		<div className="grid gap-2 text-xs text-muted-foreground md:grid-cols-2">
			<DetailField label={t("audit:details.actor", { defaultValue: "Actor" })} value={event.actor} />
			<DetailField
				label={t("audit:details.managementScope", { defaultValue: "Scope" })}
				value={managementScopeLabel}
			/>
			<DetailField label={t("audit:details.route", { defaultValue: "Route" })} value={event.route} />
			<DetailField label={t("audit:details.httpMethod", { defaultValue: "HTTP Method" })} value={event.http_method} />
			<DetailField label={t("audit:details.target", { defaultValue: "Target" })} value={event.target} />
			<AuditEntityLinkFields
				t={t}
				clientId={event.client_id}
				clientName={event.client_name}
				profileId={event.profile_id}
				profileName={event.profile_name}
				serverId={event.server_id}
				serverName={event.server_name}
			/>
			<DetailField label={t("audit:details.detail", { defaultValue: "Detail" })} value={event.detail} />
			<DetailField label={t("audit:details.taskId", { defaultValue: "Task ID" })} value={event.task_id} />
			<DetailField label={t("audit:details.relatedTaskId", { defaultValue: "Related Task ID" })} value={event.related_task_id} />
			<DetailField
				label={t("audit:details.error", { defaultValue: "Error" })}
				value={event.error_message}
				className="md:col-span-2"
			/>
			<StructuredDataSection t={t} data={event.data} />
			<RawDataSection
				t={t}
				data={event.data}
				onOpenDrawer={rawDetailsOpener(event.id, onOpenRawDetails)}
			/>
		</div>
	);
}

function McpRequestDetails(props: {
	event: AuditEventRecord;
	t: TFunction;
	onOpenRawDetails?: (eventId: number) => void;
}) {
	const { event, t, onOpenRawDetails } = props;
	const supportsDrawer = isMcpDrawerAction(event.action);
	const payloadEventId = event.id;
	const showOpenPayloadButton =
		payloadEventId != null && onOpenRawDetails != null && supportsDrawer;
	return (
		<div className="grid gap-2 text-xs text-muted-foreground md:grid-cols-2">
			<DetailField label={t("audit:details.mcpMethod", { defaultValue: "MCP Method" })} value={event.mcp_method} />
			<DetailField label={t("audit:details.target", { defaultValue: "Target" })} value={event.target} />
			<AuditEntityLinkFields
				t={t}
				clientId={event.client_id}
				clientName={event.client_name}
				profileId={event.profile_id}
				profileName={event.profile_name}
				serverId={event.server_id}
				serverName={event.server_name}
			/>
			<DetailField label={t("audit:details.sessionId", { defaultValue: "Session ID" })} value={event.session_id} />
			<DetailField label={t("audit:details.requestId", { defaultValue: "Request ID" })} value={event.request_id} />
			<DetailField label={t("audit:details.protocol", { defaultValue: "Protocol" })} value={event.protocol_version} />
			<DetailField label={t("audit:details.direction", { defaultValue: "Direction" })} value={event.direction} />
			<DetailField label={t("audit:details.progressToken", { defaultValue: "Progress Token" })} value={event.progress_token} />
			<DetailField
				label={t("audit:details.errorCode", { defaultValue: "Error Code" })}
				value={event.error_code}
			/>
			<DetailField
				label={t("audit:details.error", { defaultValue: "Error" })}
				value={event.error_message}
				className="md:col-span-2"
			/>
			{showOpenPayloadButton ? (
				<div className="md:col-span-2 flex justify-start">
					<Button
						variant="ghost"
						size="sm"
						className="h-7 px-2 text-xs text-muted-foreground hover:text-foreground"
						onClick={() => {
							if (payloadEventId == null || onOpenRawDetails == null) {
								return;
							}
							onOpenRawDetails(payloadEventId);
						}}
					>
						{t("audit:details.openRequestPayload", {
							defaultValue: "More",
						})}
					</Button>
				</div>
			) : null}
			<StructuredDataSection t={t} data={event.data} />
			<RawDataSection
				t={t}
				data={event.data}
				onOpenDrawer={supportsDrawer ? rawDetailsOpener(event.id, onOpenRawDetails) : undefined}
			/>
		</div>
	);
}

function RestLikeDetails(props: {
	event: AuditEventRecord;
	t: TFunction;
	onOpenRawDetails?: (eventId: number) => void;
}) {
	const { event, t, onOpenRawDetails } = props;
	return (
		<div className="grid gap-2 text-xs text-muted-foreground md:grid-cols-2">
			<DetailField label={t("audit:details.route", { defaultValue: "Route" })} value={event.route} />
			<DetailField label={t("audit:details.httpMethod", { defaultValue: "HTTP Method" })} value={event.http_method} />
			<DetailField label={t("audit:details.target", { defaultValue: "Target" })} value={event.target} />
			<AuditEntityLinkFields
				t={t}
				clientId={event.client_id}
				clientName={event.client_name}
				profileId={event.profile_id}
				profileName={event.profile_name}
				serverId={event.server_id}
				serverName={event.server_name}
			/>
			<DetailField label={t("audit:details.requestId", { defaultValue: "Request ID" })} value={event.request_id} />
			<DetailField label={t("audit:details.detail", { defaultValue: "Detail" })} value={event.detail} />
			<DetailField
				label={t("audit:details.errorCode", { defaultValue: "Error Code" })}
				value={event.error_code}
			/>
			<DetailField
				label={t("audit:details.error", { defaultValue: "Error" })}
				value={event.error_message}
				className="md:col-span-2"
			/>
			<StructuredDataSection t={t} data={event.data} />
			<RawDataSection
				t={t}
				data={event.data}
				onOpenDrawer={rawDetailsOpener(event.id, onOpenRawDetails)}
			/>
		</div>
	);
}

export function AuditEventDetails(props: {
	event: AuditEventRecord;
	t: TFunction;
	onOpenRawDetails?: (eventId: number) => void;
}) {
	const { event, t, onOpenRawDetails } = props;

	switch (event.category) {
		case "management":
			return <ManagementDetails event={event} t={t} onOpenRawDetails={onOpenRawDetails} />;
		case "mcp_request":
			return <McpRequestDetails event={event} t={t} onOpenRawDetails={onOpenRawDetails} />;
		default:
			return <RestLikeDetails event={event} t={t} onOpenRawDetails={onOpenRawDetails} />;
	}
}
