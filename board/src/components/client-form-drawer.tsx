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
	ImageIcon,
	Loader2,
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
import { notifyError, notifyInfo, notifySuccess } from "../lib/notify";
import { pickClientConfigFilePath, readAbsolutePathFromFile } from "../lib/pick-client-config-file";
import { isTauriEnvironmentSync } from "../lib/platform";
import { useAppStore } from "../lib/store";
import type {
	ClientConfigFileParse,
	ClientConfigFileParseInspectResp,
	ClientConfigFileParseInspectReq,
	ClientConnectionMode,
	ClientFormatRuleData,
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

type ClientFormMode = "create" | "edit";
type ClientConnectionShape = "local_with_config" | "local_without_config" | "remote_http";
const SUPPORTED_TRANSPORT_VALUES = ["streamable_http", "sse", "stdio"] as const;
type SupportedTransportValue = (typeof SUPPORTED_TRANSPORT_VALUES)[number];
const CONFIG_PARSE_FORMAT_VALUES = ["json", "json5", "toml", "yaml"] as const;
type ConfigParseFormatValue = (typeof CONFIG_PARSE_FORMAT_VALUES)[number];
const CONFIG_PARSE_CONTAINER_TYPE_VALUES = ["standard", "array"] as const;

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

type ParseInspectionView = ClientConfigFileParseInspectResp & {
	preview_text?: string;
};

const formSchema = z.object({
	identifier: z.string().min(1),
	displayName: z.string().min(1),
	connectionShape: z.enum(["local_with_config", "local_without_config", "remote_http"]),
	supportedTransports: z.array(z.enum(SUPPORTED_TRANSPORT_VALUES)),
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
	formatRulesJsonText: z.string().optional(),
});

type ClientRecordFormValues = z.infer<typeof formSchema>;

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

function parseFormatRulesFromJsonText(
	jsonText: string | undefined,
	t: TFunction,
): Record<string, ClientFormatRuleData> | undefined {
	const trimmed = jsonText?.trim();
	if (!trimmed) return undefined;
	try {
		const parsed: unknown = JSON.parse(trimmed);
		if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
			throw new Error("Expected a JSON object, not an array or primitive");
		}
		return parsed as Record<string, ClientFormatRuleData>;
	} catch (e) {
		throw new Error(
			t("detail.form.notifications.formatRulesJsonParseError", {
				defaultValue: "Invalid JSON in format rules: " + extractErrorMessage(e),
			}),
		);
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
	if (candidate === "streamable_http" || candidate === "sse" || candidate === "stdio") {
		return candidate;
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
	let supportedTransports = normalizeSupportedTransports(client?.supported_transports);
	if (supportedTransports.length === 0) {
		supportedTransports = ["streamable_http", "stdio"];
	}
	const effectiveParse = resolveEffectiveClientParse(client);

	return {
		identifier,
		displayName: client?.display_name ?? "",
		connectionShape,
		supportedTransports,
		configPath: connectionShape === "local_with_config" ? client?.config_path || "" : "",
		configFileParseFormat: (effectiveParse?.format as ConfigParseFormatValue | undefined) ?? "json",
		configFileParseContainerType: effectiveParse?.container_type === "array" ? "array" : "standard",
		configFileParseContainerKeysText: effectiveParse?.container_keys?.join(", ") ?? "mcpServers",
		clientVersion: client?.client_version ?? "",
		description: client?.description ?? "",
		homepageUrl: client?.homepage_url ?? "",
		docsUrl: client?.docs_url ?? "",
		supportUrl: client?.support_url ?? "",
		logoUrl: client?.logo_url ?? "",
		formatRulesJsonText: client?.format_rules ? JSON.stringify(client.format_rules, null, 2) : "",
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
	const [isHydrating, setIsHydrating] = useState(false);
	const [formError, setFormError] = useState<string | null>(null);
	const [deleteError, setDeleteError] = useState<string | null>(null);
	const [parseInspection, setParseInspection] = useState<ParseInspectionView | null>(null);
	const [parseInspectionError, setParseInspectionError] = useState<string | null>(null);
	const [isParseAdvancedOpen, setIsParseAdvancedOpen] = useState(false);
	const [showParseCodePreview, setShowParseCodePreview] = useState(false);
	const [isDeleteConfirmOpen, setIsDeleteConfirmOpen] = useState(false);
	const [configPathPickBusy, setConfigPathPickBusy] = useState(false);
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
		() => parseInspection?.preview_text?.trim() || inspectionPreviewText(parseInspection?.preview),
		[parseInspection?.preview, parseInspection?.preview_text],
	);

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
		setIsHydrating(true);
		form.reset(defaultValues(client));
		setIsHydrating(false);
	}, [open, client, mode, form]);

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
		[t, i18n.language],
	);
	const supportedTransportOptions = useMemo(
		() => SUPPORTED_TRANSPORT_VALUES.map((transport) => ({ value: transport, label: getTransportSupportLabel(transport, t) })),
		[t, i18n.language],
	);
	const configParseFormatOptions: SegmentOption[] = useMemo(
		() => CONFIG_PARSE_FORMAT_VALUES.map((value) => ({ value, label: value.toUpperCase() })),
		[],
	);
	const configParseContainerTypeOptions: SegmentOption[] = useMemo(
		() => [
			{ value: "standard", label: t("detail.form.configFileParse.containerTypeOptions.standard", { defaultValue: "Object Map" }) },
			{ value: "array", label: t("detail.form.configFileParse.containerTypeOptions.array", { defaultValue: "Array" }) },
		],
		[t, i18n.language],
	);

	const inspectMutation = useMutation({
		mutationFn: async (payload: ClientConfigFileParseInspectReq) => clientsApi.inspectConfigFileParse(payload),
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
			form.setValue("configFileParseFormat", inferred.format as ConfigParseFormatValue, { shouldDirty: true });
			form.setValue("configFileParseContainerType", inferred.container_type === "array" ? "array" : "standard", {
				shouldDirty: true,
			});
			form.setValue("configFileParseContainerKeysText", inferred.container_keys?.join(", ") ?? "", {
				shouldDirty: true,
			});
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

		const timer = window.setTimeout(() => {
			void inspectMutation.mutateAsync({
				config_path: trimmedPath,
				config_file_parse:
					(parseDraft.container_keys?.length ?? 0) > 0 ? parseDraft : undefined,
			});
		}, 350);

		return () => window.clearTimeout(timer);
	}, [open, connectionShape, configPath, parseDraft, inspectMutation]);

	const handleApplyDetectedRules = useCallback(() => {
		const inferred = parseInspection?.inferred_parse;
		if (!inferred) return;
		form.setValue("configFileParseFormat", inferred.format as ConfigParseFormatValue, { shouldDirty: true });
		form.setValue("configFileParseContainerType", inferred.container_type === "array" ? "array" : "standard", { shouldDirty: true });
		form.setValue("configFileParseContainerKeysText", inferred.container_keys?.join(", ") ?? "", { shouldDirty: true });
	}, [form, parseInspection?.inferred_parse]);

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
			const dirtyFields = form.formState.dirtyFields;
			const normalizedIdentifier = normalizeIdentifier(values.identifier);
			const parseForSave = parseDraftFromValues(values);
			const formatRulesTextTrimmed = values.formatRulesJsonText?.trim() ?? "";

			if (hasWritableConfig(values) && (parseForSave.container_keys?.length ?? 0) === 0) {
				throw new Error(
					t("detail.form.configFileParse.keysRequired", {
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
				supported_transports: values.supportedTransports,
				description: values.description || undefined,
				homepage_url: values.homepageUrl || undefined,
				docs_url: values.docsUrl || undefined,
				support_url: values.supportUrl || undefined,
				logo_url: values.logoUrl || undefined,
				config_file_parse: hasWritableConfig(values) ? parseForSave : undefined,
				format_rules: parseFormatRulesFromJsonText(values.formatRulesJsonText, t),
				clear_format_rules: formatRulesTextTrimmed === "" && Boolean(dirtyFields.formatRulesJsonText),
			});

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
											<div className="space-y-1">
												<p className="font-medium">{t("detail.form.configFileParse.label", { defaultValue: "Parse Rules" })}</p>
												<p className="text-sm text-muted-foreground">{t("detail.form.configFileParse.description", { defaultValue: "Edit the file format, container type, and config nodes used to locate MCP server entries." })}</p>
											</div>

											<div className="flex flex-wrap items-center gap-2 pt-1">
												{parseInspection?.inferred_parse ? (
													<Button type="button" variant="outline" size="sm" className="gap-2" onClick={handleApplyDetectedRules}>
														<Sparkles className="h-3 w-3" />
														{t("detail.form.configFileParse.applyDetected", { defaultValue: "Use detected rules" })}
													</Button>
												) : null}
												<Button
													type="button"
													variant="outline"
													size="sm"
													onClick={() => setIsParseAdvancedOpen((value) => !value)}
												>
													{isParseAdvancedOpen
														? t("detail.form.configFileParse.hideAdvanced", { defaultValue: "Hide details" })
														: t("detail.form.configFileParse.showAdvanced", { defaultValue: "Show details" })}
													<ChevronDown className={`ml-2 h-3 w-3 transition-transform ${isParseAdvancedOpen ? "rotate-180" : "rotate-0"}`} />
												</Button>
											</div>

											{isParseAdvancedOpen ? (
												<div className="rounded-md border bg-white/80 dark:bg-slate-950/10">
													<div className="grid gap-3 px-3 py-3 md:grid-cols-2">
														<FormField control={form.control} name="configFileParseFormat" render={({ field }) => (
															<FormItem className="space-y-1.5"><FormLabel className="text-xs font-medium">{t("detail.form.configFileParse.formatLabel", { defaultValue: "Config Format" })}</FormLabel><FormControl><Segment value={field.value} onValueChange={field.onChange} options={configParseFormatOptions} showDots={false} /></FormControl><FormMessage /></FormItem>
														)} />
														<FormField control={form.control} name="configFileParseContainerType" render={({ field }) => (
															<FormItem className="space-y-1.5"><FormLabel className="text-xs font-medium">{t("detail.form.configFileParse.containerTypeLabel", { defaultValue: "Container Type" })}</FormLabel><FormControl><Segment value={field.value} onValueChange={field.onChange} options={configParseContainerTypeOptions} showDots={false} /></FormControl><FormMessage /></FormItem>
														)} />
														<FormField control={form.control} name="configFileParseContainerKeysText" render={({ field }) => (
															<FormItem className="space-y-1.5 md:col-span-2"><FormLabel className="text-xs font-medium">{t("detail.form.configFileParse.containerKeysLabel", { defaultValue: "Config Nodes" })}</FormLabel><FormControl><Input {...field} className="h-8 text-sm" placeholder={t("detail.form.configFileParse.containerKeysPlaceholder", { defaultValue: "mcpServers, context_servers" })} /></FormControl><FormDescription>{t("detail.form.configFileParse.containerKeysDescription", { defaultValue: "Enter config node paths separated by commas. The first path is used as the primary write target." })}</FormDescription><FormMessage /></FormItem>
														)} />
													</div>
												</div>
											) : null}

											<div className="flex items-start justify-between gap-3 border-t pt-2 text-xs text-muted-foreground">
												<div className="min-w-0 flex-1">
													{showParseCodePreview ? (
														<div className="space-y-2">
															<div className="flex items-center gap-2">
																<span>{t("detail.form.configFileParse.previewTitle", { defaultValue: "Detected config snippet" })}</span>
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
																<span>{t("detail.form.configFileParse.validationTitle", { defaultValue: "File association check" })}</span>
															</div>
															<p className="truncate">
																{parseInspectionError
																	? parseInspectionError
																	: parseInspection?.validation?.matches
																		? t("detail.form.configFileParse.validationSuccess", { defaultValue: "The selected file matches the current parse rules." })
																		: t("detail.form.configFileParse.validationHint", { defaultValue: "Pick a config file and MCPMate will validate whether these rules can find MCP server entries." })}
															</p>
															{parseInspection?.validation ? (
																<p>
																	{t("detail.form.configFileParse.detectedFormat", { defaultValue: "Detected format" })}: {parseInspection.detected_format ?? "-"} · {t("detail.form.configFileParse.containerMatch", { defaultValue: "Container" })}: {parseInspection.validation.container_found ? t("detail.form.configFileParse.matchYes", { defaultValue: "Found" }) : t("detail.form.configFileParse.matchNo", { defaultValue: "Missing" })} · {t("detail.form.configFileParse.serverCount", { defaultValue: "Servers" })}: {parseInspection.validation.server_count}
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
													disabled={!parseInspection}
													onClick={() => setShowParseCodePreview((value) => !value)}
													aria-label={showParseCodePreview
														? t("detail.form.configFileParse.summaryViewButton", { defaultValue: "Summary view" })
														: t("detail.form.configFileParse.codeViewButton", { defaultValue: "Code preview" })}
												>
													<Code2 className="h-4 w-4" />
												</Button>
											</div>
										</div>

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
									</>
								) : (
									<div className="ml-24 rounded-md border border-dashed px-3 py-2 text-sm text-muted-foreground">
										{t("detail.form.fields.configPath.unavailableHint", { defaultValue: "This client does not currently have a writable local config path, so file-based configuration management is unavailable." })}
									</div>
								)}
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
								<FormField control={form.control} name="formatRulesJsonText" render={({ field }) => (
									<FormItem className="space-y-0">
										<div className="flex items-start gap-4">
											<FormLabel className={`${CLIENT_FORM_ROW_LABEL_CLASS} pt-3`}>{t("detail.form.fields.formatRulesJsonText.label", { defaultValue: "Format Rules (JSON)" })}</FormLabel>
											<div className="min-w-0 flex-1">
												<FormControl><Textarea {...field} rows={6} placeholder={t("detail.form.fields.formatRulesJsonText.placeholder", { defaultValue: '{"transport_name": {"type_value": "...", ...}}' })} /></FormControl>
												<FormDescription>{t("detail.form.fields.formatRulesJsonText.description", { defaultValue: "Advanced: Fine-grained transport format rules as JSON. Leave empty to reset to defaults." })}</FormDescription>
												<FormMessage />
											</div>
										</div>
									</FormItem>
								)} />
							</TabsContent>
						</Tabs>

						{formError ? <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">{formError}</div> : null}
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
