import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { serversApi } from "../lib/api";
import { notifySuccess, notifyError } from "../lib/notify";
import { startOAuthAccessFlow } from "../lib/oauth-callback-access";
import type { ServerInstallDraft } from "../hooks/use-server-install-pipeline";
import type {
	MCPServerConfig,
	ServerDetail,
	ServerIcon,
	ServerMetaInfo,
} from "../lib/types";
import { ServerInstallManualForm, type ServerInstallManualFormHandle } from "./server-install";
import { Switch } from "./ui/switch";
import { Label } from "./ui/label";

interface ServerEditDrawerProps {
	server: ServerDetail | null;
	isOpen: boolean;
	onClose: () => void;
	onSubmit: (config: Partial<MCPServerConfig>) => Promise<void> | void;
	onUpdated?: () => void;
}

type UpdateConfig = Partial<MCPServerConfig> & {
	url?: string;
	headers?: Record<string, string>;
};

type ExtendedServerIcon = ServerIcon & {
	url?: string | null;
	href?: string | null;
	mime_type?: string | null;
	size?: string | null;
};

const trim = (value?: string | null): string | undefined => {
	if (value == null) return undefined;
	const next = value.trim();
	return next.length > 0 ? next : undefined;
};

const sanitizeRecord = (
	record?: Record<string, string> | null,
): Record<string, string> | undefined => {
	return sanitizeStringRecord(record, true);
};

const sanitizeParams = (
	record?: Record<string, string> | null,
): Record<string, string> | undefined => {
	return sanitizeStringRecord(record, false);
};

const sanitizeStringRecord = (
	record: Record<string, string> | null | undefined,
	trimValue: boolean,
): Record<string, string> | undefined => {
	if (!record) return undefined;
	const next: Record<string, string> = {};
	for (const [rawKey, rawValue] of Object.entries(record)) {
		const key = rawKey?.trim();
		if (!key) continue;
		const value = rawValue == null ? "" : String(rawValue);
		next[key] = trimValue ? value.trim() : value;
	}
	return Object.keys(next).length ? next : undefined;
};

const buildRepositoryPayload = (
	repository?: ServerMetaInfo["repository"],
): NonNullable<ServerMetaInfo["repository"]> | undefined => {
	if (!repository) return undefined;
	const repoPayload: NonNullable<ServerMetaInfo["repository"]> = {};
	const url = trim(repository.url ?? undefined);
	const source = trim(repository.source ?? undefined);
	const subfolder = trim(repository.subfolder ?? undefined);
	const id = trim(repository.id ?? undefined);
	if (url) repoPayload.url = url;
	if (source) repoPayload.source = source;
	if (subfolder) repoPayload.subfolder = subfolder;
	if (id) repoPayload.id = id;
	return Object.keys(repoPayload).length ? repoPayload : undefined;
};

const mergeRefreshedMeta = (
	currentMeta: ServerInstallDraft["meta"],
	refreshedMeta: ServerMetaInfo,
): ServerInstallDraft["meta"] => {
	return {
		...currentMeta,
		description: refreshedMeta.description ?? currentMeta?.description,
		version: refreshedMeta.version ?? currentMeta?.version,
		websiteUrl: refreshedMeta.websiteUrl ?? currentMeta?.websiteUrl,
		repository: refreshedMeta.repository ?? currentMeta?.repository,
		icons: refreshedMeta.icons ?? currentMeta?.icons,
		_meta: refreshedMeta._meta ?? currentMeta?._meta,
		extras: refreshedMeta.extras ?? currentMeta?.extras,
	};
};

const buildMetaFromServer = (
	server: ServerDetail,
): ServerMetaInfo | undefined => {
	const source = server.meta;
	const meta: ServerMetaInfo = {};
	const description = trim(source?.description ?? undefined);
	const version = trim(source?.version ?? undefined);
	const websiteUrl = trim(source?.websiteUrl ?? undefined);

	if (description) meta.description = description;
	if (version) meta.version = version;
	if (websiteUrl) meta.websiteUrl = websiteUrl;

	const repository = buildRepositoryPayload(source?.repository ?? undefined);
	if (repository) meta.repository = repository;

	if (source?._meta) meta._meta = source._meta;
	if (source?.extras) meta.extras = source.extras;

	const iconSource = source?.icons?.length ? source.icons : server.icons;
	if (iconSource?.length) {
		const normalizedIcons = (iconSource as ExtendedServerIcon[])
			.map((icon) => {
				const src =
					trim(icon.src) ??
					trim(icon.url ?? undefined) ??
					trim(icon.href ?? undefined);
				if (!src) return null;
				const mimeType = trim(icon.mimeType ?? icon.mime_type ?? undefined);
				const sizes = trim(icon.sizes ?? icon.size ?? undefined);
				const payload: ServerIcon = { src };
				if (mimeType) payload.mimeType = mimeType;
				if (sizes) payload.sizes = sizes;
				return payload;
			})
			.filter((icon): icon is ServerIcon => Boolean(icon));
		if (normalizedIcons.length) {
			meta.icons = normalizedIcons;
		}
	}

	return Object.keys(meta).length ? meta : undefined;
};

const inferKind = (serverType?: string | null): ServerInstallDraft["kind"] => {
	const kind = serverType?.toLowerCase() ?? "";
	if (kind.includes("streamable")) return "streamable_http";
	if (kind.includes("http")) return "streamable_http";
	if (kind.includes("sse")) return "streamable_http";
	return "stdio";
};

const parseUrl = (
	raw: string | undefined,
): { url?: string; urlParams?: Record<string, string> } => {
	const trimmed = trim(raw);
	if (!trimmed) return {};
	const [base, query] = trimmed.split("?");
	const params: Record<string, string> = {};
	if (query) {
		const search = new URLSearchParams(query);
		search.forEach((value, key) => {
			params[key] = value;
		});
	}
	return {
		url: base,
		urlParams: Object.keys(params).length ? params : undefined,
	};
};

const buildUrlWithParams = (
	url?: string,
	params?: Record<string, string>,
): string | undefined => {
	const trimmedUrl = trim(url);
	if (!trimmedUrl) return undefined;
	const sanitized = sanitizeParams(params);
	if (!sanitized) return trimmedUrl;
	const search = new URLSearchParams();
	for (const [key, value] of Object.entries(sanitized)) {
		search.append(key, value);
	}
	const query = search.toString();
	return query ? `${trimmedUrl}?${query}` : trimmedUrl;
};

const convertServerDetailToDraft = (
	server: ServerDetail,
): ServerInstallDraft => {
	const kind = inferKind(server.server_type);
	const args = Array.isArray(server.args)
		? server.args.filter((item): item is string => Boolean(item))
		: undefined;
	const meta = buildMetaFromServer(server);
	const registryServerId = server.registry_server_id ?? undefined;

	const headersSource = server.headers ?? server.env ?? undefined;
	const sanitizedHeaders = sanitizeRecord(headersSource ?? undefined);

	if (kind === "stdio") {
		return {
			name: server.name,
			kind,
			command: trim(server.command ?? undefined),
			args,
			env: sanitizeRecord(server.env ?? undefined),
			meta,
			registryServerId,
		};
	}

	const rawEndpoint =
		server.url ??
		(typeof server.command === "string" ? server.command : undefined);
	const { url, urlParams } = parseUrl(rawEndpoint ?? undefined);

	return {
		name: server.name,
		kind,
		url,
		urlParams,
		headers: sanitizedHeaders,
		meta,
		registryServerId,
	};
};

const buildMetaPayload = (
	meta: ServerInstallDraft["meta"],
): ServerMetaInfo | undefined => {
	if (!meta) return undefined;
	const payload: ServerMetaInfo = {};
	const description = trim(meta.description ?? undefined);
	const version = trim(meta.version ?? undefined);
	const websiteUrl = trim(meta.websiteUrl ?? undefined);
	if (description) payload.description = description;
	if (version) payload.version = version;
	if (websiteUrl) payload.websiteUrl = websiteUrl;

	const repository = buildRepositoryPayload(meta.repository);
	if (repository) payload.repository = repository;

	if (meta._meta) payload._meta = meta._meta;
	if (meta.extras) payload.extras = meta.extras;
	if (meta.icons?.length) payload.icons = meta.icons;

	return Object.keys(payload).length ? payload : undefined;
};

const draftToUpdateConfig = (draft: ServerInstallDraft): UpdateConfig => {
	const args = draft.args
		?.map((value) => value.trim())
		.filter((value) => value.length > 0);
	const meta = draft.meta ? buildMetaPayload(draft.meta) : undefined;

	if (draft.kind === "stdio") {
		return {
			kind: draft.kind,
			command: trim(draft.command ?? undefined),
			args,
			env: sanitizeRecord(draft.env),
			meta,
		};
	}

	return {
		kind: draft.kind,
		url: buildUrlWithParams(draft.url, draft.urlParams),
		headers: sanitizeRecord(draft.headers),
		args,
		meta,
	};
};

export function ServerEditDrawer({
	server,
	isOpen,
	onClose,
	onSubmit,
	onUpdated,
}: ServerEditDrawerProps) {
	const { t } = useTranslation("servers");
	const formRef = useRef<ServerInstallManualFormHandle>(null);
	const [isRefreshing, setIsRefreshing] = useState(false);
	const [unifyEligible, setUnifyEligible] = useState(false);

	useEffect(() => {
		if (isOpen && server) {
			setUnifyEligible(server.unify_direct_exposure_eligible ?? false);
		}
	}, [isOpen, server]);

	const initialDraft = useMemo(
		() => (server ? convertServerDetailToDraft(server) : null),
		[server],
	);

	const handleSubmit = useCallback(
		async (draft: ServerInstallDraft) => {
			if (!server) return;
			const payload = draftToUpdateConfig(draft);
			await onSubmit({
				...payload,
				unify_direct_exposure_eligible: unifyEligible,
			});
			onUpdated?.();
		},
		[onSubmit, onUpdated, server, unifyEligible],
	);

	const handleInitiateOAuth = useCallback(
		async (config: import("../lib/types").OAuthConfigRequest) => {
			if (!server?.id) return;

			try {
				await startOAuthAccessFlow(server.id, config);
			} catch (error) {
				console.error("Failed to initiate OAuth:", error);
				throw error;
			}
		},
		[server]
	);

	const handleRefreshFromRegistry = useCallback(async () => {
		if (!server?.registry_server_id || !server.id) return;
		try {
			setIsRefreshing(true);
			const currentDraft = formRef.current?.getCurrentDraft();
			if (!currentDraft) return;

			const refreshed = await serversApi.refreshRegistryMetadata(server.id);
			const refreshedMeta = refreshed.meta;
			if (!refreshedMeta) {
				notifyError(
					t("manual.refresh.notFound", { defaultValue: "Server not found in registry" }),
				);
				return;
			}

			const nextDraft: ServerInstallDraft = {
				...currentDraft,
				meta: mergeRefreshedMeta(currentDraft.meta, refreshedMeta),
			};

			formRef.current?.loadDraft(nextDraft);
			notifySuccess(t("manual.refresh.success", { defaultValue: "Metadata refreshed from registry" }));
		} catch (error) {
			notifyError(t("manual.refresh.error", { defaultValue: "Failed to refresh metadata" }), String(error));
		} finally {
			setIsRefreshing(false);
		}
	}, [server, t]);


	const unifyTabContent = (
		<div className="space-y-6">
			<div className="rounded-lg border border-warning/20 bg-warning/5 p-4 space-y-4">
				<div>
					<p className="text-xs font-semibold uppercase tracking-wide text-warning-foreground/80">
						{t("manual.fields.unifyEligibility.badge", {
							defaultValue: "Advanced exposure control",
						})}
					</p>
					<p className="mt-2 text-sm text-muted-foreground">
						{t("manual.fields.unifyEligibility.description", {
							defaultValue:
								"This option marks the server as eligible for direct exposure in Unify mode. Eligible servers can expose tools, prompts, resources, and templates directly to selected clients.",
						})}
					</p>
				</div>

				<h4 className="text-sm font-semibold text-warning-foreground mb-2">
					{t("manual.fields.unifyEligibility.whatIsIt", { defaultValue: "What is this option?" })}
				</h4>
				<p className="text-sm text-muted-foreground mb-4">
					{t("manual.fields.unifyEligibility.whatIsItDesc", { defaultValue: "Enable this only when the server should be available for direct capability exposure in Unify instead of being reached only through the UCAN broker workflow." })}
				</p>

				<h4 className="text-sm font-semibold text-warning-foreground mb-2">
					{t("manual.fields.unifyEligibility.whenToUse", { defaultValue: "When to use it" })}
				</h4>
				<p className="text-sm text-muted-foreground mb-4">
					{t("manual.fields.unifyEligibility.whenToUseDesc", { defaultValue: "Use this for servers that should allow direct exposure of key capabilities to selected Unify clients, such as memory, audit, or always-on context services." })}
				</p>

				<h4 className="text-sm font-semibold text-warning-foreground mb-2">
					{t("manual.fields.unifyEligibility.watchOut", { defaultValue: "What to watch out for" })}
				</h4>
				<p className="text-sm text-muted-foreground mb-4">
					{t("manual.fields.unifyEligibility.watchOutDesc", { defaultValue: "Do not enable this casually. Once a client selects direct exposure, capabilities from this server can bypass the UCAN-only path and enter the direct client context." })}
				</p>

				<h4 className="text-sm font-semibold text-warning-foreground mb-2">
					{t("manual.fields.unifyEligibility.howToEnable", {
						defaultValue: "How to enable it",
					})}
				</h4>
				<p className="text-sm text-muted-foreground">
					{t("manual.fields.unifyEligibility.howToEnableDesc", {
						defaultValue:
						"First mark the server as eligible here. Then open a Client in Unify mode and choose Server Level (all capabilities) or Capability Level (selected tools/prompts/resources/templates).",
					})}
				</p>
			</div>

			<div className="flex items-center justify-between gap-4 p-4 border rounded-lg">
				<div className="flex-1 space-y-1">
					<Label htmlFor="unify-eligible-switch" className="text-sm font-medium leading-none">
						{t("manual.fields.unifyEligibility.title", { defaultValue: "Mark as Unify-eligible server" })}
					</Label>
					<p className="text-sm text-muted-foreground mt-1.5">
						{t("manual.fields.unifyEligibility.toggleHint", { defaultValue: "This only marks eligibility. Clients still decide whether and how to expose it." })}
					</p>
				</div>
				<Switch
					id="unify-eligible-switch"
					checked={unifyEligible}
					onCheckedChange={setUnifyEligible}
				/>
			</div>
		</div>
	);

	return (
		<ServerInstallManualForm
			ref={formRef}
			isOpen={isOpen}
			onClose={onClose}
			onSubmit={handleSubmit}
			mode="edit"
			serverId={server?.id}
			onInitiateOAuth={handleInitiateOAuth}
			allowJsonEditing={false}
			initialDraft={initialDraft ?? undefined}
			onRefreshFromRegistry={server?.registry_server_id ? handleRefreshFromRegistry : undefined}
			isRefreshingRegistry={isRefreshing}
			extraTab={{
				value: "unify",
				label: t("manual.tabs.unify", { defaultValue: "Direct Exposure" }),
				content: unifyTabContent,
			}}
		/>
	);

}
