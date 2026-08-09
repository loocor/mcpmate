import { useMutation } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	buildPreviewPayload,
	type ServerInstallDraft,
} from "../hooks/use-server-install-pipeline";
import {
	completeServerImportForProfile,
	extractImportStats,
	serversApi,
} from "../lib/api";
import { resolveAutoAddTargetProfileId } from "../lib/default-profile";
import { notifyError, notifyInfo, notifySuccess } from "../lib/notify";
import { profileSyncErrorTranslationKey } from "../lib/profile-sync-error";
import { formatNameList, summarizeSkipped } from "../lib/server-import-utils";
import {
	resolveImportDrawerOpen,
	shouldAcceptImportDrawerChange,
} from "../lib/import-drawer-lifecycle";
import { useAppStore } from "../lib/store";
import { Button } from "./ui/button";
import {
	Drawer,
	DrawerContent,
	DrawerDescription,
	DrawerFooter,
	DrawerHeader,
	DrawerTitle,
} from "./ui/drawer";
import { Textarea } from "./ui/textarea";

interface PreviewCapabilitySummary {
	items?: unknown[];
}

interface PreviewItem {
	name?: string;
	ok?: boolean;
	error?: unknown;
	tools?: PreviewCapabilitySummary;
	resources?: PreviewCapabilitySummary;
	resource_templates?: PreviewCapabilitySummary;
	prompts?: PreviewCapabilitySummary;
	[key: string]: unknown;
}

interface PreviewResponseData {
	items: PreviewItem[];
}

interface PreviewResult {
	success: boolean;
	data?: PreviewResponseData | null;
	error?: unknown | null;
}

type PreviewPayload = ReturnType<typeof buildPreviewPayload>;

interface ImportPayload {
	mcpServers: Record<string, unknown>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

function parseStringArray(value: unknown, field: string): string[] | undefined {
	if (value === undefined || value === null) return undefined;
	if (!Array.isArray(value) || !value.every((item) => typeof item === "string")) {
		throw new Error(`${field} must be an array of strings`);
	}
	return value;
}

function parseStringRecord(
	value: unknown,
	field: string,
): Record<string, string> | undefined {
	if (value === undefined || value === null) return undefined;
	if (!isRecord(value) || !Object.values(value).every((item) => typeof item === "string")) {
		throw new Error(`${field} must be an object with string values`);
	}
	return value as Record<string, string>;
}

function parsePreviewDraft(
	name: string,
	definition: unknown,
): ServerInstallDraft {
	if (!isRecord(definition)) {
		throw new Error(`Server "${name}" must be an object`);
	}
	const kind = definition.type ?? definition.kind ?? "stdio";
	if (
		kind !== "stdio" &&
		kind !== "sse" &&
		kind !== "streamable_http"
	) {
		throw new Error(`Server "${name}" must declare a supported transport type`);
	}
	if (definition.command !== undefined && definition.command !== null && typeof definition.command !== "string") {
		throw new Error(`Server "${name}" command must be a string`);
	}
	if (definition.url !== undefined && definition.url !== null && typeof definition.url !== "string") {
		throw new Error(`Server "${name}" url must be a string`);
	}
	return {
		name,
		kind,
		command: definition.command ?? undefined,
		args: parseStringArray(definition.args, `Server "${name}" args`),
		env: parseStringRecord(definition.env, `Server "${name}" env`),
		url: definition.url ?? undefined,
		headers: parseStringRecord(definition.headers, `Server "${name}" headers`),
	};
}

export function ServerImportDrawer({
	open,
	onOpenChange,
	onImported,
}: {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	onImported?: () => void;
}) {
	const [text, setText] = useState<string>(sample());
	const [preview, setPreview] = useState<PreviewResult | null>(null);
	const [importing, setImporting] = useState(false);
	const importInFlightRef = useRef(false);
	const { t } = useTranslation();
	const effectiveOpen = resolveImportDrawerOpen(open, importing);

	useEffect(() => {
		if (!effectiveOpen) {
			setPreview(null);
		}
	}, [effectiveOpen]);

	const previewM = useMutation<PreviewResult, unknown, PreviewPayload>({
		mutationFn: async (payload) => serversApi.previewServers(payload),
		onSuccess: (res) => setPreview(res as PreviewResult),
		onError: (e) => notifyError("Preview failed", String(e)),
	});

	function parsePayload(): {
		ok: boolean;
		payload?: PreviewPayload;
		error?: string;
	} {
		try {
			const obj = JSON.parse(text) as unknown;
			if (isRecord(obj) && isRecord(obj.mcpServers)) {
				const drafts = Object.entries(obj.mcpServers).map(([name, definition]) =>
					parsePreviewDraft(name, definition),
				);
				return { ok: true, payload: buildPreviewPayload(drafts) };
			}
			if (isRecord(obj) && Array.isArray(obj.servers)) {
				const drafts = obj.servers.map((definition) => {
					if (!isRecord(definition) || typeof definition.name !== "string") {
						throw new Error("servers[] items must include name");
					}
					return parsePreviewDraft(definition.name, definition);
				});
				return {
					ok: true,
					payload: buildPreviewPayload(drafts),
				};
			}
			return {
				ok: false,
				error: "JSON must include `mcpServers` mapping or `servers` array",
			};
		} catch (e) {
			return { ok: false, error: String(e) };
		}
	}

	function parseImport(): {
		ok: boolean;
		payload?: ImportPayload;
		error?: string;
	} {
		try {
			const obj = JSON.parse(text);
			if (obj.mcpServers && typeof obj.mcpServers === "object") {
				return { ok: true, payload: { mcpServers: obj.mcpServers } };
			}
			if (Array.isArray(obj.servers)) {
				const mapping: Record<string, unknown> = {};
				for (const s of obj.servers) {
					if (!s?.name)
						return { ok: false, error: "servers[] items must include name" };
					mapping[s.name] = {
						type: s.kind || s.type || "stdio",
						command: s.command ?? null,
						args: s.args ?? null,
						env: s.env ?? null,
						url: s.url ?? null,
					};
				}
				return { ok: true, payload: { mcpServers: mapping } };
			}
			return {
				ok: false,
				error: "JSON must include `mcpServers` mapping or `servers` array",
			};
		} catch (e) {
			return { ok: false, error: String(e) };
		}
	}

	async function doPreview() {
		const p = parsePayload();
		if (!p.ok || !p.payload) return notifyError("Invalid JSON", p.error);
		previewM.mutate(p.payload);
	}

	async function doImport() {
		if (importInFlightRef.current) {
			return;
		}
		const p = parseImport();
		if (!p.ok || !p.payload) return notifyError("Invalid JSON", p.error);
		importInFlightRef.current = true;
		try {
			setImporting(true);
			const targetProfileId = await resolveAutoAddTargetProfileId({
				autoAddEnabled:
					useAppStore.getState().dashboardSettings
						.autoAddServerToDefaultProfile,
			});
			const res = await serversApi.importServers(p.payload);
			const stats = extractImportStats(res);
			const didSucceed =
				typeof res?.success === "boolean"
					? res.success
					: (res as { status?: string })?.status === "success" ||
					!("error" in (res ?? {}));
			if (didSucceed) {
				await completeServerImportForProfile(targetProfileId, stats);
				const { importedCount, skippedCount, skippedServers, skippedDetails } =
					stats;
				const skippedSummary = summarizeSkipped(skippedDetails, t);
				const fallbackList = formatNameList(skippedServers, t);
				const skippedDescription = skippedSummary
					? skippedSummary
					: skippedCount > 0
						? `${skippedCount} server${skippedCount > 1 ? "s" : ""} skipped${fallbackList ? ` (${fallbackList})` : ""}`
						: "";
				const messageParts: string[] = [
					`Imported ${importedCount} server${importedCount === 1 ? "" : "s"}`,
				];
				if (skippedCount > 0) {
					messageParts.push(skippedDescription);
				}
				notifySuccess("Import completed", messageParts.join("; "));
				if (skippedCount > 0 && importedCount === 0) {
					notifyInfo(
						"Skipped existing servers",
						skippedDescription ||
						`${skippedCount} server${skippedCount > 1 ? "s" : ""} skipped (already installed).`,
					);
				}
				if (onImported) onImported();
				onOpenChange(false);
				return;
			}
			notifyError(
				t("profileSyncErrors.importFailedTitle"),
				String(res.error ?? "Unknown error"),
			);
		} catch (e) {
			notifyError(
				t("profileSyncErrors.importFailedTitle"),
				t(profileSyncErrorTranslationKey(e)),
			);
		} finally {
			importInFlightRef.current = false;
			setImporting(false);
		}
	}

	function formatJson() {
		try {
			const obj = JSON.parse(text);
			setText(JSON.stringify(obj, null, 2));
		} catch {
			// ignore
		}
	}

	return (
		<Drawer
			open={effectiveOpen}
			onOpenChange={(nextOpen) => {
				if (!shouldAcceptImportDrawerChange(nextOpen, importInFlightRef.current)) {
					return;
				}
				onOpenChange(nextOpen);
			}}
		>
			<DrawerContent>
				<DrawerHeader>
					<DrawerTitle>Import / Preview Servers</DrawerTitle>
					<DrawerDescription>
						Paste JSON with `mcpServers` mapping or a `servers` array.
					</DrawerDescription>
				</DrawerHeader>
				<div className="p-4 space-y-4">
					<Textarea
						rows={12}
						value={text}
						onChange={(e) => setText(e.target.value)}
						className="font-mono text-xs"
					/>
					<div className="flex gap-2">
						<Button variant="outline" onClick={formatJson}>
							Format
						</Button>
						<Button onClick={doPreview} disabled={previewM.isPending}>
							Preview
						</Button>
						<Button variant="secondary" onClick={doImport} disabled={importing}>
							Import
						</Button>
					</div>

					{preview && (
						<div className="rounded border p-3">
							{preview.success && preview.data?.items?.length ? (
								<div className="space-y-2 text-sm">
									{preview.data.items.map((it) => {
										const name = typeof it.name === "string" ? it.name : "Unnamed";
										const hasError = it.ok === false;
										const errorMessage =
											typeof it.error === "string"
												? it.error
												: it.error instanceof Error
													? it.error.message
													: undefined;
										const toolsCount = Array.isArray(it.tools?.items)
											? it.tools?.items?.length ?? 0
											: 0;
										const resourcesCount = Array.isArray(it.resources?.items)
											? it.resources?.items?.length ?? 0
											: 0;
										const templatesCount = Array.isArray(it.resource_templates?.items)
											? it.resource_templates?.items?.length ?? 0
											: 0;
										const promptsCount = Array.isArray(it.prompts?.items)
											? it.prompts?.items?.length ?? 0
											: 0;

										return (
											<div key={name} className="rounded border p-2">
												<div className="font-medium">
													{name}{" "}
													{hasError ? (
														<span className="text-red-500">(error)</span>
													) : null}
												</div>
												{errorMessage ? (
													<div className="text-xs text-red-500">
														{errorMessage}
													</div>
												) : null}
												<div className="text-xs text-slate-500 mt-1">
													tools: {toolsCount} • resources: {resourcesCount} • templates: {templatesCount} • prompts: {promptsCount}
												</div>
											</div>
										);
									})}
								</div>
							) : (
								<div className="text-sm text-slate-500">No preview data.</div>
							)}
						</div>
					)}
				</div>
				<DrawerFooter />
			</DrawerContent>
		</Drawer>
	);
}

function sample() {
	return JSON.stringify(
		{
			mcpServers: {
				example_stdio: {
					type: "stdio",
					command: "node",
					args: ["server.js"],
					env: { NODE_ENV: "production" },
				},
				example_http: { type: "streamable_http", url: "http://localhost:9000" },
			},
		},
		null,
		2,
	);
}

export default ServerImportDrawer;
