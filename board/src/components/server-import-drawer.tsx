import { useMutation } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { extractImportStats, serversApi } from "../lib/api";
import { notifyError, notifyInfo, notifySuccess } from "../lib/notify";
import { formatNameList, summarizeSkipped } from "../lib/server-import-utils";
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

interface PreviewServerDefinition {
	name: string;
	kind: string;
	command?: unknown;
	args?: unknown;
	env?: unknown;
	url?: unknown;
}

interface PreviewPayload {
	servers: PreviewServerDefinition[];
	include_details: boolean;
}

interface ImportPayload {
	mcpServers: Record<string, PreviewServerDefinition>;
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

	useEffect(() => {
		if (!open) {
			setPreview(null);
		}
	}, [open]);

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
			const obj = JSON.parse(text);
			if (obj.mcpServers && typeof obj.mcpServers === "object") {
				const servers = Object.keys(obj.mcpServers).map((name) => {
					const c = obj.mcpServers[name] || {};
					return {
						name,
						kind: c.type || c.kind || "stdio",
						command: c.command ?? null,
						args: c.args ?? null,
						env: c.env ?? null,
						url: c.url ?? null,
					};
				});
				return { ok: true, payload: { servers, include_details: true } };
			}
			if (Array.isArray(obj.servers)) {
				return {
					ok: true,
					payload: {
						servers: obj.servers as PreviewServerDefinition[],
						include_details: true,
					},
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
				const mapping: Record<string, PreviewServerDefinition> = {};
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
		const p = parseImport();
		if (!p.ok || !p.payload) return notifyError("Invalid JSON", p.error);
		try {
			setImporting(true);
			const res = await serversApi.importServers(p.payload);
			const stats = extractImportStats(res);
			const didSucceed =
				typeof res?.success === "boolean"
					? res.success
					: (res as { status?: string })?.status === "success" ||
						!("error" in (res ?? {}));
			if (didSucceed) {
				const { importedCount, skippedCount, skippedServers, skippedDetails } =
					stats;
				const skippedSummary = summarizeSkipped(skippedDetails);
				const fallbackList = formatNameList(skippedServers);
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
			notifyError("Import failed", String(res.error ?? "Unknown error"));
		} catch (e) {
			notifyError("Import failed", String(e));
		} finally {
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
		<Drawer open={open} onOpenChange={onOpenChange}>
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
