import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
	AlertCircle,
	Check,
	CheckCircle2,
	ChevronDown,
	ChevronsUpDown,
	Code2,
	FolderOpen,
	Info,
	ImageIcon,
	Loader2,
	Plus,
	Sparkles,
	Trash2,
} from "lucide-react";
import React, { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { type ControllerRenderProps, useForm } from "react-hook-form";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import * as z from "zod";
import { clientsApi } from "../lib/api";
import { mapDashboardSettingsToClientBackupPolicy } from "../lib/client-backup-policy";
import {
	applyClientConfigWithResolvedSelection,
	canApplyClientConfigWithState,
	resolveClientConfigSyncErrorMessage,
	resolveClientConfigMode,
} from "../lib/client-config-sync";
import { notifyError, notifyInfo, notifySuccess } from "../lib/notify";
import { pickClientConfigFilePath, readAbsolutePathFromFile } from "../lib/pick-client-config-file";
import { isTauriEnvironmentSync } from "../lib/platform";
import { useAppStore } from "../lib/store";
import type {
	ClientConfigFileParse,
	ClientConfigFileParseInspectExistingReq,
	ClientConfigFileParseInspectResp,
	ClientConfigFileParseInspectReq,
	ClientConnectionMode,
	TransportRuleData,
	ClientInfo,
} from "../lib/types";
import { cn } from "../lib/utils";
import { ConfirmDialog } from "./confirm-dialog";
import { Button } from "./ui/button";
import {
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
} from "./ui/command";
import {
	Drawer,
	DrawerContent,
	DrawerDescription,
	DrawerFooter,
	DrawerHeader,
	DrawerTitle,
} from "./ui/drawer";
import {
	Form,
	FormControl,
	FormDescription,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
} from "./ui/form";
import { Input } from "./ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import { Segment, type SegmentOption } from "./ui/segment";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs";
import { Textarea } from "./ui/textarea";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "./ui/tooltip";

type ClientFormMode = "create" | "edit";
type ClientConnectionShape = "local_with_config" | "local_without_config" | "remote_http";
const SUPPORTED_TRANSPORT_VALUES = ["streamable_http", "sse", "stdio"] as const;
type SupportedTransportValue = (typeof SUPPORTED_TRANSPORT_VALUES)[number];
const CONFIG_PARSE_FORMAT_VALUES = ["json", "json5", "toml", "yaml"] as const;
type ConfigParseFormatValue = (typeof CONFIG_PARSE_FORMAT_VALUES)[number];
const CONFIG_PARSE_CONTAINER_TYPE_VALUES = ["standard", "array"] as const;

function createFormSchema(t: TFunction) {
	return z.object({
		identifier: z.string().min(1),
		displayName: z.string().min(1),
		connectionShape: z.enum(["local_with_config", "local_without_config", "remote_http"]),
		supportedTransports: z
			.array(z.enum(SUPPORTED_TRANSPORT_VALUES))
			.min(
				1,
				t("detail.form.transportSupport.validation.required", {
					defaultValue: "Select at least one supported transport before saving.",
				}),
			),
		configPath: z.string().optional(),
		configFileParseFormat: z.enum(CONFIG_PARSE_FORMAT_VALUES),
		configFileParseContainerType: z.enum(CONFIG_PARSE_CONTAINER_TYPE_VALUES),
		configFileParseContainerKeysText: z.string().optional(),
		clientVersion: z.string().optional(),
		description: z.string().optional(),
		homepageUrl: z.string().optional(),
		docsUrl: z.string().optional(),
		supportUrl: z.string().optional(),
		logoUrl: z.string().optional(),
	});
}

interface ClientFormDrawerProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	mode: ClientFormMode;
	client?: ClientInfo | null;
	onSuccess?: (identifier: string) => void;
	onDeleteSuccess?: (identifier: string) => void;
}

function resolveEffectiveClientParse(client: ClientInfo | null | undefined): ClientConfigFileParse | null {
	if (!client) return null;
	return client.config_file_parse_override ?? client.config_file_parse_effective ?? null;
}

type ParseInspectionView = ClientConfigFileParseInspectResp;

interface EditableExtraFieldEntry {
	id: string;
	key: string;
	value: string;
}

interface TransportRuleEditorValue {
	includeType: boolean;
	typeValue: string;
	commandField: string;
	argsField: string;
	envField: string;
	urlField: string;
	headersField: string;
	extraFields: EditableExtraFieldEntry[];
}

interface TransportRulePreset {
	id: string;
	label: string;
	value: TransportRuleEditorValue;
}

type TransportRuleEditors = Record<SupportedTransportValue, TransportRuleEditorValue>;

function createEditorId(): string {
	return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function createEmptyTransportRuleEditor(): TransportRuleEditorValue {
	return {
		includeType: false,
		typeValue: "",
		commandField: "",
		argsField: "",
		envField: "",
		urlField: "",
		headersField: "",
		extraFields: [],
	};
}

function transportRuleEditorFromData(rule?: TransportRuleData | null): TransportRuleEditorValue {
	return {
		includeType: Boolean(rule?.include_type),
		typeValue: rule?.type_value?.toString() ?? "",
		commandField: rule?.command_field?.toString() ?? "",
		argsField: rule?.args_field?.toString() ?? "",
		envField: rule?.env_field?.toString() ?? "",
		urlField: rule?.url_field?.toString() ?? "",
		headersField: rule?.headers_field?.toString() ?? "",
		extraFields: Object.entries(rule?.extra_fields ?? {}).map(([key, value]) => ({
			id: createEditorId(),
			key,
			value: typeof value === "string" ? value : JSON.stringify(value),
		})),
	};
}

function transportRuleEditorsFromClient(client?: ClientInfo | null): TransportRuleEditors {
	return {
		streamable_http: transportRuleEditorFromData(client?.transports?.streamable_http),
		sse: transportRuleEditorFromData(client?.transports?.sse),
		stdio: transportRuleEditorFromData(client?.transports?.stdio),
	};
}

function cloneTransportRuleEditorValue(value: TransportRuleEditorValue): TransportRuleEditorValue {
	return {
		...value,
		extraFields: value.extraFields.map((entry) => ({ ...entry, id: createEditorId() })),
	};
}

function transportRuleEditorsSignature(editors: TransportRuleEditors): string {
	return JSON.stringify(
		Object.fromEntries(
			SUPPORTED_TRANSPORT_VALUES.map((transport) => {
				const editor = editors[transport] ?? createEmptyTransportRuleEditor();
				return [
					transport,
					{
						includeType: editor.includeType,
						typeValue: editor.typeValue.trim(),
						commandField: editor.commandField.trim(),
						argsField: editor.argsField.trim(),
						envField: editor.envField.trim(),
						urlField: editor.urlField.trim(),
						headersField: editor.headersField.trim(),
						extraFields: editor.extraFields.map((entry) => ({
							key: entry.key.trim(),
							value: entry.value.trim(),
						})),
					},
				];
			}),
		),
	);
}

function isSameSupportedTransports(
	left: SupportedTransportValue[],
	right: SupportedTransportValue[],
): boolean {
	if (left.length !== right.length) return false;
	return left.every((value, index) => value === right[index]);
}

function isTransportRuleEditorEmpty(value: TransportRuleEditorValue): boolean {
	return !value.includeType &&
		!value.typeValue.trim() &&
		!value.commandField.trim() &&
		!value.argsField.trim() &&
		!value.envField.trim() &&
		!value.urlField.trim() &&
		!value.headersField.trim() &&
		value.extraFields.length === 0;
}

function buildTransportRulePresets(
	transport: SupportedTransportValue,
	_client: ClientInfo | null | undefined,
	t: TFunction,
): TransportRulePreset[] {
	if (transport === "stdio") {
		return [
			{
				id: "common",
				label: t("detail.form.transportRules.presets.common", { defaultValue: "Common" }),
				value: {
					...createEmptyTransportRuleEditor(),
					commandField: "command",
					argsField: "args",
					envField: "env",
				},
			},
			{
				id: "with-type",
				label: t("detail.form.transportRules.presets.withType", { defaultValue: "With type" }),
				value: {
					...createEmptyTransportRuleEditor(),
					includeType: true,
					typeValue: "stdio",
					commandField: "command",
					argsField: "args",
					envField: "env",
				},
			},
		];
	}

	return [
		{
			id: "common",
			label: t("detail.form.transportRules.presets.common", { defaultValue: "Common" }),
			value: {
				...createEmptyTransportRuleEditor(),
				urlField: "url",
				headersField: "headers",
			},
		},
		{
			id: "with-type",
			label: t("detail.form.transportRules.presets.withType", { defaultValue: "With type" }),
			value: {
				...createEmptyTransportRuleEditor(),
				includeType: true,
				typeValue: transport,
				urlField: "url",
				headersField: "headers",
			},
		},
	];
}

function parseExtraFieldValue(raw: string): unknown {
	const trimmed = raw.trim();
	if (!trimmed) return "";
	try {
		return JSON.parse(trimmed);
	} catch {
		return trimmed;
	}
}

function transportRuleDataFromEditor(
	transport: SupportedTransportValue,
	editor: TransportRuleEditorValue,
	t: TFunction,
): TransportRuleData {
	const commandField = editor.commandField.trim();
	const argsField = editor.argsField.trim();
	const envField = editor.envField.trim();
	const urlField = editor.urlField.trim();
	const headersField = editor.headersField.trim();
	const typeValue = editor.typeValue.trim();

	if (transport === "stdio" && !commandField) {
		throw new Error(
			t("detail.form.transportRules.validation.commandRequired", {
				defaultValue: "STDIO requires a command field.",
			}),
		);
	}

	if ((transport === "sse" || transport === "streamable_http") && !urlField) {
		throw new Error(
			t("detail.form.transportRules.validation.urlRequired", {
				defaultValue: "HTTP-based transports require a URL field.",
			}),
		);
	}

	if (editor.includeType && !typeValue) {
		throw new Error(
			t("detail.form.transportRules.validation.typeValueRequired", {
				defaultValue: "Type value is required when including the type field.",
			}),
		);
	}

	const extraFields = editor.extraFields.reduce<Record<string, unknown>>((acc, entry) => {
		const key = entry.key.trim();
		if (!key) return acc;
		acc[key] = parseExtraFieldValue(entry.value);
		return acc;
	}, {});

	return {
		include_type: editor.includeType,
		type_value: editor.includeType ? typeValue || transport : null,
		command_field: commandField || null,
		args_field: argsField || null,
		env_field: envField || null,
		url_field: urlField || null,
		headers_field: headersField || null,
		extra_fields: Object.keys(extraFields).length > 0 ? extraFields : null,
	};
}

function buildTransportRulesPayload(
	transports: SupportedTransportValue[],
	editors: TransportRuleEditors,
	client: ClientInfo | null | undefined,
	t: TFunction,
	hasWritableRules: boolean,
): Record<string, TransportRuleData> {
	const selectedTransport = findSelectedTransport(client?.transports);

	if (!hasWritableRules) {
		const result: Record<string, TransportRuleData> = {};
		if (selectedTransport && transports.includes(selectedTransport)) {
			result[selectedTransport] = { selected: true };
		}
		return result;
	}

	return transports.reduce<Record<string, TransportRuleData>>((acc, transport) => {
		const selected = selectedTransport === transport ? true : undefined;
		const currentEditor = editors[transport] ?? createEmptyTransportRuleEditor();
		const editor = isTransportRuleEditorEmpty(currentEditor)
			? cloneTransportRuleEditorValue(
				(buildTransportRulePresets(transport, client, t).find((preset) => preset.id === "common")
					?.value ?? createEmptyTransportRuleEditor()),
			)
			: currentEditor;
		const rule = transportRuleDataFromEditor(transport, editor, t);
		acc[transport] = selected ? { ...rule, selected } : rule;
		return acc;
	}, {});
}

function findSelectedTransport(
	transports: Record<string, TransportRuleData> | null | undefined,
): SupportedTransportValue | null {
	for (const transport of SUPPORTED_TRANSPORT_VALUES) {
		if (transports?.[transport]?.selected === true) {
			return transport;
		}
	}
	return null;
}

function filterCurrentTransportPayload(
	currentTransports: Record<string, TransportRuleData> | null | undefined,
	supportedTransports: SupportedTransportValue[],
): Record<string, TransportRuleData> {
	if (!currentTransports) return {};
	const allowed = new Set(supportedTransports);
	return Object.fromEntries(
		Object.entries(currentTransports).filter(([transport]) => allowed.has(transport as SupportedTransportValue)),
	);
}

type ClientRecordFormValues = z.infer<ReturnType<typeof createFormSchema>>;

const CLIENT_FORM_ROW_LABEL_CLASS = "w-20 shrink-0 text-right";

function logoUrlIsPreviewable(value: string): boolean {
	const trimmed = value.trim();
	if (!trimmed) return false;
	return /^https?:\/\//i.test(trimmed) || /^data:image\//i.test(trimmed);
}

function extractErrorMessage(error: unknown): string {
	if (error instanceof Error && error.message.trim()) return error.message;
	if (typeof error === "string" && error.trim()) return error;
	try {
		return JSON.stringify(error);
	} catch {
		return String(error);
	}
}

function LogoUrlFieldWithPreview({
	label,
	placeholder,
	field,
}: {
	label: ReactNode;
	placeholder: string;
	field: ControllerRenderProps<ClientRecordFormValues, "logoUrl">;
}) {
	const [broken, setBroken] = useState(false);
	const trimmed = field.value?.trim() ?? "";

	useEffect(() => {
		setBroken(false);
	}, [trimmed]);

	const showImg = logoUrlIsPreviewable(trimmed) && !broken;

	return (
		<FormItem className="min-w-0 space-y-0">
			<div className="flex items-center gap-4">
				<FormLabel className={CLIENT_FORM_ROW_LABEL_CLASS}>{label}</FormLabel>
				<div className="min-w-0 flex-1">
					<div className="flex items-center gap-2">
						<div className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-md border border-input bg-muted">
							{showImg ? (
								<img
									src={trimmed}
									alt=""
									className="h-full w-full object-contain"
									referrerPolicy="no-referrer"
									onError={() => setBroken(true)}
								/>
							) : (
								<ImageIcon className="h-4 w-4 text-muted-foreground" aria-hidden />
							)}
						</div>
						<FormControl>
							<Input {...field} placeholder={placeholder} className="min-w-0" />
						</FormControl>
					</div>
					<FormMessage />
				</div>
			</div>
		</FormItem>
	);
}

function normalizeIdentifier(value: string): string {
	return value.trim().toLowerCase().replace(/\s+/g, "-");
}

function connectionShapeToMode(shape: ClientConnectionShape): ClientConnectionMode {
	switch (shape) {
		case "local_with_config":
			return "local_config_detected";
		case "remote_http":
			return "remote_http";
		default:
			return "manual";
	}
}

function connectionModeToShape(
	connectionMode: ClientConnectionMode | null | undefined,
	configPath?: string | null,
): ClientConnectionShape {
	if (connectionMode === "remote_http") return "remote_http";
	if (connectionMode === "local_config_detected" && configPath?.trim()) return "local_with_config";
	return "local_without_config";
}

function hasWritableConfig(values: Pick<ClientRecordFormValues, "connectionShape" | "configPath">): boolean {
	return values.connectionShape === "local_with_config" && Boolean(values.configPath?.trim());
}

function parseSupportedTransportValue(value: unknown): SupportedTransportValue | null {
	const candidate = String(value).trim().toLowerCase();
	if (!candidate) return null;
	if (candidate === "sse" || candidate === "stdio") return candidate;
	if (
		candidate === "streamable_http" ||
		candidate === "streamablehttp" ||
		candidate === "streamable-http" ||
		candidate === "http"
	) {
		return "streamable_http";
	}
	return null;
}

function normalizeSupportedTransports(value: unknown): SupportedTransportValue[] {
	const seen = new Set<SupportedTransportValue>();
	const normalized: SupportedTransportValue[] = [];

	for (const item of Array.isArray(value) ? value : []) {
		const transport = parseSupportedTransportValue(item);
		if (!transport || seen.has(transport)) continue;
		seen.add(transport);
		normalized.push(transport);
	}

	return normalized.sort(
		(left, right) =>
			SUPPORTED_TRANSPORT_VALUES.indexOf(left) - SUPPORTED_TRANSPORT_VALUES.indexOf(right),
	);
}

function normalizeConfigParseKeys(value: string | undefined): string[] {
	const seen = new Set<string>();
	const keys: string[] = [];

	for (const entry of (value ?? "").split(/[\n,]/)) {
		const trimmed = entry.trim();
		if (!trimmed || seen.has(trimmed)) continue;
		seen.add(trimmed);
		keys.push(trimmed);
	}

	return keys;
}

function parseDraftFromValues(
	values: Pick<
		ClientRecordFormValues,
		"configFileParseFormat" | "configFileParseContainerType" | "configFileParseContainerKeysText"
	>,
): ClientConfigFileParse {
	return {
		format: values.configFileParseFormat,
		container_type: values.configFileParseContainerType,
		container_keys: normalizeConfigParseKeys(values.configFileParseContainerKeysText),
	};
}

function inspectionPreviewText(preview: unknown): string {
	if (typeof preview === "string") return preview;
	try {
		return JSON.stringify(preview ?? null, null, 2);
	} catch {
		return String(preview ?? "");
	}
}

function getTransportSupportLabel(
	transport: SupportedTransportValue,
	t: ReturnType<typeof useTranslation>["t"],
): string {
	switch (transport) {
		case "streamable_http":
			return t("detail.form.transportSupport.options.streamableHttpLegacy", {
				defaultValue: "Streamable HTTP",
			});
		case "sse":
			return t("detail.form.transportSupport.options.sseLegacy", {
				defaultValue: "SSE (Legacy)",
			});
		case "stdio":
			return t("detail.form.transportSupport.options.stdio", { defaultValue: "STDIO" });
	}
}

const TransportSupportCombobox = React.forwardRef<
	HTMLButtonElement,
	{
		value: SupportedTransportValue[];
		onChange: (next: SupportedTransportValue[]) => void;
		options: Array<{ value: SupportedTransportValue; label: string }>;
		placeholder: string;
		emptyText: string;
	}
>(({ value, onChange, options, placeholder, emptyText }, ref) => {
	const [open, setOpen] = useState(false);
	const selectedLabels = options.filter((option) => value.includes(option.value)).map((option) => option.label);

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<Button ref={ref} type="button" variant="outline" role="combobox" aria-expanded={open} className="w-full justify-between">
					<span className="truncate text-left font-normal">{selectedLabels.length > 0 ? selectedLabels.join(", ") : placeholder}</span>
					<ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
				</Button>
			</PopoverTrigger>
			<PopoverContent align="start" className="w-[var(--radix-popover-trigger-width)] p-0">
				<Command>
					<CommandInput placeholder={placeholder} />
					<CommandList>
						<CommandEmpty>{emptyText}</CommandEmpty>
						<CommandGroup>
							{options.map((option) => {
								const selected = value.includes(option.value);
								return (
									<CommandItem
										key={option.value}
										value={option.label}
										onSelect={() => {
											const next = selected ? value.filter((item) => item !== option.value) : [...value, option.value];
											onChange(normalizeSupportedTransports(next));
										}}
									>
										<Check className={cn("mr-2 h-4 w-4", selected ? "opacity-100" : "opacity-0")} />
										{option.label}
									</CommandItem>
								);
							})}
						</CommandGroup>
					</CommandList>
				</Command>
			</PopoverContent>
		</Popover>
	);
});

TransportSupportCombobox.displayName = "TransportSupportCombobox";

function defaultValues(client?: ClientInfo | null): ClientRecordFormValues {
	const identifier = client?.identifier ?? "";
	const connectionShape = connectionModeToShape(client?.connection_mode, client?.config_path);
	const supportedTransports = (() => {
		if (!client) return ["streamable_http", "stdio"] satisfies SupportedTransportValue[];
		const fromRules = normalizeSupportedTransports(Object.keys(client?.transports ?? {}));
		if (fromRules.length > 0) return fromRules;
		if (client?.transport) return normalizeSupportedTransports([client.transport]);
		return [];
	})();
	const effectiveParse = resolveEffectiveClientParse(client);

	return {
		identifier,
		displayName: client?.display_name ?? "",
		connectionShape,
		supportedTransports,
		configPath: connectionShape === "local_with_config" ? client?.config_path || "" : "",
		configFileParseFormat: (effectiveParse?.format as ConfigParseFormatValue | undefined) ?? "json",
		configFileParseContainerType: effectiveParse?.container_type === "array" ? "array" : "standard",
		configFileParseContainerKeysText: effectiveParse?.container_keys?.join(", ") ?? "",
		clientVersion: client?.client_version ?? "",
		description: client?.description ?? "",
		homepageUrl: client?.homepage_url ?? "",
		docsUrl: client?.docs_url ?? "",
		supportUrl: client?.support_url ?? "",
		logoUrl: client?.logo_url ?? "",
	};
}

function TextInputRow({
	label,
	placeholder,
	field,
	disabled,
}: {
	label: string;
	placeholder: string;
	field: ControllerRenderProps<ClientRecordFormValues>;
	disabled?: boolean;
}) {
	return (
		<FormItem className="space-y-0">
			<div className="flex items-center gap-4">
				<FormLabel className={CLIENT_FORM_ROW_LABEL_CLASS}>{label}</FormLabel>
				<div className="min-w-0 flex-1">
					<FormControl>
						<Input {...field} disabled={disabled} placeholder={placeholder} />
					</FormControl>
					<FormMessage />
				</div>
			</div>
		</FormItem>
	);
}

function TransportRuleField({
	label,
	placeholder,
	value,
	onChange,
}: {
	label: string;
	placeholder: string;
	value: string;
	onChange: (next: string) => void;
}) {
	return (
		<label className="space-y-1.5">
			<span className="text-xs font-medium text-muted-foreground">{label}</span>
			<Input value={value} onChange={(event) => onChange(event.currentTarget.value)} placeholder={placeholder} className="h-8 text-sm" />
		</label>
	);
}

function ExtraFieldsEditor({
	label,
	entries,
	onChange,
	addLabel,
	keyPlaceholder,
	valuePlaceholder,
}: {
	label: string;
	entries: EditableExtraFieldEntry[];
	onChange: (next: EditableExtraFieldEntry[]) => void;
	addLabel: string;
	keyPlaceholder: string;
	valuePlaceholder: string;
}) {
	const rows = entries.length > 0 ? entries : [{ id: "empty", key: "", value: "" }];

	return (
		<div className="space-y-2">
			<div className="flex items-center justify-between gap-2">
				<p className="text-xs font-medium text-muted-foreground">{label}</p>
				<Button
					type="button"
					variant="outline"
					size="sm"
					className="h-7 gap-1 px-2"
					onClick={() => onChange([...entries, { id: createEditorId(), key: "", value: "" }])}
				>
					<Plus className="h-3 w-3" />
					{addLabel}
				</Button>
			</div>
			<div className="space-y-2">
				{rows.map((entry, index) => {
					const isGhost = entry.id === "empty";
					return (
						<div
							key={entry.id}
							className={`grid gap-2 ${isGhost ? "md:grid-cols-2" : "md:grid-cols-[1fr_1fr_auto]"}`}
						>
							<Input
								value={entry.key}
								onChange={(event) => {
									const next = isGhost ? [...entries, { id: createEditorId(), key: event.currentTarget.value, value: "" }] : [...entries];
									const targetIndex = isGhost ? next.length - 1 : index;
									next[targetIndex] = { ...next[targetIndex], key: event.currentTarget.value };
									onChange(next);
								}}
								placeholder={keyPlaceholder}
								className="h-8 text-sm"
							/>
							<Input
								value={entry.value}
								onChange={(event) => {
									const next = isGhost ? [...entries, { id: createEditorId(), key: "", value: event.currentTarget.value }] : [...entries];
									const targetIndex = isGhost ? next.length - 1 : index;
									next[targetIndex] = { ...next[targetIndex], value: event.currentTarget.value };
									onChange(next);
								}}
								placeholder={valuePlaceholder}
								className="h-8 text-sm"
							/>
							{isGhost ? null : (
								<Button
									type="button"
									variant="ghost"
									size="icon"
									className="h-8 w-8"
									onClick={() => onChange(entries.filter((item) => item.id !== entry.id))}
								>
									<Trash2 className="h-3.5 w-3.5" />
								</Button>
							)}
						</div>
					);
				})}
			</div>
		</div>
	);
}

export function ClientFormDrawer({
	open,
	onOpenChange,
	mode,
	client,
	onSuccess,
	onDeleteSuccess,
}: ClientFormDrawerProps) {
	const { t, i18n } = useTranslation("clients");
	const dashboardSettings = useAppStore((state) => state.dashboardSettings);
	const qc = useQueryClient();
	const formSchema = useMemo(() => createFormSchema(t), [t, i18n.language]);
	const [isHydrating, setIsHydrating] = useState(false);
	const [formError, setFormError] = useState<string | null>(null);
	const [deleteError, setDeleteError] = useState<string | null>(null);
	const [parseInspection, setParseInspection] = useState<ParseInspectionView | null>(null);
	const [parseInspectionError, setParseInspectionError] = useState<string | null>(null);
	const [isParseAdvancedOpen, setIsParseAdvancedOpen] = useState(false);
	const [showParseCodePreview, setShowParseCodePreview] = useState(false);
	const [isDeleteConfirmOpen, setIsDeleteConfirmOpen] = useState(false);
	const [configPathPickBusy, setConfigPathPickBusy] = useState(false);
	const [transportRuleEditors, setTransportRuleEditors] = useState<TransportRuleEditors>(() =>
		transportRuleEditorsFromClient(client),
	);
	const [selectedTransportTab, setSelectedTransportTab] = useState<SupportedTransportValue | "">("");
	const previousSupportedTransportsRef = useRef<SupportedTransportValue[]>([]);
	const initialSupportedTransportsRef = useRef<SupportedTransportValue[]>(
		defaultValues(client).supportedTransports,
	);
	const isTauriShell = useMemo(() => isTauriEnvironmentSync(), []);
	const configPathFileInputRef = useRef<HTMLInputElement>(null);
	const autoAppliedInferenceRef = useRef<string | null>(null);

	const form = useForm<ClientRecordFormValues>({
		resolver: zodResolver(formSchema),
		defaultValues: defaultValues(client),
	});

	const connectionShape = form.watch("connectionShape");
	const identifier = form.watch("identifier");
	const configPath = form.watch("configPath");
	const configFileParseFormat = form.watch("configFileParseFormat");
	const configFileParseContainerType = form.watch("configFileParseContainerType");
	const configFileParseContainerKeysText = form.watch("configFileParseContainerKeysText");
	const supportedTransports = form.watch("supportedTransports");
	const parseFieldsDirty = Boolean(
		form.formState.dirtyFields.configFileParseFormat ||
		form.formState.dirtyFields.configFileParseContainerType ||
		form.formState.dirtyFields.configFileParseContainerKeysText,
	);
	const parseDraft = useMemo(
		() =>
			parseDraftFromValues({
				configFileParseFormat,
				configFileParseContainerType,
				configFileParseContainerKeysText,
			}),
		[configFileParseFormat, configFileParseContainerType, configFileParseContainerKeysText],
	);
	const previewText = useMemo(
		() => inspectionPreviewText(parseInspection?.preview),
		[parseInspection?.preview],
	);
	const hasInspectablePreview = useMemo(() => {
		const preview = parseInspection?.preview;
		if (preview == null) return false;
		if (typeof preview === "string") return preview.trim().length > 0;
		if (typeof preview === "object") return Object.keys(preview as Record<string, unknown>).length > 0;
		return true;
	}, [parseInspection?.preview]);

	useEffect(() => {
		if (!open) return;
		setFormError(null);
		setDeleteError(null);
		setParseInspection(null);
		setParseInspectionError(null);
		setIsParseAdvancedOpen(false);
		setShowParseCodePreview(false);
		setIsDeleteConfirmOpen(false);
		autoAppliedInferenceRef.current = null;
		setTransportRuleEditors(transportRuleEditorsFromClient(client));
		setSelectedTransportTab("");
		initialSupportedTransportsRef.current = defaultValues(client).supportedTransports;
		setIsHydrating(true);
		form.reset(defaultValues(client));
		setIsHydrating(false);
	}, [open, client, mode, form]);

	useEffect(() => {
		if (supportedTransports.length === 0) {
			if (selectedTransportTab !== "") {
				setSelectedTransportTab("");
			}
			return;
		}

		if (!selectedTransportTab || !supportedTransports.includes(selectedTransportTab)) {
			setSelectedTransportTab(supportedTransports[0]);
		}
	}, [selectedTransportTab, supportedTransports]);

	useEffect(() => {
		if (isHydrating) {
			previousSupportedTransportsRef.current = supportedTransports;
			return;
		}

		const previousSet = new Set(previousSupportedTransportsRef.current);
		const added = supportedTransports.filter((transport) => !previousSet.has(transport));
		if (added.length === 0) {
			previousSupportedTransportsRef.current = supportedTransports;
			return;
		}

		setTransportRuleEditors((current) => {
			let changed = false;
			const next: TransportRuleEditors = { ...current };

			for (const transport of added) {
				const existing = current[transport] ?? createEmptyTransportRuleEditor();
				if (!isTransportRuleEditorEmpty(existing)) continue;
				const commonPreset = buildTransportRulePresets(transport, client, t).find((preset) => preset.id === "common");
				if (!commonPreset) continue;
				next[transport] = cloneTransportRuleEditorValue(commonPreset.value);
				changed = true;
			}

			return changed ? next : current;
		});

		previousSupportedTransportsRef.current = supportedTransports;
	}, [client, isHydrating, supportedTransports, t]);

	useEffect(() => {
		if (isHydrating || mode !== "create") return;
		const normalized = normalizeIdentifier(identifier ?? "");
		if (normalized && normalized !== identifier) {
			form.setValue("identifier", normalized, { shouldDirty: true });
		}
	}, [identifier, form, isHydrating, mode]);

	useEffect(() => {
		if (connectionShape !== "local_with_config") {
			if (form.getValues("configPath")) {
				form.setValue("configPath", "", { shouldDirty: true });
			}
			setParseInspection(null);
			setParseInspectionError(null);
			setShowParseCodePreview(false);
		}
	}, [connectionShape, form]);

	const connectionOptions: SegmentOption[] = useMemo(
		() => [
			{ value: "local_with_config", label: t("detail.form.connectionShape.options.localWithConfig", { defaultValue: "Local + Config" }) },
			{ value: "local_without_config", label: t("detail.form.connectionShape.options.localWithoutConfig", { defaultValue: "Local / Unknown Config" }) },
			{ value: "remote_http", label: t("detail.form.connectionShape.options.remoteHttp", { defaultValue: "Remote HTTP" }) },
		],
		[t],
	);
	const supportedTransportOptions = useMemo(
		() => SUPPORTED_TRANSPORT_VALUES.map((transport) => ({ value: transport, label: getTransportSupportLabel(transport, t) })),
		[t],
	);
	const configParseFormatOptions: SegmentOption[] = useMemo(
		() => CONFIG_PARSE_FORMAT_VALUES.map((value) => ({ value, label: value.toUpperCase() })),
		[],
	);
	const configParseContainerTypeOptions: SegmentOption[] = useMemo(
		() => [
			{ value: "standard", label: t("detail.form.fields.configFileParse.containerTypeOptions.standard", { defaultValue: "Object Map" }) },
			{ value: "array", label: t("detail.form.fields.configFileParse.containerTypeOptions.array", { defaultValue: "Array" }) },
		],
		[t],
	);
	const transportRuleValidationMessages = useMemo(
		() => [
			t("detail.form.transportRules.validation.commandRequired", {
				defaultValue: "STDIO requires a command field.",
			}),
			t("detail.form.transportRules.validation.urlRequired", {
				defaultValue: "HTTP-based transports require a URL field.",
			}),
			t("detail.form.transportRules.validation.typeValueRequired", {
				defaultValue: "Type value is required when including the type field.",
			}),
		],
		[t],
	);
	const isTransportRuleValidationError =
		Boolean(formError) && transportRuleValidationMessages.includes(formError ?? "");
	const applyInferredParseToForm = useCallback(
		(inferred: ClientConfigFileParse) => {
			form.setValue("configFileParseFormat", inferred.format as ConfigParseFormatValue, { shouldDirty: true });
			form.setValue("configFileParseContainerType", inferred.container_type === "array" ? "array" : "standard", {
				shouldDirty: true,
			});
			form.setValue("configFileParseContainerKeysText", inferred.container_keys?.join(", ") ?? "", {
				shouldDirty: true,
			});
		},
		[form],
	);
	const updateTransportRuleEditor = useCallback(
		(
			transport: SupportedTransportValue,
			updater: (current: TransportRuleEditorValue) => TransportRuleEditorValue,
		) => {
			setTransportRuleEditors((current) => ({
				...current,
				[transport]: updater(current[transport] ?? createEmptyTransportRuleEditor()),
			}));
		},
		[],
	);
	const applyTransportRulePreset = useCallback((transport: SupportedTransportValue, preset: TransportRulePreset) => {
		updateTransportRuleEditor(transport, () => cloneTransportRuleEditorValue(preset.value));
	}, [updateTransportRuleEditor]);
	const transportRulesHelpHref =
		client?.docs_url ?? client?.homepage_url ?? client?.template?.docs_url ?? client?.template?.homepage_url ?? null;

	const inspectMutation = useMutation({
		mutationFn: async (
			payload:
				| { kind: "create"; request: ClientConfigFileParseInspectReq }
				| { kind: "existing"; request: ClientConfigFileParseInspectExistingReq },
		) =>
			payload.kind === "existing"
				? clientsApi.inspectExistingClientConfigFileParse(payload.request)
				: clientsApi.inspectConfigFileParse(payload.request),
		onSuccess: (data) => {
			if (!data) {
				setParseInspection(null);
				setParseInspectionError(null);
				return;
			}
			setParseInspection(data);
			setParseInspectionError(null);

			const inferred = data.inferred_parse;
			const currentPath = form.getValues("configPath")?.trim();
			if (!inferred || !currentPath || parseFieldsDirty) return;

			const signature = `${currentPath}:${JSON.stringify(inferred)}`;
			if (autoAppliedInferenceRef.current === signature) return;

			autoAppliedInferenceRef.current = signature;
			applyInferredParseToForm(inferred);
		},
		onError: (error) => {
			setParseInspection(null);
			setParseInspectionError(extractErrorMessage(error));
		},
	});

	useEffect(() => {
		const validationFailed =
			parseInspectionError ||
			(parseInspection?.validation && !parseInspection.validation.matches) ||
			(parseInspection && !parseInspection.inferred_parse && !inspectMutation.isPending);
		if (validationFailed) {
			setIsParseAdvancedOpen(true);
		}
	}, [inspectMutation.isPending, parseInspection, parseInspectionError]);

	useEffect(() => {
		if (!open || connectionShape !== "local_with_config") return;
		const trimmedPath = configPath?.trim();
		if (!trimmedPath) {
			setParseInspection(null);
			setParseInspectionError(null);
			setShowParseCodePreview(false);
			return;
		}

		const persistedPath = client?.config_path?.trim() ?? "";
		const canInspectExistingClientPath =
			mode === "edit" && Boolean(identifier?.trim()) && trimmedPath === persistedPath;
		const canInspectCreatePath = mode === "create";
		if (!canInspectExistingClientPath && !canInspectCreatePath) {
			setParseInspection(null);
			setParseInspectionError(
				mode === "edit" && trimmedPath !== persistedPath
					? t("detail.form.fields.configFileParse.inspectAfterSaveHint", {
						defaultValue:
							"Save the updated config path first, then MCPMate can inspect the stored target for this client.",
					})
					: null,
			);
			setShowParseCodePreview(false);
			return;
		}

		const timer = window.setTimeout(() => {
			if (canInspectExistingClientPath && identifier?.trim()) {
				void inspectMutation.mutateAsync({
					kind: "existing",
					request: {
						identifier,
						config_file_parse:
							(parseDraft.container_keys?.length ?? 0) > 0 ? parseDraft : undefined,
					},
				});
				return;
			}

			void inspectMutation.mutateAsync({
				kind: "create",
				request: {
					config_path: trimmedPath,
					config_file_parse:
						(parseDraft.container_keys?.length ?? 0) > 0 ? parseDraft : undefined,
				},
			});
		}, 350);

		return () => window.clearTimeout(timer);
	}, [open, connectionShape, configPath, parseDraft, inspectMutation, mode, identifier, client?.config_path, t]);

	const handleApplyDetectedRules = useCallback(() => {
		const inferred = parseInspection?.inferred_parse;
		if (!inferred) return;
		applyInferredParseToForm(inferred);
	}, [applyInferredParseToForm, parseInspection?.inferred_parse]);

	const handleConfigPathBrowse = useCallback(async () => {
		if (!isTauriShell) return;
		setConfigPathPickBusy(true);
		try {
			const path = await pickClientConfigFilePath(
				t("detail.form.fields.configPath.dialogTitle", { defaultValue: "Select MCP configuration file" }),
			);
			if (path) {
				autoAppliedInferenceRef.current = null;
				form.setValue("configPath", path, { shouldDirty: true, shouldTouch: true, shouldValidate: true });
			}
		} catch (error) {
			notifyError(t("detail.form.fields.configPath.pickFailedTitle", { defaultValue: "Could not open file dialog" }), extractErrorMessage(error));
		} finally {
			setConfigPathPickBusy(false);
		}
	}, [form, isTauriShell, t]);

	const handleConfigPathWebFileChange = useCallback(
		(event: React.ChangeEvent<HTMLInputElement>) => {
			const input = event.currentTarget;
			try {
				const file = input.files?.[0];
				if (!file) return;
				const path = readAbsolutePathFromFile(file);
				if (path) {
					autoAppliedInferenceRef.current = null;
					form.setValue("configPath", path, { shouldDirty: true, shouldTouch: true, shouldValidate: true });
					return;
				}
				notifyInfo(
					t("detail.form.fields.configPath.webPickInfoTitle", { defaultValue: "Could not read file path" }),
					t("detail.form.fields.configPath.webPickInfoDescription", {
						defaultValue:
							"Standard browsers hide full file paths for security. Enter the absolute path manually, or use MCPMate Desktop; the same button opens the native file picker there.",
					}),
					undefined,
					true,
				);
			} finally {
				input.value = "";
			}
		},
		[form, t],
	);

	const handleConfigPathBrowseButton = useCallback(() => {
		if (isTauriShell) {
			void handleConfigPathBrowse();
			return;
		}
		configPathFileInputRef.current?.click();
	}, [handleConfigPathBrowse, isTauriShell]);

	const saveMutation = useMutation({
		mutationFn: async () => {
			const values = form.getValues();
			const normalizedIdentifier = normalizeIdentifier(values.identifier);
			const parseForSave = parseDraftFromValues(values);
			const hasWritableRules = hasWritableConfig(values);
			const supportedTransportsChanged = !isSameSupportedTransports(
				initialSupportedTransportsRef.current,
				values.supportedTransports,
			);
			const transportEditorsChanged =
				hasWritableRules &&
				transportRuleEditorsSignature(transportRuleEditors) !==
				transportRuleEditorsSignature(transportRuleEditorsFromClient(client));
			const shouldPersistTransports =
				mode === "create" || supportedTransportsChanged || transportEditorsChanged;
			const transports = shouldPersistTransports
				? mode === "edit" && !supportedTransportsChanged && !transportEditorsChanged
					? (client?.transports ?? undefined)
					: mode === "edit" && !hasWritableRules
						? filterCurrentTransportPayload(client?.transports, values.supportedTransports)
						: buildTransportRulesPayload(
							values.supportedTransports,
							transportRuleEditors,
							client,
							t,
							hasWritableRules,
						)
				: undefined;
			if (hasWritableConfig(values) && (parseForSave.container_keys?.length ?? 0) === 0) {
				throw new Error(
					t("detail.form.fields.configFileParse.keysRequired", {
						defaultValue: "Add at least one config node path before saving parse rules.",
					}),
				);
			}

			await clientsApi.update({
				identifier: normalizedIdentifier,
				display_name: values.displayName || undefined,
				connection_mode: connectionShapeToMode(values.connectionShape),
				config_path: hasWritableConfig(values) ? values.configPath?.trim() || undefined : undefined,
				client_version: values.clientVersion?.trim() || undefined,
				description: values.description || undefined,
				homepage_url: values.homepageUrl || undefined,
				docs_url: values.docsUrl || undefined,
				support_url: values.supportUrl || undefined,
				logo_url: values.logoUrl || undefined,
				config_file_parse: hasWritableConfig(values) ? parseForSave : undefined,
				transports,
			});

			if (
				mode === "edit" &&
				!isSameSupportedTransports(
					initialSupportedTransportsRef.current,
					values.supportedTransports,
				)
			) {
				const details = await clientsApi.configDetails(normalizedIdentifier, false);
				const configMode = resolveClientConfigMode(
					details?.config_mode ?? client?.config_mode,
				);
				if (
					canApplyClientConfigWithState({
						mode: configMode,
						writableConfig: details?.writable_config,
						approvalStatus: details?.approval_status,
					}) &&
					configMode
				) {
					try {
						await applyClientConfigWithResolvedSelection({
							identifier: normalizedIdentifier,
							mode: configMode,
							backupPolicy: mapDashboardSettingsToClientBackupPolicy(
								dashboardSettings,
							),
						});
					} catch (error) {
						notifyError(
							t("detail.notifications.applyFailed.title", {
								defaultValue: "Apply failed",
							}),
							resolveClientConfigSyncErrorMessage(error, t),
						);
					}
				}
			}

			if (mode === "create") {
				try {
					await clientsApi.setBackupPolicy({
						identifier: normalizedIdentifier,
						policy: mapDashboardSettingsToClientBackupPolicy(dashboardSettings),
					});
				} catch {
					notifyError(
						t("detail.form.notifications.saveFailed.title", { defaultValue: "Unable to save client record" }),
						t("detail.form.notifications.createBackupPolicyFailed.message", {
							defaultValue:
								"Client record was created, but applying initial backup policy failed. You can retry in Backup settings.",
						}),
					);
				}
			}

			return normalizedIdentifier;
		},
		// transportRuleEditors is outside react-hook-form and must participate explicitly
		// in mutation closure updates.
		onSuccess: async (savedIdentifier) => {
			setFormError(null);
			await qc.invalidateQueries({ queryKey: ["clients"] });
			await qc.invalidateQueries({ queryKey: ["client-config", savedIdentifier] });
			await qc.invalidateQueries({ queryKey: ["client-capability-config", savedIdentifier] });
			notifySuccess(
				mode === "create"
					? t("detail.form.notifications.createSuccess.title", { defaultValue: "Client record created" })
					: t("detail.form.notifications.editSuccess.title", { defaultValue: "Client record updated" }),
				mode === "create"
					? t("detail.form.notifications.createSuccess.message", { defaultValue: "The client record has been created." })
					: t("detail.form.notifications.editSuccess.message", { defaultValue: "The client record has been updated." }),
			);
			onOpenChange(false);
			onSuccess?.(savedIdentifier);
		},
		onError: (error) => {
			const message = extractErrorMessage(error);
			setFormError(message);
			notifyError(t("detail.form.notifications.saveFailed.title", { defaultValue: "Unable to save client record" }), message);
		},
	});

	const deleteMutation = useMutation({
		mutationFn: async () => {
			if (!client?.identifier) {
				throw new Error(
					t("detail.form.notifications.deleteFailed.messageMissingIdentifier", {
						defaultValue: "Client identifier is missing.",
					}),
				);
			}
			await clientsApi.deleteRecord(client.identifier);
			return client.identifier;
		},
		onSuccess: async (deletedIdentifier) => {
			setDeleteError(null);
			setIsDeleteConfirmOpen(false);
			await qc.invalidateQueries({ queryKey: ["clients"] });
			await qc.invalidateQueries({ queryKey: ["client-config", deletedIdentifier] });
			await qc.invalidateQueries({ queryKey: ["client-capability-config", deletedIdentifier] });
			notifySuccess(
				t("detail.form.notifications.deleteSuccess.title", { defaultValue: "Client record deleted" }),
				t("detail.form.notifications.deleteSuccess.message", { defaultValue: "The client record has been deleted." }),
			);
			onOpenChange(false);
			onDeleteSuccess?.(deletedIdentifier);
		},
		onError: (error) => {
			const message = extractErrorMessage(error);
			setDeleteError(message);
			notifyError(t("detail.form.notifications.deleteFailed.title", { defaultValue: "Unable to delete client record" }), message);
		},
	});

	return (
		<Drawer open={open} onOpenChange={onOpenChange}>
			<DrawerContent>
				<DrawerHeader>
					<DrawerTitle>
						{mode === "create"
							? t("detail.form.titleCreate", { defaultValue: "Add Client Record" })
							: t("detail.form.titleEdit", { defaultValue: "Edit Client Record" })}
					</DrawerTitle>
					<DrawerDescription>
						{mode === "create"
							? t("detail.form.descriptionCreate", { defaultValue: "Create a client record with its management shape and metadata." })
							: t("detail.form.descriptionEdit", { defaultValue: "Update this client record and its management settings." })}
					</DrawerDescription>
				</DrawerHeader>

				<Form {...form}>
					<form className="space-y-4 px-4 py-4">
						<Tabs defaultValue="basic" className="w-full">
							<TabsList className="grid w-full grid-cols-2">
								<TabsTrigger value="basic">{t("detail.form.tabs.basic", { defaultValue: "Basic" })}</TabsTrigger>
								<TabsTrigger value="meta">{t("detail.form.tabs.meta", { defaultValue: "Meta" })}</TabsTrigger>
							</TabsList>

							<TabsContent value="basic" className="space-y-4 pt-4">
								<div className="space-y-4">
									<FormField control={form.control} name="displayName" render={({ field }) => (
										<TextInputRow label={t("detail.form.fields.displayName.label", { defaultValue: "Client Name" })} placeholder={t("detail.form.fields.displayName.placeholder", { defaultValue: "Cursor Desktop" })} field={field} />
									)} />
									<FormField control={form.control} name="identifier" render={({ field }) => (
										<TextInputRow label={t("detail.form.fields.identifier.label", { defaultValue: "Client ID" })} placeholder={t("detail.form.fields.identifier.placeholder", { defaultValue: "cursor-desktop" })} field={field} disabled={mode !== "create"} />
									)} />
									<FormField control={form.control} name="clientVersion" render={({ field }) => (
										<TextInputRow label={t("detail.form.fields.clientVersion.label", { defaultValue: "Client Version" })} placeholder={t("detail.form.fields.clientVersion.placeholder", { defaultValue: "optional" })} field={field} />
									)} />
								</div>

								{mode === "create" ? (
									<p className="pl-24 text-sm text-muted-foreground">
										{t("detail.form.fields.identifier.description", {
											defaultValue:
												"Spaces and casing are normalized automatically when creating a new record.",
										})}
									</p>
								) : null}

								<FormField control={form.control} name="connectionShape" render={({ field }) => (
									<FormItem className="space-y-0">
										<div className="flex items-start gap-4">
											<FormLabel className={`${CLIENT_FORM_ROW_LABEL_CLASS} pt-2`}>
												{t("detail.form.connectionShape.label", { defaultValue: "Client Shape" })}
											</FormLabel>
											<div className="min-w-0 flex-1">
												<FormControl>
													<Segment value={field.value} onValueChange={field.onChange} options={connectionOptions} showDots={false} />
												</FormControl>
												<FormDescription>
													{t("detail.form.connectionShape.description", { defaultValue: "Choose whether this client has a writable local config file or is a non-writable remote/unknown client." })}
												</FormDescription>
												<FormMessage />
											</div>
										</div>
									</FormItem>
								)} />

								{connectionShape === "local_with_config" ? (
									<FormField control={form.control} name="supportedTransports" render={({ field }) => (
										<FormItem className="space-y-0">
											<div className="flex items-start gap-4">
												<FormLabel className={`${CLIENT_FORM_ROW_LABEL_CLASS} pt-2`}>{t("detail.form.transportSupport.label", { defaultValue: "Transport Support" })}</FormLabel>
												<div className="min-w-0 flex-1 space-y-2">
													<FormControl><TransportSupportCombobox value={field.value} onChange={field.onChange} options={supportedTransportOptions} placeholder={t("detail.form.transportSupport.placeholder", { defaultValue: "Select supported transports" })} emptyText={t("detail.form.transportSupport.empty", { defaultValue: "No transports found." })} /></FormControl>
													<FormDescription>{t("detail.form.transportSupport.description", { defaultValue: "Choose which runtime transports this client supports. This array is the only source used to constrain hosted/unify transport selection." })}</FormDescription>
													<FormMessage />
												</div>
											</div>
										</FormItem>
									)} />
								) : null}

								{connectionShape === "local_with_config" ? (
									<>
										<FormField control={form.control} name="configPath" render={({ field }) => (
											<FormItem className="space-y-0">
												<div className="flex items-start gap-4">
													<FormLabel className={`${CLIENT_FORM_ROW_LABEL_CLASS} pt-2`}>
														{t("detail.form.fields.configPath.label", { defaultValue: "Config File Path" })}
													</FormLabel>
													<div className="min-w-0 flex-1">
														<input ref={configPathFileInputRef} type="file" className="hidden" accept=".json,.json5,.yaml,.yml,.toml,application/json,text/yaml" aria-hidden tabIndex={-1} onChange={handleConfigPathWebFileChange} />
														<div className="flex w-full gap-2">
															<FormControl>
																<Input {...field} autoComplete="off" spellCheck={false} placeholder={t("detail.form.fields.configPath.placeholder", { defaultValue: "~/.cursor/mcp.json" })} className="min-w-0 flex-1 font-mono text-sm" />
															</FormControl>
															<Button type="button" variant="outline" className="shrink-0 gap-2" disabled={configPathPickBusy || saveMutation.isPending} onClick={handleConfigPathBrowseButton} aria-label={t("detail.form.fields.configPath.browseAria", { defaultValue: "Browse for configuration file on disk" })}>
																<FolderOpen className="h-4 w-4 shrink-0" aria-hidden />
																<span>{t("detail.form.fields.configPath.browse", { defaultValue: "Choose…" })}</span>
															</Button>
														</div>
														<FormDescription>
															{t("detail.form.fields.configPath.description", { defaultValue: "A writable local config path enables MCPMate to manage this client through file-based configuration operations." })}
														</FormDescription>
														<FormMessage />
													</div>
												</div>
											</FormItem>
										)} />

										<div className="ml-24 space-y-3 rounded-lg border border-dashed bg-muted/20 p-3">
											<div className="space-y-2">
												<div className="flex items-center justify-between gap-2">
													<p className="font-medium">{t("detail.form.fields.configFileParse.label", { defaultValue: "Parse Rules" })}</p>
													<div className="flex items-center gap-1">
														{parseInspection?.inferred_parse ? (
															<Button
																type="button"
																variant="outline"
																size="icon"
																className="h-7 w-7"
																onClick={handleApplyDetectedRules}
																aria-label={t("detail.form.fields.configFileParse.applyDetected", { defaultValue: "Use detected rules" })}
																title={t("detail.form.fields.configFileParse.applyDetected", { defaultValue: "Use detected rules" })}
															>
																<Sparkles className="h-3 w-3" />
															</Button>
														) : null}
														<Button
															type="button"
															variant="outline"
															size="icon"
															className="h-7 w-7"
															onClick={() => setIsParseAdvancedOpen((value) => !value)}
															aria-label={isParseAdvancedOpen
																? t("detail.form.fields.configFileParse.hideAdvanced", { defaultValue: "Hide details" })
																: t("detail.form.fields.configFileParse.showAdvanced", { defaultValue: "Show details" })}
															title={isParseAdvancedOpen
																? t("detail.form.fields.configFileParse.hideAdvanced", { defaultValue: "Hide details" })
																: t("detail.form.fields.configFileParse.showAdvanced", { defaultValue: "Show details" })}
														>
															<ChevronDown className={`h-3.5 w-3.5 transition-transform ${isParseAdvancedOpen ? "rotate-180" : "rotate-0"}`} />
														</Button>
													</div>
												</div>
												<div className="flex items-start justify-between gap-3 border-t pt-2 text-xs text-muted-foreground">
													<div className="min-w-0 flex-1">
														{showParseCodePreview && hasInspectablePreview ? (
															<div className="space-y-2">
																<div className="flex items-center gap-2">
																	<span>{t("detail.form.fields.configFileParse.previewTitle", { defaultValue: "Detected config snippet" })}</span>
																	{parseInspection?.detected_format ? (
																		<span className="rounded border px-1.5 py-0.5 uppercase tracking-wide">{parseInspection.detected_format}</span>
																	) : null}
																</div>
																<pre className="overflow-x-auto rounded-md px-3 py-2 text-xs whitespace-pre-wrap break-words">{previewText}</pre>
															</div>
														) : (
															<div className="space-y-1">
																<div className="flex items-center gap-2">
																	{inspectMutation.isPending ? (
																		<Loader2 className="h-3.5 w-3.5 animate-spin" />
																	) : parseInspectionError ? (
																		<AlertCircle className="h-3.5 w-3.5 text-destructive" />
																	) : parseInspection?.validation?.matches ? (
																		<CheckCircle2 className="h-3.5 w-3.5 text-emerald-600" />
																	) : (
																		<AlertCircle className="h-3.5 w-3.5 text-amber-600" />
																	)}
																	<span>{t("detail.form.fields.configFileParse.validationTitle", { defaultValue: "File association check" })}</span>
																</div>
																<p className="truncate">
																	{parseInspectionError
																		? parseInspectionError
																		: parseInspection?.validation?.matches
																			? t("detail.form.fields.configFileParse.validationSuccess", { defaultValue: "The selected file matches the current parse rules." })
																			: t("detail.form.fields.configFileParse.validationHint", { defaultValue: "Pick a config file and MCPMate will validate whether these rules can find MCP server entries." })}
																</p>
																{parseInspection?.validation ? (
																	<p>
																		{t("detail.form.fields.configFileParse.detectedFormat", { defaultValue: "Detected format" })}: {parseInspection.detected_format ?? "-"} · {t("detail.form.fields.configFileParse.containerMatch", { defaultValue: "Container" })}: {parseInspection.validation.container_found ? t("detail.form.fields.configFileParse.matchYes", { defaultValue: "Found" }) : t("detail.form.fields.configFileParse.matchNo", { defaultValue: "Missing" })} · {t("detail.form.fields.configFileParse.serverCount", { defaultValue: "Servers" })}: {parseInspection.validation.server_count}
																	</p>
																) : null}
															</div>
														)}
													</div>
													<Button
														type="button"
														variant="ghost"
														size="icon"
														className="h-7 w-7 shrink-0"
														disabled={!parseInspection || !hasInspectablePreview}
														onClick={() => setShowParseCodePreview((value) => !value)}
														aria-label={showParseCodePreview
															? t("detail.form.fields.configFileParse.summaryViewButton", { defaultValue: "Summary view" })
															: t("detail.form.fields.configFileParse.codeViewButton", { defaultValue: "Code preview" })}
													>
														<Code2 className="h-4 w-4" />
													</Button>
												</div>
											</div>

											{isParseAdvancedOpen ? (
												<div className="rounded-md border bg-white/80 dark:bg-slate-950/10">
													<div className="grid gap-3 px-3 py-3 md:grid-cols-2">
														<FormField control={form.control} name="configFileParseFormat" render={({ field }) => (
															<FormItem className="space-y-1.5"><FormLabel className="text-xs font-medium">{t("detail.form.fields.configFileParse.formatLabel", { defaultValue: "Config Format" })}</FormLabel><FormControl><Segment value={field.value} onValueChange={field.onChange} options={configParseFormatOptions} showDots={false} /></FormControl><FormMessage /></FormItem>
														)} />
														<FormField control={form.control} name="configFileParseContainerType" render={({ field }) => (
															<FormItem className="space-y-1.5"><FormLabel className="text-xs font-medium">{t("detail.form.fields.configFileParse.containerTypeLabel", { defaultValue: "Container Type" })}</FormLabel><FormControl><Segment value={field.value} onValueChange={field.onChange} options={configParseContainerTypeOptions} showDots={false} /></FormControl><FormMessage /></FormItem>
														)} />
														<FormField
															control={form.control}
															name="configFileParseContainerKeysText"
															render={({ field }) => (
																<FormItem className="space-y-1.5 md:col-span-2">
																	<FormLabel className="text-xs font-medium">
																		{t("detail.form.fields.configFileParse.containerKeysLabel", { defaultValue: "Config Nodes" })}
																	</FormLabel>
																	<FormControl>
																		<Input
																			{...field}
																			className="h-8 text-sm"
																			placeholder={t("detail.form.fields.configFileParse.containerKeysPlaceholder", {
																				defaultValue: "mcpServers, context_servers",
																			})}
																		/>
																	</FormControl>
																	<FormMessage />
																</FormItem>
															)}
														/>
														<div className="space-y-3 border-t border-dashed pt-3 md:col-span-2">
															<div className="flex items-center gap-1.5">
																<FormLabel className="text-xs font-medium">{t("detail.form.transportRules.label", { defaultValue: "Transport Rules" })}</FormLabel>
																<TooltipProvider delayDuration={200}>
																	<Tooltip>
																		<TooltipTrigger asChild>
																			<Button type="button" variant="ghost" size="icon" className="h-5 w-5 rounded-full text-muted-foreground hover:text-foreground">
																				<Info className="h-3.5 w-3.5" />
																			</Button>
																		</TooltipTrigger>
																		<TooltipContent side="top" align="start" className="max-w-sm space-y-2 leading-relaxed">
																			<p>{t("detail.form.transportRules.help.summary", { defaultValue: "These fields describe the target client's config keys, not MCPMate's own protocol fields." })}</p>
																			<p>{t("detail.form.transportRules.help.docs", { defaultValue: "If you are unsure which keys a client expects, check that client's official documentation or an existing working config file first." })}</p>
																			<p>{t("detail.form.transportRules.help.presets", { defaultValue: "Use the suggested variants below as a starting point, then verify the result against the client's real config structure." })}</p>
																			{transportRulesHelpHref ? (
																				<a href={transportRulesHelpHref} target="_blank" rel="noopener noreferrer" className="inline-flex text-primary hover:underline">
																					{t("detail.form.transportRules.help.openDocs", { defaultValue: "Open client documentation" })}
																				</a>
																			) : null}
																		</TooltipContent>
																	</Tooltip>
																</TooltipProvider>
															</div>
															{supportedTransports.length > 0 ? (
																<Tabs value={selectedTransportTab} onValueChange={(value) => setSelectedTransportTab(value as SupportedTransportValue)} className="space-y-3">
																	<TabsList className={`grid w-full ${supportedTransports.length === 1 ? "grid-cols-1" : supportedTransports.length === 2 ? "grid-cols-2" : "grid-cols-3"}`}>
																		{supportedTransports.map((transport) => (
																			<TabsTrigger key={transport} value={transport}>{getTransportSupportLabel(transport, t)}</TabsTrigger>
																		))}
																	</TabsList>
																	{supportedTransports.map((transport) => {
																		const editor = transportRuleEditors[transport] ?? createEmptyTransportRuleEditor();
																		const presets = buildTransportRulePresets(transport, client, t);
																		return (
																			<TabsContent key={transport} value={transport} className="mt-0">
																				<div className="space-y-3">
																					<div className="flex flex-wrap items-center gap-2 rounded-md border border-dashed bg-muted/20 px-3 py-2">
																						<span className="text-xs font-medium text-muted-foreground">{t("detail.form.transportRules.suggestedVariants", { defaultValue: "Suggested variants" })}</span>
																						{presets.map((preset) => (
																							<Button key={preset.id} type="button" variant="outline" size="sm" className="h-7 px-2 text-xs" onClick={() => applyTransportRulePreset(transport, preset)}>
																								{preset.label}
																							</Button>
																						))}
																					</div>
																					<div className="grid gap-3 md:grid-cols-2">
																						{transport === "stdio" ? (
																							<>
																								<TransportRuleField label={t("detail.form.transportRules.commandField", { defaultValue: "Command Field" })} placeholder="command" value={editor.commandField} onChange={(next) => updateTransportRuleEditor(transport, (current) => ({ ...current, commandField: next }))} />
																								<TransportRuleField label={t("detail.form.transportRules.argsField", { defaultValue: "Args Field" })} placeholder="args" value={editor.argsField} onChange={(next) => updateTransportRuleEditor(transport, (current) => ({ ...current, argsField: next }))} />
																								<TransportRuleField label={t("detail.form.transportRules.envField", { defaultValue: "Env Field" })} placeholder="env" value={editor.envField} onChange={(next) => updateTransportRuleEditor(transport, (current) => ({ ...current, envField: next }))} />
																							</>
																						) : (
																							<>
																								<TransportRuleField label={t("detail.form.transportRules.urlField", { defaultValue: "URL Field" })} placeholder="url" value={editor.urlField} onChange={(next) => updateTransportRuleEditor(transport, (current) => ({ ...current, urlField: next }))} />
																								<TransportRuleField label={t("detail.form.transportRules.headersField", { defaultValue: "Headers Field" })} placeholder="headers" value={editor.headersField} onChange={(next) => updateTransportRuleEditor(transport, (current) => ({ ...current, headersField: next }))} />
																							</>
																						)}
																						<div className="space-y-2 md:col-span-2 rounded-md border px-3 py-3">
																							<label className="flex items-center gap-2 text-sm font-medium">
																								<input type="checkbox" checked={editor.includeType} onChange={(event) => updateTransportRuleEditor(transport, (current) => ({ ...current, includeType: event.currentTarget.checked, typeValue: current.typeValue || transport }))} />
																								<span>{t("detail.form.transportRules.includeType", { defaultValue: "Include type field" })}</span>
																							</label>
																							{editor.includeType ? <TransportRuleField label={t("detail.form.transportRules.typeValue", { defaultValue: "Type Value" })} placeholder={transport} value={editor.typeValue} onChange={(next) => updateTransportRuleEditor(transport, (current) => ({ ...current, typeValue: next }))} /> : null}
																						</div>
																						<div className="md:col-span-2">
																							<ExtraFieldsEditor label={t("detail.form.transportRules.extraFields", { defaultValue: "Extra Fields" })} entries={editor.extraFields} onChange={(next) => updateTransportRuleEditor(transport, (current) => ({ ...current, extraFields: next }))} addLabel={t("detail.form.transportRules.addExtraField", { defaultValue: "Add field" })} keyPlaceholder={t("detail.form.transportRules.extraFieldKeyPlaceholder", { defaultValue: "enabled" })} valuePlaceholder={t("detail.form.transportRules.extraFieldValuePlaceholder", { defaultValue: "true or \"custom\"" })} />
																						</div>
																					</div>
																				</div>
																			</TabsContent>
																		);
																	})}
																</Tabs>
															) : (
																<div className="rounded-md border border-dashed px-3 py-2 text-sm text-muted-foreground">
																	{t("detail.form.transportRules.empty", { defaultValue: "Select at least one transport to edit its write rules." })}
																</div>
															)}
														</div>
														{formError && isTransportRuleValidationError ? (
															<div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive md:col-span-2">
																{formError}
															</div>
														) : null}
													</div>
												</div>
											) : null}

										</div>

									</>
								) : null}
							</TabsContent>

							<TabsContent value="meta" className="space-y-4 pt-4">
								<div className="space-y-4">
									<FormField control={form.control} name="logoUrl" render={({ field }) => (
										<LogoUrlFieldWithPreview field={field} label={t("detail.form.fields.logoUrl.label", { defaultValue: "Logo URL" })} placeholder={t("detail.form.fields.logoUrl.placeholder", { defaultValue: "https://example.com/logo.png" })} />
									)} />
									<FormField control={form.control} name="homepageUrl" render={({ field }) => (
										<TextInputRow label={t("detail.form.fields.homepageUrl.label", { defaultValue: "Homepage URL" })} placeholder={t("detail.form.fields.homepageUrl.placeholder", { defaultValue: "https://example.com" })} field={field} />
									)} />
									<FormField control={form.control} name="docsUrl" render={({ field }) => (
										<TextInputRow label={t("detail.form.fields.docsUrl.label", { defaultValue: "Docs URL" })} placeholder={t("detail.form.fields.docsUrl.placeholder", { defaultValue: "https://docs.example.com" })} field={field} />
									)} />
									<FormField control={form.control} name="supportUrl" render={({ field }) => (
										<TextInputRow label={t("detail.form.fields.supportUrl.label", { defaultValue: "Support URL" })} placeholder={t("detail.form.fields.supportUrl.placeholder", { defaultValue: "https://support.example.com" })} field={field} />
									)} />
								</div>
								<FormField control={form.control} name="description" render={({ field }) => (
									<FormItem className="space-y-0">
										<div className="flex items-start gap-4">
											<FormLabel className={`${CLIENT_FORM_ROW_LABEL_CLASS} pt-3`}>{t("detail.form.fields.description.label", { defaultValue: "Description" })}</FormLabel>
											<div className="min-w-0 flex-1">
												<FormControl><Textarea {...field} rows={4} placeholder={t("detail.form.fields.description.placeholder", { defaultValue: "A short summary of this client." })} /></FormControl>
												<FormDescription>{t("detail.form.fields.description.description", { defaultValue: "These meta fields are stored for display and guidance; the old template files remain only as compatibility seeds." })}</FormDescription>
												<FormMessage />
											</div>
										</div>
									</FormItem>
								)} />
							</TabsContent>
						</Tabs>

						{formError && !isTransportRuleValidationError ? <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">{formError}</div> : null}
					</form>
				</Form>

				<DrawerFooter className="mt-auto border-t px-6 py-4">
					<div className="flex w-full items-center justify-between gap-3">
						<Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={saveMutation.isPending || deleteMutation.isPending}>{t("detail.form.buttons.cancel", { defaultValue: "Cancel" })}</Button>
						<div className="flex items-center gap-3">
							{mode === "edit" ? (
								<Button type="button" variant="destructive" className="gap-2" onClick={() => { setDeleteError(null); setIsDeleteConfirmOpen(true); }} disabled={saveMutation.isPending || deleteMutation.isPending}><Trash2 className="h-4 w-4" />{t("detail.form.buttons.delete", { defaultValue: "Delete" })}</Button>
							) : null}
							<Button type="button" onClick={form.handleSubmit(() => saveMutation.mutate())} disabled={saveMutation.isPending || deleteMutation.isPending}>{mode === "create" ? t("detail.form.buttons.create", { defaultValue: "Create Record" }) : t("detail.form.buttons.save", { defaultValue: "Save Changes" })}</Button>
						</div>
					</div>
				</DrawerFooter>
				<ConfirmDialog
					isOpen={isDeleteConfirmOpen}
					onClose={() => {
						if (deleteMutation.isPending) return;
						setIsDeleteConfirmOpen(false);
						setDeleteError(null);
					}}
					onConfirm={async () => {
						try {
							await deleteMutation.mutateAsync();
						} catch {
							// handled by mutation state
						}
					}}
					title={t("detail.form.confirmDelete.title", { defaultValue: "Delete Client Record" })}
					description={t("detail.form.confirmDelete.description", { defaultValue: "Are you sure you want to delete this client record? This action cannot be undone." })}
					confirmLabel={t("detail.form.confirmDelete.confirm", { defaultValue: "Delete" })}
					cancelLabel={t("detail.form.confirmDelete.cancel", { defaultValue: "Cancel" })}
					variant="destructive"
					isLoading={deleteMutation.isPending}
					error={deleteError}
				/>
			</DrawerContent>
		</Drawer>
	);
}
