import { Copy, Eraser, Loader2, Play, RotateCcw } from "lucide-react";
import type { ReactNode } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { CardListScrollBody } from "../../components/card-list-scroll-body";
import { InspectorMcpResponseViewer } from "../../components/inspector-mcp-response-viewer";
import { SchemaForm } from "../../components/schema-form";
import { defaultFromSchema } from "../../components/schema-form-utils";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { ButtonGroup } from "../../components/ui/button-group";
import { Label } from "../../components/ui/label";
import { Textarea } from "../../components/ui/textarea";
import { inspectorApi, isInspectorSessionUnavailableError } from "../../lib/api";
import { writeClipboardText } from "../../lib/clipboard";
import type { InspectorNativeTargetRequest } from "../../lib/hooks/use-inspector-native-session";
import type { CreateInspectorLogEntryInput } from "../../lib/inspector-event-log";
import type { InspectorCapabilityKind } from "../../lib/inspector-capability";
import { notifyError, notifySuccess, stringifyError } from "../../lib/notify";
import { cn } from "../../lib/utils";
import type { JsonObject, JsonSchema, JsonValue } from "../../types/json";
import {
	INSPECTOR_CAPABILITY_FAMILIES,
	type InspectorCapabilityFamily,
	type InspectorCapabilityFamilyOption,
	type InspectorCapabilityListItem,
} from "./inspector-feature-config";
import {
	buildResourceUriFromTemplate,
	missingResourceTemplateVariables,
	schemaFromResourceTemplateUri,
} from "./inspector-resource-template-uri";

type InspectorCapabilityWorkspaceProps = {
	activeFamily: InspectorCapabilityFamily | null;
	selectedItem: InspectorCapabilityListItem | null;
	items: InspectorCapabilityListItem[];
	targetRequest: InspectorNativeTargetRequest | null;
	serverLogId: string | null;
	requestTimeoutMs: number;
	ensureSession: () => Promise<string | undefined>;
	onSessionUnavailable: () => void;
	onLogActivity: (entry: CreateInspectorLogEntryInput) => void;
};

type InspectorCallableFamily =
	| "tools"
	| "prompts"
	| "resources"
	| "resource_templates";

type InspectorInvocationState =
	| { status: "idle"; response: null; error: null; durationMs: null }
	| { status: "running"; response: null; error: null; durationMs: null }
	| { status: "success"; response: unknown; error: null; durationMs: number }
	| { status: "error"; response: null; error: string; durationMs: number };

type InspectorApiResponse = {
	success?: boolean;
	data?: unknown;
	error?: unknown;
};

const EMPTY_ARGS: JsonObject = {};

function capabilityFamilyToKind(
	family: InspectorCapabilityFamily,
): InspectorCapabilityKind | null {
	switch (family) {
		case "tools":
			return "tool";
		case "prompts":
			return "prompt";
		case "resources":
			return "resource";
		case "resource_templates":
			return "template";
		default:
			return null;
	}
}

function capabilityFamilyToMethod(
	family: InspectorCallableFamily,
): "tools/call" | "prompts/get" | "resources/read" {
	switch (family) {
		case "tools":
			return "tools/call";
		case "prompts":
			return "prompts/get";
		case "resources":
		case "resource_templates":
			return "resources/read";
	}
}

function callableFamilyFromActiveFamily(
	activeFamily: InspectorCapabilityFamily | null,
): InspectorCallableFamily | null {
	switch (activeFamily) {
		case "tools":
		case "prompts":
		case "resources":
		case "resource_templates":
			return activeFamily;
		default:
			return null;
	}
}

function isJsonObject(value: JsonValue): value is JsonObject {
	return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function schemaToJsonSchema(
	schema: Record<string, unknown> | undefined,
): JsonSchema | undefined {
	return schema as JsonSchema | undefined;
}

function defaultJsonObjectFromSchema(
	schema: Record<string, unknown> | undefined,
): JsonObject {
	const jsonSchema = schemaToJsonSchema(schema);
	if (!jsonSchema) return EMPTY_ARGS;
	const value = defaultFromSchema(jsonSchema);
	return isJsonObject(value) ? value : EMPTY_ARGS;
}

function serializeJsonObject(value: JsonObject): string {
	return JSON.stringify(value, null, 2);
}

function parseJsonObject(value: string): JsonObject {
	const parsed = JSON.parse(value) as JsonValue;
	if (!isJsonObject(parsed)) {
		throw new Error("Request arguments must be a JSON object.");
	}
	return parsed;
}

function extractResultPayload(response: InspectorApiResponse): unknown {
	const data = response.data;
	if (data && typeof data === "object" && "result" in data) {
		return (data as { result?: unknown }).result ?? data;
	}
	return data ?? response;
}

function schemaPreview(value: Record<string, unknown> | undefined, emptyLabel: string) {
	if (!value || Object.keys(value).length === 0) {
		return <p className="text-xs text-muted-foreground">{emptyLabel}</p>;
	}
	return (
		<pre className="max-h-52 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-background/70 p-3 font-mono text-xs text-muted-foreground">
			{JSON.stringify(value, null, 2)}
		</pre>
	);
}

function capabilityMetadataPreview(
	selectedItem: InspectorCapabilityListItem,
	familyMeta: InspectorCapabilityFamilyOption,
): Record<string, unknown> {
	return {
		family: familyMeta.value,
		key: selectedItem.key,
		title: selectedItem.title,
		description: selectedItem.description ?? null,
		input_schema: selectedItem.inputSchema ?? null,
		output_schema: selectedItem.outputSchema ?? null,
		metadata: selectedItem.metadata ?? null,
	};
}

function requestPreview(
	family: InspectorCallableFamily,
	selectedItem: InspectorCapabilityListItem,
	args: JsonObject,
	timeoutMs: number,
): Record<string, unknown> {
	switch (family) {
		case "tools":
			return {
				method: "tools/call",
				params: {
					name: selectedItem.key,
					arguments: args,
					timeout_ms: timeoutMs,
				},
			};
		case "prompts":
			return {
				method: "prompts/get",
				params: {
					name: selectedItem.key,
					arguments: args,
				},
			};
		case "resource_templates": {
			const missingVariables = missingResourceTemplateVariables(selectedItem.key, args);
			if (missingVariables.length > 0) {
				return requestPreviewWithError(
					family,
					selectedItem,
					`Missing required URI variable(s): ${missingVariables.join(", ")}`,
				);
			}
			return {
				method: "resources/read",
				params: {
					uri: buildResourceUriFromTemplate(selectedItem.key, args),
					template: selectedItem.key,
					arguments: args,
				},
			};
		}
		case "resources":
			return {
				method: "resources/read",
				params: {
					uri: selectedItem.key,
				},
			};
	}
}

function requestPreviewWithError(
	family: InspectorCallableFamily,
	selectedItem: InspectorCapabilityListItem,
	error: string,
): Record<string, unknown> {
	if (family === "resource_templates") {
		return {
			method: capabilityFamilyToMethod(family),
			params: {
				uri: "<invalid JSON>",
				template: selectedItem.key,
				arguments: "<invalid JSON>",
			},
			error,
		};
	}

	return {
		method: capabilityFamilyToMethod(family),
		params: {
			name: selectedItem.key,
			arguments: "<invalid JSON>",
		},
		error,
	};
}

function validateResourceTemplateArgs(
	selectedItem: InspectorCapabilityListItem,
	args: JsonObject,
): string | null {
	const missingVariables = missingResourceTemplateVariables(selectedItem.key, args);
	if (missingVariables.length === 0) return null;
	return `Missing required URI variable(s): ${missingVariables.join(", ")}`;
}

function invocationBadgeVariant(
	status: InspectorInvocationState["status"],
): "outline" | "secondary" | "success" | "warning" | "destructive" {
	switch (status) {
		case "success":
			return "success";
		case "running":
			return "warning";
		case "error":
			return "destructive";
		case "idle":
			return "outline";
	}
}

function invocationTone(
	status: InspectorInvocationState["status"],
): "default" | "good" | "warn" | "bad" {
	switch (status) {
		case "success":
			return "good";
		case "running":
			return "warn";
		case "error":
			return "bad";
		case "idle":
			return "default";
	}
}

function invocationStatusLabel(status: InspectorInvocationState["status"]): string {
	switch (status) {
		case "success":
			return "OK";
		case "running":
			return "PENDING";
		case "error":
			return "ERROR";
		case "idle":
			return "READY";
	}
}

function durationLabel(invocation: InspectorInvocationState): string {
	if (invocation.durationMs != null) {
		return `${invocation.durationMs}ms`;
	}
	if (invocation.status === "running") {
		return "pending";
	}
	return "n/a";
}

function timeoutLabel(
	callableFamily: InspectorCallableFamily | null,
	requestTimeoutMs: number,
): string {
	return callableFamily === "tools" ? `${requestTimeoutMs}ms` : "n/a";
}

function actionLabel(callableFamily: InspectorCallableFamily | null): string {
	switch (callableFamily) {
		case "resources":
		case "resource_templates":
			return "Read";
		case "prompts":
			return "Get";
		default:
			return "Call";
	}
}

function previewLabel(callableFamily: InspectorCallableFamily | null): string {
	return callableFamily ? "Request preview" : "Advertised metadata";
}

function copyPreviewLabel(callableFamily: InspectorCallableFamily | null): string {
	return callableFamily ? "Copy request preview" : "Copy advertised metadata";
}

function copySuccessTitle(callableFamily: InspectorCallableFamily | null): string {
	return callableFamily ? "Request copied" : "Metadata copied";
}

function copySuccessMessage(callableFamily: InspectorCallableFamily | null): string {
	return callableFamily
		? "Request preview copied to clipboard."
		: "Advertised metadata copied to clipboard.";
}

function responsePlaceholder(
	invocation: InspectorInvocationState,
	callableFamily: InspectorCallableFamily | null,
): string {
	switch (invocation.status) {
		case "running":
			return "Waiting for MCP response...";
		case "error":
			return invocation.error;
		case "idle":
		case "success":
			return callableFamily
				? "Run the request to render a response here."
				: "This capability family exposes metadata only in the current Inspector API.";
	}
}

function resolveRequestArgs({
	callableFamily,
	useRaw,
	args,
	argsJson,
}: {
	callableFamily: InspectorCallableFamily;
	useRaw: boolean;
	args: JsonObject;
	argsJson: string;
}): JsonObject {
	if (callableFamily === "resources") {
		return EMPTY_ARGS;
	}
	if (useRaw) {
		return parseJsonObject(argsJson);
	}
	return args;
}

function InspectorEvidenceStat({
	label,
	value,
	tone,
}: {
	label: string;
	value: string;
	tone?: "default" | "good" | "warn" | "bad";
}): ReactNode {
	return (
		<div
			className={cn(
				"min-w-0 rounded-md border border-border bg-background/60 px-3 py-2",
				tone === "good" && "border-emerald-500/30 bg-emerald-500/5",
				tone === "warn" && "border-amber-500/30 bg-amber-500/5",
				tone === "bad" && "border-destructive/30 bg-destructive/5",
			)}
		>
			<p className="text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">
				{label}
			</p>
			<p className="mt-1 truncate font-mono text-xs text-foreground">{value}</p>
		</div>
	);
}

export function InspectorCapabilityWorkspace({
	activeFamily,
	selectedItem,
	items,
	targetRequest,
	serverLogId,
	requestTimeoutMs,
	ensureSession,
	onSessionUnavailable,
	onLogActivity,
}: InspectorCapabilityWorkspaceProps) {
	const familyMeta = INSPECTOR_CAPABILITY_FAMILIES.find(
		(entry) => entry.value === activeFamily,
	);
	const capabilityKind = activeFamily ? capabilityFamilyToKind(activeFamily) : null;
	const callableFamily = callableFamilyFromActiveFamily(activeFamily);
	const inputSchema = useMemo(() => {
		if (activeFamily === "resource_templates" && selectedItem) {
			return selectedItem.inputSchema ?? schemaFromResourceTemplateUri(selectedItem.key);
		}
		return selectedItem?.inputSchema;
	}, [activeFamily, selectedItem?.inputSchema, selectedItem?.key]);
	const outputSchema = selectedItem?.outputSchema;
	const [useRaw, setUseRaw] = useState(false);
	const [args, setArgs] = useState<JsonObject>(EMPTY_ARGS);
	const [argsJson, setArgsJson] = useState("{}");
	const [invocation, setInvocation] = useState<InspectorInvocationState>({
		status: "idle",
		response: null,
		error: null,
		durationMs: null,
	});
	const selectionKey = `${activeFamily ?? "none"}:${selectedItem?.key ?? ""}`;
	const selectionKeyRef = useRef(selectionKey);
	const invocationSeqRef = useRef(0);

	useEffect(() => {
		selectionKeyRef.current = selectionKey;
		invocationSeqRef.current += 1;
		const nextArgs = defaultJsonObjectFromSchema(inputSchema);
		setArgs(nextArgs);
		setArgsJson(serializeJsonObject(nextArgs));
		setUseRaw(false);
		setInvocation({ status: "idle", response: null, error: null, durationMs: null });
	}, [inputSchema, selectionKey]);

	const rawArgsPreview = useMemo(() => {
		if (!useRaw) {
			return { value: args, error: null };
		}
		try {
			return { value: parseJsonObject(argsJson), error: null };
		} catch (error) {
			return { value: null, error: stringifyError(error) };
		}
	}, [args, argsJson, useRaw]);

	const currentRequestPreview = useMemo(() => {
		if (!callableFamily || !selectedItem) return null;
		if (callableFamily !== "resources" && useRaw && rawArgsPreview.error) {
			return requestPreviewWithError(callableFamily, selectedItem, rawArgsPreview.error);
		}
		const previewArgs =
			callableFamily === "resources"
				? EMPTY_ARGS
				: rawArgsPreview.value ?? EMPTY_ARGS;
		return requestPreview(callableFamily, selectedItem, previewArgs, requestTimeoutMs);
	}, [callableFamily, rawArgsPreview, requestTimeoutMs, selectedItem, useRaw]);

	const currentMetadataPreview = useMemo(() => {
		if (!selectedItem || !familyMeta) return null;
		return capabilityMetadataPreview(selectedItem, familyMeta);
	}, [familyMeta, selectedItem]);

	const serializedRequestPreview = useMemo(
		() =>
			currentRequestPreview
				? JSON.stringify(currentRequestPreview, null, 2)
				: "No request action available.",
		[currentRequestPreview],
	);
	const serializedMetadataPreview = useMemo(
		() =>
			currentMetadataPreview
				? JSON.stringify(currentMetadataPreview, null, 2)
				: "No capability metadata available.",
		[currentMetadataPreview],
	);

	const handleFillDefaults = useCallback(() => {
		const nextArgs = defaultJsonObjectFromSchema(inputSchema);
		setArgs(nextArgs);
		setArgsJson(serializeJsonObject(nextArgs));
	}, [inputSchema]);

	const handleClear = useCallback(() => {
		setArgs(EMPTY_ARGS);
		setArgsJson("{}");
	}, []);

	const handleCopyPreview = useCallback(async () => {
		try {
			await writeClipboardText(
				callableFamily ? serializedRequestPreview : serializedMetadataPreview,
			);
			notifySuccess(
				copySuccessTitle(callableFamily),
				copySuccessMessage(callableFamily),
			);
		} catch (error) {
			notifyError("Copy failed", stringifyError(error));
		}
	}, [callableFamily, serializedMetadataPreview, serializedRequestPreview]);

	const handleClearResponse = useCallback(() => {
		setInvocation({ status: "idle", response: null, error: null, durationMs: null });
	}, []);

	const handleInvoke = useCallback(async () => {
		if (!callableFamily || !selectedItem) return;
		if (!targetRequest || !serverLogId) {
			notifyError("Select a server", "Capability calls require a connected target.");
			return;
		}

		let requestArgs: JsonObject;
		try {
			requestArgs = resolveRequestArgs({
				callableFamily,
				useRaw,
				args,
				argsJson,
			});
		} catch (error) {
			notifyError("Invalid request JSON", stringifyError(error));
			return;
		}
		if (callableFamily === "resource_templates") {
			const templateError = validateResourceTemplateArgs(selectedItem, requestArgs);
			if (templateError) {
				notifyError("Invalid resource URI", templateError);
				return;
			}
		}

		const invocationSeq = invocationSeqRef.current + 1;
		invocationSeqRef.current = invocationSeq;
		const invocationSelectionKey = selectionKey;
		const isCurrentInvocation = () =>
			invocationSeqRef.current === invocationSeq &&
			selectionKeyRef.current === invocationSelectionKey;

		setInvocation({ status: "running", response: null, error: null, durationMs: null });
		const startedAt = Date.now();
		const method = capabilityFamilyToMethod(callableFamily);
		let sessionId: string | undefined;

		try {
			sessionId = await ensureSession();
			if (!sessionId) {
				throw new Error("Failed to open inspector session");
			}

			const preview = requestPreview(
				callableFamily,
				selectedItem,
				requestArgs,
				requestTimeoutMs,
			);
			const requestPayload = {
				jsonrpc: "2.0",
				method,
				params: {
					...preview.params,
					session_id: sessionId,
				},
			};

			onLogActivity({
				data: {
					event: "mcp_exchange",
					direction: "outbound",
					method,
					server_id: serverLogId,
					mode: "native",
					session_id: sessionId,
				},
				request: requestPayload,
			});

			let response: InspectorApiResponse;
			if (callableFamily === "tools") {
				response = (await inspectorApi.toolCall({
					...targetRequest,
					session_id: sessionId,
					tool: selectedItem.key,
					arguments: requestArgs,
					timeout_ms: requestTimeoutMs,
				})) as InspectorApiResponse;
			} else if (callableFamily === "prompts") {
				response = (await inspectorApi.promptGet({
					...targetRequest,
					session_id: sessionId,
					name: selectedItem.key,
					arguments: requestArgs,
				})) as InspectorApiResponse;
			} else {
				const uri =
					callableFamily === "resource_templates"
						? buildResourceUriFromTemplate(selectedItem.key, requestArgs)
						: selectedItem.key;
				response = (await inspectorApi.resourceRead({
					...targetRequest,
					session_id: sessionId,
					uri,
				})) as InspectorApiResponse;
			}

			if (!response?.success) {
				throw new Error(response?.error ? String(response.error) : `${method} failed`);
			}

			const result = extractResultPayload(response);
			const durationMs = Date.now() - startedAt;
			onLogActivity({
				data: {
					event: "mcp_exchange",
					direction: "inbound",
					method,
					server_id: serverLogId,
					mode: "native",
					session_id: sessionId,
				},
				response: {
					jsonrpc: "2.0",
					result,
				},
				durationMs,
			});
			if (isCurrentInvocation()) {
				setInvocation({
					status: "success",
					response: result,
					error: null,
					durationMs,
				});
				notifySuccess("Capability completed", `${method} finished in ${durationMs}ms.`);
			}
		} catch (error) {
			if (isInspectorSessionUnavailableError(error)) {
				onSessionUnavailable();
			}
			const message = stringifyError(error);
			onLogActivity({
				data: {
					event: "error",
					call_id: `${method}:${selectedItem.key}`,
					server_id: serverLogId,
					message,
				},
				request: {
					operation: method,
					preview: currentRequestPreview,
					...(sessionId ? { session_id: sessionId } : {}),
				},
				durationMs: Date.now() - startedAt,
			});
			if (isCurrentInvocation()) {
				setInvocation({
					status: "error",
					response: null,
					error: message,
					durationMs: Date.now() - startedAt,
				});
				notifyError("Capability call failed", message);
			}
		}
	}, [
		args,
		argsJson,
		callableFamily,
		currentRequestPreview,
		ensureSession,
		onLogActivity,
		onSessionUnavailable,
		requestTimeoutMs,
		selectedItem,
		selectionKey,
		serverLogId,
		targetRequest,
		useRaw,
	]);

	if (!activeFamily) {
		return (
			<div className="flex min-h-0 flex-1 flex-col items-center justify-center rounded-md border border-dashed border-border bg-card/20 p-8 text-center">
				<p className="text-base font-medium text-foreground">Select a capability family</p>
				<p className="mt-2 max-w-md text-sm text-muted-foreground">
					Expand a family in the sidebar, run List, then choose an item to inspect its
					schema step by step.
				</p>
			</div>
		);
	}

	if (!familyMeta) {
		return null;
	}

	if (!selectedItem) {
		return (
			<div className="flex min-h-0 flex-1 items-center justify-center rounded-md border border-dashed border-border bg-card/20 p-6 text-sm text-muted-foreground">
				{items.length > 0
					? "Choose a capability from the sidebar list."
					: "Run List in the sidebar to populate this workspace."}
			</div>
		);
	}

	const canInvoke = Boolean(callableFamily && targetRequest);
	const responseKind = capabilityKind ?? "tool";
	const requestResponseTitle = callableFamily
		? "Request / Response"
		: "Capability Metadata";
	const requestResponseDescription = callableFamily
		? "Composer and payload panes follow the standalone Inspector activity log shape."
		: "List-only capabilities expose their advertised metadata without a call action.";
	const methodLabel = callableFamily ? capabilityFamilyToMethod(callableFamily) : "metadata";

	return (
		<div className="grid min-h-0 flex-1 gap-4 xl:grid-cols-[minmax(22rem,0.9fr)_minmax(0,1.35fr)]">
			<section className="flex min-h-0 flex-col rounded-md border border-border bg-card/40">
				<div className="border-b border-border p-4">
					<div className="flex flex-wrap items-start justify-between gap-3">
						<div className="min-w-0 space-y-2">
							<div className="flex flex-wrap items-center gap-2">
								<Badge variant="outline">{familyMeta.label}</Badge>
								{callableFamily ? (
									<Badge variant="secondary">
										{capabilityFamilyToMethod(callableFamily)}
									</Badge>
								) : (
									<Badge variant="secondary">List only</Badge>
								)}
							</div>
							<div>
								<p className="truncate text-lg font-semibold text-foreground">
									{selectedItem.title}
								</p>
								<p className="mt-1 break-all font-mono text-xs text-muted-foreground">
									{selectedItem.key}
								</p>
							</div>
						</div>
						<Button
							type="button"
							className="h-9 gap-2"
							disabled={!canInvoke || invocation.status === "running"}
							onClick={() => void handleInvoke()}
						>
							{invocation.status === "running" ? (
								<Loader2 className="h-4 w-4 animate-spin" />
							) : (
								<Play className="h-4 w-4" />
							)}
							{actionLabel(callableFamily)}
						</Button>
					</div>
					<p className="mt-3 text-sm leading-relaxed text-muted-foreground">
						{selectedItem.description || "No description provided."}
					</p>
				</div>

				<div className="flex min-h-0 flex-1 flex-col gap-4 p-4">
					{callableFamily && callableFamily !== "resources" ? (
						<div className="space-y-3">
							<div className="flex items-center justify-between gap-3">
								<Label>Request parameters</Label>
								<ButtonGroup className="overflow-hidden rounded-md border border-input text-xs divide-x divide-input">
									<Button
										type="button"
										variant="ghost"
										size="sm"
										className="h-auto px-2 py-1 text-xs font-medium"
										onClick={handleFillDefaults}
									>
										<RotateCcw className="mr-1 h-3 w-3" />
										Defaults
									</Button>
									<Button
										type="button"
										variant="ghost"
										size="sm"
										className="h-auto px-2 py-1 text-xs font-medium"
										onClick={handleClear}
									>
										Clear
									</Button>
									<Button
										type="button"
										variant={useRaw ? "default" : "ghost"}
										size="sm"
										className="h-auto px-2 py-1 text-xs font-medium"
										onClick={() => setUseRaw((value) => !value)}
									>
										{useRaw ? "Form" : "JSON"}
									</Button>
								</ButtonGroup>
							</div>

							<CardListScrollBody className="min-h-[14rem] flex-none">
								<div className="p-3">
									{useRaw || !inputSchema ? (
										<Textarea
											value={argsJson}
											onChange={(event) => setArgsJson(event.target.value)}
											className="min-h-[12rem] resize-none border-0 bg-transparent p-0 font-mono text-xs shadow-none focus-visible:ring-0 focus-visible:ring-offset-0"
										/>
									) : (
										<SchemaForm
											schema={schemaToJsonSchema(inputSchema)!}
											value={args}
											compact
											onChange={(value) => {
												const next = isJsonObject(value) ? value : EMPTY_ARGS;
												setArgs(next);
												setArgsJson(serializeJsonObject(next));
											}}
										/>
									)}
								</div>
							</CardListScrollBody>
						</div>
					) : (
						<div className="rounded-md border border-dashed border-border bg-background/40 px-3 py-2 text-xs leading-relaxed text-muted-foreground">
							{callableFamily === "resources"
								? "This resource is read directly from its URI."
								: "This capability is currently inspectable through its advertised list metadata."}
						</div>
					)}

					<div className="grid min-h-0 gap-3 lg:grid-cols-2">
						<div className="space-y-2">
							<p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
								Input schema
							</p>
							{schemaPreview(inputSchema, "No input schema listed.")}
						</div>
						<div className="space-y-2">
							<p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
								Output schema
							</p>
							{schemaPreview(outputSchema, "No output schema listed.")}
						</div>
					</div>
				</div>
			</section>

			<section className="flex min-h-0 flex-col rounded-md border border-border bg-card/40">
				<div className="border-b border-border p-4">
					<div className="flex items-center justify-between gap-3">
						<div>
							<p className="text-sm font-semibold text-foreground">
								{requestResponseTitle}
							</p>
							<p className="mt-1 text-xs text-muted-foreground">
								{requestResponseDescription}
							</p>
						</div>
						<div className="flex shrink-0 items-center gap-2">
							<Badge variant={invocationBadgeVariant(invocation.status)}>
								{invocationStatusLabel(invocation.status)}
							</Badge>
							<Button
								type="button"
								variant="outline"
								size="icon"
								className="h-8 w-8"
								disabled={invocation.status === "idle" || invocation.status === "running"}
								aria-label="Clear response"
								onClick={handleClearResponse}
							>
								<Eraser className="h-3.5 w-3.5" />
							</Button>
						</div>
					</div>
					<div className="mt-4 grid gap-2 md:grid-cols-4">
						<InspectorEvidenceStat label="Method" value={methodLabel} />
						<InspectorEvidenceStat
							label="Status"
							value={invocationStatusLabel(invocation.status)}
							tone={invocationTone(invocation.status)}
						/>
						<InspectorEvidenceStat
							label="Timeout"
							value={timeoutLabel(callableFamily, requestTimeoutMs)}
						/>
						<InspectorEvidenceStat label="Duration" value={durationLabel(invocation)} />
					</div>
				</div>

				<div className="grid min-h-0 flex-1 gap-4 p-4 lg:grid-rows-[minmax(11rem,0.75fr)_minmax(16rem,1.25fr)]">
					<div className="flex min-h-0 flex-col gap-2">
						<div className="flex items-center justify-between gap-2">
							<p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
								{previewLabel(callableFamily)}
							</p>
							<Button
								type="button"
								variant="outline"
								size="icon"
								className="h-7 w-7"
								disabled={callableFamily ? !currentRequestPreview : !currentMetadataPreview}
								aria-label={copyPreviewLabel(callableFamily)}
								onClick={() => void handleCopyPreview()}
							>
								<Copy className="h-3.5 w-3.5" />
							</Button>
						</div>
						<CardListScrollBody>
							<pre className="p-3 font-mono text-xs leading-relaxed text-muted-foreground">
								{callableFamily
									? serializedRequestPreview
									: serializedMetadataPreview}
							</pre>
						</CardListScrollBody>
					</div>

					<div className="flex min-h-0 flex-col">
						{invocation.status === "success" ? (
							<InspectorMcpResponseViewer
								response={invocation.response}
								kind={responseKind}
								fill
								title="Response"
								className="min-h-0 flex-1"
							/>
						) : (
							<div
								className={cn(
									"flex min-h-0 flex-1 items-center justify-center rounded-md border border-dashed border-border bg-background/40 p-6 text-center text-sm text-muted-foreground",
									invocation.status === "error" && "border-destructive/40 text-destructive",
								)}
							>
								{responsePlaceholder(invocation, callableFamily)}
							</div>
						)}
					</div>
				</div>
			</section>
		</div>
	);
}
