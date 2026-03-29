import {
	useMemo,
	useRef,
	type MouseEvent,
	type ReactNode,
	type Ref,
} from "react";
import type { TFunction } from "i18next";
import {
	CAPABILITY_DETAILS_CLASS,
	CAPABILITY_SUMMARY_CLASS,
} from "../../../components/capability-disclosure-classes";
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

function getRawDataOnOpenDrawer(
	event: AuditEventRecord,
	onOpenRawDetails?: (eventId: number) => void,
): (() => void) | undefined {
	if (event.category === "mcp_request" && !isMcpDrawerAction(event.action)) {
		return undefined;
	}
	return rawDetailsOpener(event.id, onOpenRawDetails);
}

function getRawDataPresentation(
	data: unknown,
	onOpenDrawer?: () => void,
): { hasRawData: boolean; useDrawer: boolean } {
	if (!isPresent(data)) {
		return { hasRawData: false, useDrawer: false };
	}
	const rawText = JSON.stringify(data, null, 2);
	const useDrawer =
		rawText.length > RAW_DATA_INLINE_CHAR_LIMIT && !!onOpenDrawer;
	return { hasRawData: true, useDrawer };
}

function isInteractivePanelTarget(el: HTMLElement | null): boolean {
	if (!el) return false;
	const selector = "button, a, input, textarea, select, [role=button]";
	let cur: HTMLElement | null = el;
	while (cur && cur !== document.body) {
		if (cur.matches?.(selector)) return true;
		cur = cur.parentElement;
	}
	return false;
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

function RawDataSection(props: {
	t: TFunction;
	data: unknown;
	onOpenDrawer?: () => void;
	rawDetailsRef?: Ref<HTMLDetailsElement>;
}) {
	const { t, data, onOpenDrawer, rawDetailsRef } = props;
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

	const rawLabel = t("audit:details.rawData", { defaultValue: "Raw data" });

	return (
		<div className="md:col-span-2">
			{useDrawer ? (
				<>
					<details
						ref={rawDetailsRef}
						className={CAPABILITY_DETAILS_CLASS}
					>
						<summary
							className={CAPABILITY_SUMMARY_CLASS}
							onClick={(e) => {
								e.preventDefault();
								onOpenDrawer?.();
							}}
						>
							{rawLabel}
						</summary>
					</details>
					<p className="mt-2 text-xs text-slate-500">
						{t("audit:details.rawDataMovedToDrawer", {
							defaultValue: "Raw data is large, open it in the detail drawer.",
						})}
					</p>
				</>
			) : (
				<details ref={rawDetailsRef} className={CAPABILITY_DETAILS_CLASS}>
					<summary className={CAPABILITY_SUMMARY_CLASS}>{rawLabel}</summary>
					<div className="mt-2 space-y-2 overflow-hidden">
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
	rawDetailsRef?: Ref<HTMLDetailsElement>;
}) {
	const { event, t, onOpenRawDetails, rawDetailsRef } = props;
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
			<RawDataSection
				t={t}
				data={event.data}
				onOpenDrawer={rawDetailsOpener(event.id, onOpenRawDetails)}
				rawDetailsRef={rawDetailsRef}
			/>
		</div>
	);
}

function McpRequestDetails(props: {
	event: AuditEventRecord;
	t: TFunction;
	onOpenRawDetails?: (eventId: number) => void;
	rawDetailsRef?: Ref<HTMLDetailsElement>;
}) {
	const { event, t, onOpenRawDetails, rawDetailsRef } = props;
	const supportsDrawer = isMcpDrawerAction(event.action);
	const payloadEventId = event.id;
	const showOpenPayloadButton =
		payloadEventId != null && onOpenRawDetails != null && supportsDrawer;
	const openRequestPayload = showOpenPayloadButton
		? rawDetailsOpener(payloadEventId, onOpenRawDetails)
		: undefined;
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
			{openRequestPayload ? (
				<div className="md:col-span-2">
					<details className={CAPABILITY_DETAILS_CLASS}>
						<summary
							className={CAPABILITY_SUMMARY_CLASS}
							onClick={(e) => {
								e.preventDefault();
								openRequestPayload();
							}}
						>
							{t("audit:details.openRequestPayload", {
								defaultValue: "Request payload",
							})}
						</summary>
					</details>
				</div>
			) : null}
			<RawDataSection
				t={t}
				data={event.data}
				onOpenDrawer={supportsDrawer ? rawDetailsOpener(event.id, onOpenRawDetails) : undefined}
				rawDetailsRef={rawDetailsRef}
			/>
		</div>
	);
}

function RestLikeDetails(props: {
	event: AuditEventRecord;
	t: TFunction;
	onOpenRawDetails?: (eventId: number) => void;
	rawDetailsRef?: Ref<HTMLDetailsElement>;
}) {
	const { event, t, onOpenRawDetails, rawDetailsRef } = props;
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
			<RawDataSection
				t={t}
				data={event.data}
				onOpenDrawer={rawDetailsOpener(event.id, onOpenRawDetails)}
				rawDetailsRef={rawDetailsRef}
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
	const rawDetailsRef = useRef<HTMLDetailsElement>(null);
	const onOpenDrawer = getRawDataOnOpenDrawer(event, onOpenRawDetails);
	const { hasRawData, useDrawer } = getRawDataPresentation(event.data, onOpenDrawer);

	const handleExpandedPanelClick = (e: MouseEvent<HTMLDivElement>) => {
		if (!hasRawData) return;
		const sel =
			typeof window !== "undefined" ? window.getSelection()?.toString() : "";
		if (sel?.trim()) return;
		const target = e.target as HTMLElement;
		if (isInteractivePanelTarget(target)) return;
		if (target.closest("details")) return;

		if (useDrawer) {
			onOpenDrawer?.();
		} else {
			const d = rawDetailsRef.current;
			if (d) {
				d.open = !d.open;
			}
		}
	};

	let body: ReactNode;
	switch (event.category) {
		case "management":
			body = (
				<ManagementDetails
					event={event}
					t={t}
					onOpenRawDetails={onOpenRawDetails}
					rawDetailsRef={rawDetailsRef}
				/>
			);
			break;
		case "mcp_request":
			body = (
				<McpRequestDetails
					event={event}
					t={t}
					onOpenRawDetails={onOpenRawDetails}
					rawDetailsRef={rawDetailsRef}
				/>
			);
			break;
		default:
			body = (
				<RestLikeDetails
					event={event}
					t={t}
					onOpenRawDetails={onOpenRawDetails}
					rawDetailsRef={rawDetailsRef}
				/>
			);
	}

	return (
		<div
			className={hasRawData ? "cursor-pointer" : undefined}
			onClick={handleExpandedPanelClick}
		>
			{body}
		</div>
	);
}
