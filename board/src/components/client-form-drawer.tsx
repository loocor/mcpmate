import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Check, ChevronsUpDown, FolderOpen, ImageIcon } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { type ControllerRenderProps, useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import * as z from "zod";
import { clientsApi } from "../lib/api";
import { notifyError, notifyInfo, notifySuccess } from "../lib/notify";
import { pickClientConfigFilePath, readAbsolutePathFromFile } from "../lib/pick-client-config-file";
import { isTauriEnvironmentSync } from "../lib/platform";
import type { ClientConnectionMode, ClientInfo } from "../lib/types";
import { cn } from "../lib/utils";
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

interface ClientFormDrawerProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	mode: ClientFormMode;
	client?: ClientInfo | null;
	onSuccess?: (identifier: string) => void;
}

const formSchema = z.object({
	identifier: z.string().min(1),
	displayName: z.string().min(1),
	connectionShape: z.enum(["local_with_config", "local_without_config", "remote_http"]),
	supportedTransports: z.array(z.enum(SUPPORTED_TRANSPORT_VALUES)),
	configPath: z.string().optional(),
	clientVersion: z.string().optional(),
	description: z.string().optional(),
	homepageUrl: z.string().optional(),
	docsUrl: z.string().optional(),
	supportUrl: z.string().optional(),
	logoUrl: z.string().optional(),
});

type ClientRecordFormValues = z.infer<typeof formSchema>;

/** Matches server edit / install manual form: label column + control column */
const CLIENT_FORM_ROW_LABEL_CLASS = "w-20 shrink-0 text-right";

function logoUrlIsPreviewable(value: string): boolean {
	const v = value.trim();
	if (!v) return false;
	if (/^https?:\/\//i.test(v)) return true;
	if (/^data:image\//i.test(v)) return true;
	return false;
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

	const canTryImage = logoUrlIsPreviewable(trimmed);
	const showImg = canTryImage && !broken;

	return (
		<FormItem className="min-w-0 space-y-0">
			<div className="flex items-center gap-4">
				<FormLabel className={CLIENT_FORM_ROW_LABEL_CLASS}>{label}</FormLabel>
				<div className="min-w-0 flex-1">
					<div className="flex items-center gap-2">
						<div
							className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-md border border-input bg-muted"
							aria-hidden
						>
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
	if (connectionMode === "remote_http") {
		return "remote_http";
	}

	if (connectionMode === "local_config_detected" && configPath?.trim()) {
		return "local_with_config";
	}

	return "local_without_config";
}

function hasWritableConfig(values: Pick<ClientRecordFormValues, "connectionShape" | "configPath">): boolean {
	return values.connectionShape === "local_with_config" && Boolean(values.configPath?.trim());
}

function normalizeSupportedTransports(value: unknown): SupportedTransportValue[] {
	const seen = new Set<SupportedTransportValue>();
	const normalized: SupportedTransportValue[] = [];

	for (const item of Array.isArray(value) ? value : []) {
		const candidate = String(item).trim().toLowerCase();
		const transport =
			candidate === "streamable_http"
				? "streamable_http"
				: candidate === "sse"
					? "sse"
					: candidate === "stdio"
						? "stdio"
						: null;
		if (transport && !seen.has(transport)) {
			seen.add(transport);
			normalized.push(transport);
		}
	}

	return normalized.sort(
		(left, right) =>
			SUPPORTED_TRANSPORT_VALUES.indexOf(left) - SUPPORTED_TRANSPORT_VALUES.indexOf(right),
	);
}

function getTransportSupportLabel(
	transport: SupportedTransportValue,
	t: ReturnType<typeof useTranslation>["t"],
): string {
	return transport === "streamable_http"
		? t("detail.form.transportSupport.options.streamableHttpLegacy", {
				defaultValue: "Streamable HTTP",
			})
		: transport === "sse"
			? t("detail.form.transportSupport.options.sseLegacy", {
					defaultValue: "SSE (Legacy)",
				})
			: t("detail.form.transportSupport.options.stdio", {
					defaultValue: "STDIO",
				});
}

function TransportSupportCombobox({
	value,
	onChange,
	options,
	placeholder,
	emptyText,
}: {
	value: SupportedTransportValue[];
	onChange: (next: SupportedTransportValue[]) => void;
	options: Array<{ value: SupportedTransportValue; label: string }>;
	placeholder: string;
	emptyText: string;
}) {
	const [open, setOpen] = useState(false);
	const selectedLabels = options
		.filter((option) => value.includes(option.value))
		.map((option) => option.label);

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<Button
					type="button"
					variant="outline"
					role="combobox"
					aria-expanded={open}
					className="w-full justify-between"
				>
					<span className="truncate text-left font-normal">
						{selectedLabels.length > 0 ? selectedLabels.join(", ") : placeholder}
					</span>
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
											const next = selected
												? value.filter((item) => item !== option.value)
												: [...value, option.value];
											onChange(normalizeSupportedTransports(next));
										}}
									>
										<Check
											className={cn(
												"mr-2 h-4 w-4",
												selected ? "opacity-100" : "opacity-0",
											)}
										/>
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
}

function defaultValues(client?: ClientInfo | null): ClientRecordFormValues {
	const identifier = client?.identifier ?? "";
	const connectionShape = connectionModeToShape(client?.connection_mode, client?.config_path);
	const supportedTransports = normalizeSupportedTransports(client?.supported_transports);
	return {
		identifier,
		displayName: client?.display_name ?? "",
		connectionShape,
		supportedTransports:
			supportedTransports.length > 0 ? supportedTransports : ["streamable_http", "stdio"],
		configPath: connectionShape === "local_with_config" ? client?.config_path || "" : "",
		clientVersion: client?.client_version ?? "",
		description: client?.description ?? "",
		homepageUrl: client?.homepage_url ?? "",
		docsUrl: client?.docs_url ?? "",
		supportUrl: client?.support_url ?? "",
		logoUrl: client?.logo_url ?? "",
	};
}


export function ClientFormDrawer({
	open,
	onOpenChange,
	mode,
	client,
	onSuccess,
}: ClientFormDrawerProps) {
	const { t } = useTranslation("clients");
	const qc = useQueryClient();
	const [isHydrating, setIsHydrating] = useState(false);
	const [formError, setFormError] = useState<string | null>(null);
	const [configPathPickBusy, setConfigPathPickBusy] = useState(false);
	const isTauriShell = useMemo(() => isTauriEnvironmentSync(), []);
	const configPathFileInputRef = useRef<HTMLInputElement>(null);

	const form = useForm<ClientRecordFormValues>({
		resolver: zodResolver(formSchema),
		defaultValues: defaultValues(client),
	});

	const connectionShape = form.watch("connectionShape");
	const identifier = form.watch("identifier");
	useEffect(() => {
		if (!open) return;
		setFormError(null);
		setIsHydrating(true);

		const baseValues = defaultValues(client);
		form.reset(baseValues);
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
		if (connectionShape !== "local_with_config" && form.getValues("configPath")) {
			form.setValue("configPath", "", { shouldDirty: true });
		}
	}, [connectionShape, form]);

	const connectionOptions: SegmentOption[] = useMemo(
		() => [
			{
				value: "local_with_config",
				label: t("detail.form.connectionShape.options.localWithConfig", { defaultValue: "Local + Config" }),
			},
			{
				value: "local_without_config",
				label: t("detail.form.connectionShape.options.localWithoutConfig", { defaultValue: "Local / Unknown Config" }),
			},
			{
				value: "remote_http",
				label: t("detail.form.connectionShape.options.remoteHttp", { defaultValue: "Remote HTTP" }),
			},
		],
		[t],
	);
	const supportedTransportOptions = useMemo(
		() =>
			SUPPORTED_TRANSPORT_VALUES.map((transport) => ({
				value: transport,
				label: getTransportSupportLabel(transport, t),
			})),
		[t],
	);

	const handleConfigPathBrowse = useCallback(async () => {
		if (!isTauriShell) return;
		setConfigPathPickBusy(true);
		try {
			const path = await pickClientConfigFilePath(
				t("detail.form.fields.configPath.dialogTitle", {
					defaultValue: "Select MCP configuration file",
				}),
			);
			if (path) {
				form.setValue("configPath", path, {
					shouldDirty: true,
					shouldTouch: true,
					shouldValidate: true,
				});
			}
		} catch (error) {
			notifyError(
				t("detail.form.fields.configPath.pickFailedTitle", {
					defaultValue: "Could not open file dialog",
				}),
				String(error),
			);
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
					form.setValue("configPath", path, {
						shouldDirty: true,
						shouldTouch: true,
						shouldValidate: true,
					});
					return;
				}
				notifyInfo(
					t("detail.form.fields.configPath.webPickInfoTitle", {
						defaultValue: "Could not read file path",
					}),
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
			await clientsApi.update({
				identifier: normalizedIdentifier,
				display_name: values.displayName || undefined,
				connection_mode: connectionShapeToMode(values.connectionShape),
				config_path: hasWritableConfig(values) ? (values.configPath?.trim() || undefined) : undefined,
				client_version: values.clientVersion?.trim() || undefined,
				supported_transports: values.supportedTransports,
				description: values.description || undefined,
				homepage_url: values.homepageUrl || undefined,
				docs_url: values.docsUrl || undefined,
				support_url: values.supportUrl || undefined,
				logo_url: values.logoUrl || undefined,
			});
			return normalizedIdentifier;
		},
		onSuccess: async (savedIdentifier) => {
			setFormError(null);
			await qc.invalidateQueries({ queryKey: ["clients"] });
			await qc.invalidateQueries({ queryKey: ["client-config", savedIdentifier] });
			await qc.invalidateQueries({ queryKey: ["client-capability-config", savedIdentifier] });
			notifySuccess(
				mode === "create"
					? t("detail.form.notifications.createSuccess.title", { defaultValue: "Client record created" }) : t("detail.form.notifications.editSuccess.title", { defaultValue: "Client record updated" }),
				mode === "create"
					? t("detail.form.notifications.createSuccess.message", { defaultValue: "The client record has been created." }) : t("detail.form.notifications.editSuccess.message", { defaultValue: "The client record has been updated." }),
			);
			onOpenChange(false);
			onSuccess?.(savedIdentifier);
		},
		onError: (error) => {
			const message = String(error);
			setFormError(message);
			notifyError(
				t("detail.form.notifications.saveFailed.title", { defaultValue: "Unable to save client record" }),
				message,
			);
		},
	});

	return (
		<Drawer open={open} onOpenChange={onOpenChange}>
			<DrawerContent>
				<DrawerHeader>
					<DrawerTitle>
						{mode === "create"
							? t("detail.form.titleCreate", { defaultValue: "Add Client Record" }) : t("detail.form.titleEdit", { defaultValue: "Edit Client Record" })}
					</DrawerTitle>
					<DrawerDescription>
						{mode === "create"
							? t("detail.form.descriptionCreate", { defaultValue: "Create a client record with its management shape and metadata." }) : t("detail.form.descriptionEdit", { defaultValue: "Update this client record and its management settings." })}
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
									<FormField
										control={form.control}
										name="displayName"
										render={({ field }) => (
											<FormItem className="space-y-0">
												<div className="flex items-center gap-4">
													<FormLabel className={CLIENT_FORM_ROW_LABEL_CLASS}>
														{t("detail.form.fields.displayName.label", { defaultValue: "Client Name" })}
													</FormLabel>
													<div className="min-w-0 flex-1">
														<FormControl>
															<Input
																{...field}
																placeholder={t("detail.form.fields.displayName.placeholder", {
																	defaultValue: "Cursor Desktop",
																})}
															/>
														</FormControl>
														<FormMessage />
													</div>
												</div>
											</FormItem>
										)}
									/>
									<FormField
										control={form.control}
										name="identifier"
										render={({ field }) => (
											<FormItem className="space-y-0">
												<div className="flex items-center gap-4">
													<FormLabel className={CLIENT_FORM_ROW_LABEL_CLASS}>
														{t("detail.form.fields.identifier.label", { defaultValue: "Client ID" })}
													</FormLabel>
													<div className="min-w-0 flex-1">
														<FormControl>
															<Input
																{...field}
																disabled={mode !== "create"}
																placeholder={t("detail.form.fields.identifier.placeholder", {
																	defaultValue: "cursor-desktop",
																})}
															/>
														</FormControl>
														<FormMessage />
													</div>
												</div>
											</FormItem>
										)}
									/>
									<FormField
										control={form.control}
										name="clientVersion"
										render={({ field }) => (
											<FormItem className="space-y-0">
												<div className="flex items-center gap-4">
													<FormLabel className={CLIENT_FORM_ROW_LABEL_CLASS}>
														{t("detail.form.fields.clientVersion.label", { defaultValue: "Client Version" })}
													</FormLabel>
													<div className="min-w-0 flex-1">
														<FormControl>
															<Input
																{...field}
																placeholder={t("detail.form.fields.clientVersion.placeholder", {
																	defaultValue: "optional",
																})}
															/>
														</FormControl>
														<FormMessage />
													</div>
												</div>
											</FormItem>
										)}
									/>
								</div>
								{mode === "create" ? (
									<p className="pl-24 text-sm text-muted-foreground">
										{t("detail.form.fields.identifier.description", {
											defaultValue:
												"Spaces and casing are normalized automatically when creating a new record.",
										})}
									</p>
								) : null}

								<FormField
									control={form.control}
									name="connectionShape"
									render={({ field }) => (
										<FormItem className="space-y-0">
											<div className="flex items-start gap-4">
												<FormLabel className={`${CLIENT_FORM_ROW_LABEL_CLASS} pt-2`}>
													{t("detail.form.connectionShape.label", { defaultValue: "Client Shape" })}
												</FormLabel>
												<div className="min-w-0 flex-1">
													<FormControl>
														<Segment
															value={field.value}
															onValueChange={field.onChange}
															options={connectionOptions}
															showDots={false}
														/>
													</FormControl>
													<FormDescription>
														{t("detail.form.connectionShape.description", {
															defaultValue:
																"Choose whether this client has a writable local config file or is a non-writable remote/unknown client.",
														})}
													</FormDescription>
													<FormMessage />
												</div>
											</div>
										</FormItem>
									)}
								/>

								{connectionShape === "local_with_config" ? (
									<FormField
										control={form.control}
										name="configPath"
										render={({ field }) => (
											<FormItem className="space-y-0">
												<div className="flex items-start gap-4">
													<FormLabel className={`${CLIENT_FORM_ROW_LABEL_CLASS} pt-2`}>
														{t("detail.form.fields.configPath.label", { defaultValue: "Config File Path" })}
													</FormLabel>
													<div className="min-w-0 flex-1">
														<input
															ref={configPathFileInputRef}
															type="file"
															className="hidden"
															accept=".json,.yaml,.yml,.toml,application/json,text/yaml"
															aria-hidden
															tabIndex={-1}
															onChange={handleConfigPathWebFileChange}
														/>
														<div className="flex w-full gap-2">
															<FormControl>
											<Input
												{...field}
												autoComplete="off"
												spellCheck={false}
												placeholder={t("detail.form.fields.configPath.placeholder", {
													defaultValue: "~/.cursor/mcp.json",
												})}
												className="min-w-0 flex-1 font-mono text-sm"
											/>
															</FormControl>
															<Button
																type="button"
																variant="outline"
																className="shrink-0 gap-2"
																disabled={
																	configPathPickBusy || saveMutation.isPending
																}
																onClick={() => handleConfigPathBrowseButton()}
																aria-label={t("detail.form.fields.configPath.browseAria", {
																	defaultValue: "Browse for configuration file on disk",
																})}
															>
																<FolderOpen className="h-4 w-4 shrink-0" aria-hidden />
																<span>
																	{t("detail.form.fields.configPath.browse", { defaultValue: "Choose…" })}
																</span>
															</Button>
														</div>
														<FormDescription>
															{t("detail.form.fields.configPath.description", {
																defaultValue:
																	"A writable local config path enables MCPMate to manage this client through file-based configuration operations.",
															})}
														</FormDescription>
														<FormMessage />
													</div>
												</div>
											</FormItem>
										)}
									/>
								) : (
									<div className="ml-24 rounded-md border border-dashed px-3 py-2 text-sm text-muted-foreground">
										{t("detail.form.fields.configPath.unavailableHint", {
											defaultValue:
												"This client does not currently have a writable local config path, so file-based configuration management is unavailable.",
										})}
									</div>
								)}

								<FormField
									control={form.control}
									name="supportedTransports"
									render={({ field }) => (
										<FormItem className="space-y-0">
											<div className="flex items-start gap-4">
												<FormLabel className={`${CLIENT_FORM_ROW_LABEL_CLASS} pt-2`}>
													{t("detail.form.transportSupport.label", { defaultValue: "Transport Support" })}
												</FormLabel>
												<div className="min-w-0 flex-1 space-y-2">
													<FormControl>
														<TransportSupportCombobox
															value={field.value}
															onChange={field.onChange}
															options={supportedTransportOptions}
															placeholder={t("detail.form.transportSupport.placeholder", {
																defaultValue: "Select supported transports",
															})}
															emptyText={t("detail.form.transportSupport.empty", {
																defaultValue: "No transports found.",
															})}
														/>
													</FormControl>
													{field.value.length > 0 ? (
														<div className="flex flex-wrap gap-2">
															{field.value.map((transport) => (
																<span
																	key={transport}
																	className="inline-flex items-center rounded-md border bg-muted px-2 py-1 text-xs text-muted-foreground"
																>
																	{getTransportSupportLabel(transport, t)}
																</span>
															))}
														</div>
													) : null}
													<FormDescription>
														{t("detail.form.transportSupport.description", {
															defaultValue:
																"Choose which runtime transports this client supports. This array is the only source used to constrain hosted/unify transport selection.",
														})}
													</FormDescription>
													<FormMessage />
												</div>
											</div>
										</FormItem>
									)}
								/>

							</TabsContent>

							<TabsContent value="meta" className="space-y-4 pt-4">
								<div className="space-y-4">
									<FormField
										control={form.control}
										name="logoUrl"
										render={({ field }) => (
											<LogoUrlFieldWithPreview
												field={field}
												label={t("detail.form.fields.logoUrl.label", { defaultValue: "Logo URL" })}
												placeholder={t("detail.form.fields.logoUrl.placeholder", {
													defaultValue: "https://example.com/logo.png",
												})}
											/>
										)}
									/>
									<FormField
										control={form.control}
										name="homepageUrl"
										render={({ field }) => (
											<FormItem className="space-y-0">
												<div className="flex items-center gap-4">
													<FormLabel className={CLIENT_FORM_ROW_LABEL_CLASS}>
														{t("detail.form.fields.homepageUrl.label", { defaultValue: "Homepage URL" })}
													</FormLabel>
													<div className="min-w-0 flex-1">
														<FormControl>
													<Input
														{...field}
														placeholder={t("detail.form.fields.homepageUrl.placeholder", {
															defaultValue: "https://example.com",
														})}
													/>
														</FormControl>
														<FormMessage />
													</div>
												</div>
											</FormItem>
										)}
									/>
									<FormField
										control={form.control}
										name="docsUrl"
										render={({ field }) => (
											<FormItem className="space-y-0">
												<div className="flex items-center gap-4">
													<FormLabel className={CLIENT_FORM_ROW_LABEL_CLASS}>
														{t("detail.form.fields.docsUrl.label", { defaultValue: "Docs URL" })}
													</FormLabel>
													<div className="min-w-0 flex-1">
														<FormControl>
													<Input
														{...field}
														placeholder={t("detail.form.fields.docsUrl.placeholder", {
															defaultValue: "https://docs.example.com",
														})}
													/>
														</FormControl>
														<FormMessage />
													</div>
												</div>
											</FormItem>
										)}
									/>
									<FormField
										control={form.control}
										name="supportUrl"
										render={({ field }) => (
											<FormItem className="space-y-0">
												<div className="flex items-center gap-4">
													<FormLabel className={CLIENT_FORM_ROW_LABEL_CLASS}>
														{t("detail.form.fields.supportUrl.label", { defaultValue: "Support URL" })}
													</FormLabel>
													<div className="min-w-0 flex-1">
														<FormControl>
													<Input
														{...field}
														placeholder={t("detail.form.fields.supportUrl.placeholder", {
															defaultValue: "https://support.example.com",
														})}
													/>
														</FormControl>
														<FormMessage />
													</div>
												</div>
											</FormItem>
										)}
									/>
								</div>
								<FormField
									control={form.control}
									name="description"
									render={({ field }) => (
										<FormItem className="space-y-0">
											<div className="flex items-start gap-4">
												<FormLabel className={`${CLIENT_FORM_ROW_LABEL_CLASS} pt-3`}>
													{t("detail.form.fields.description.label", { defaultValue: "Description" })}
												</FormLabel>
												<div className="min-w-0 flex-1">
													<FormControl>
														<Textarea
															{...field}
															rows={4}
															placeholder={t("detail.form.fields.description.placeholder", {
																defaultValue: "A short summary of this client.",
															})}
														/>
													</FormControl>
													<FormDescription>
														{t("detail.form.fields.description.description", {
															defaultValue:
																"These meta fields are stored for display and guidance; the old template files remain only as compatibility seeds.",
														})}
													</FormDescription>
													<FormMessage />
												</div>
											</div>
										</FormItem>
									)}
								/>
							</TabsContent>
						</Tabs>

						{formError ? <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">{formError}</div> : null}
					</form>
				</Form>

				<DrawerFooter className="mt-auto border-t px-6 py-4">
					<div className="flex w-full items-center justify-between gap-3">
						<Button
							type="button"
							variant="outline"
							onClick={() => onOpenChange(false)}
							disabled={saveMutation.isPending}
						>
							{t("detail.form.buttons.cancel", { defaultValue: "Cancel" })}
						</Button>
						<div className="flex items-center gap-3">
							<Button
								type="button"
								onClick={form.handleSubmit(() => saveMutation.mutate())}
								disabled={saveMutation.isPending}
							>
								{mode === "create"
									? t("detail.form.buttons.create", { defaultValue: "Create Record" }) : t("detail.form.buttons.save", { defaultValue: "Save Changes" })}
							</Button>
						</div>
					</div>
				</DrawerFooter>
			</DrawerContent>
		</Drawer>
	);
}
