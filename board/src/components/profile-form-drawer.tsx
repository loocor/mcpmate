import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Loader2 } from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { usePageTranslations } from "../lib/i18n/usePageTranslations";
import { useNavigate } from "react-router-dom";
import { configSuitsApi, serversApi } from "../lib/api";
import { notifyError, notifySuccess } from "../lib/notify";
import type {
	ConfigSuit,
	ConfigSuitPrompt,
	ConfigSuitResource,
	ConfigSuitServer,
	ConfigSuitTool,
	CreateConfigSuitRequest,
	UpdateConfigSuitRequest,
} from "../lib/types";
import { Button } from "./ui/button";
import {
	Drawer,
	DrawerContent,
	DrawerDescription,
	DrawerFooter,
	DrawerHeader,
	DrawerTitle,
} from "./ui/drawer";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./ui/select";
import { Switch } from "./ui/switch";
import { Textarea } from "./ui/textarea";
import { Transfer, type TransferItem } from "./ui/transfer";

type DrawerStep = "details" | "servers";

const arraysEqual = (a: string[], b: string[]) => {
	if (a.length !== b.length) {
		return false;
	}
	const setB = new Set(b);
	return a.every((id) => setB.has(id));
};

const syncSuitServers = async (
	suitId: string,
	previous: string[],
	next: string[],
) => {
	const previousSet = new Set(previous);
	const nextSet = new Set(next);
	const toEnable = next.filter((id) => !previousSet.has(id));
	const toRemove = previous.filter((id) => !nextSet.has(id));

	if (toEnable.length > 0) {
		await Promise.all(
			toEnable.map((id) => configSuitsApi.enableServer(suitId, id)),
		);
	}

	if (toRemove.length > 0) {
		await Promise.all(
			toRemove.map((id) => configSuitsApi.removeServer(suitId, id)),
		);
	}
};

const formatProfileTypeLabel = (value: string) =>
	value
		.split(/[\s_]+/)
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1))
		.join(" ");

interface ProfileFormDrawerProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	mode: "create" | "edit";
	suit?: ConfigSuit;
	onSuccess?: () => void;
	restrictProfileType?: string; // Restrict to specific profile type
}

export function ProfileFormDrawer({
	open,
	onOpenChange,
	mode,
	suit,
	onSuccess,
	restrictProfileType,
}: ProfileFormDrawerProps) {
	const { t, i18n } = useTranslation();
	usePageTranslations("profiles");
	const queryClient = useQueryClient();
	const navigate = useNavigate();

	// Form state
	const [formData, setFormData] = useState<{
		name: string;
		description: string;
		suit_type: string;
		multi_select: boolean;
		priority: number;
		is_active: boolean;
		is_default: boolean;
		clone_from_id: string;
	}>({
		name: "",
		description: "",
		suit_type: restrictProfileType || "shared",
		multi_select: true,
		priority: 50,
		is_active: false,
		is_default: false,
		clone_from_id: "none",
	});

	// Generate unique IDs for form elements
	const nameId = useId();
	const descriptionId = useId();
	const multiSelectId = useId();
	const isActiveId = useId();
	const isDefaultId = useId();
	const cloneFromId = useId();

	const [step, setStep] = useState<DrawerStep>("details");
	const [selectedServerIds, setSelectedServerIds] = useState<string[]>([]);
	const [selectionInitialized, setSelectionInitialized] = useState(false);
	const [serverSelectionTouched, setServerSelectionTouched] = useState(false);
	const [cloneSelectionApplied, setCloneSelectionApplied] = useState(false);
	const [isClosing, setIsClosing] = useState(false);
	const steps: Array<{ id: DrawerStep; label: string; hint: string }> = [
		{
			id: "details",
			label: t("profiles:form.steps.profile", { defaultValue: "Profile" }),
			hint: t("profiles:form.steps.hints.basics", { defaultValue: "Basics" }),
		},
		{
			id: "servers",
			label: t("profiles:form.steps.servers", { defaultValue: "Servers" }),
			hint: t("profiles:form.steps.hints.assign", { defaultValue: "Assign" }),
		},
	];

	// 完全重置所有状态的函数
	const resetAllStates = useCallback(() => {
		setStep("details");
		setSelectionInitialized(false);
		setServerSelectionTouched(false);
		setCloneSelectionApplied(false);
		setSelectedServerIds([]);

		if (mode === "edit" && suit) {
			setFormData({
				name: suit.name,
				description: suit.description || "",
				suit_type: suit.suit_type,
				multi_select: suit.multi_select,
				priority: suit.priority,
				is_active: suit.is_active,
				is_default: suit.is_default,
				clone_from_id: "none", // Not applicable in edit mode
			});
		} else {
			// Create mode - reset to empty form
			setFormData({
				name: "",
				description: "",
				suit_type: restrictProfileType || "shared",
				multi_select: true,
				priority: 50,
				is_active: false,
				is_default: false,
				clone_from_id: "none",
			});
		}
	}, [mode, suit, restrictProfileType]);

	// Overlay close handler (immediate, no delay)
	const handleOverlayClose = useCallback(() => {
		if (!isClosing) {
			setIsClosing(true);
			resetAllStates();
			onOpenChange(false);
			setIsClosing(false);
		}
	}, [onOpenChange, resetAllStates, isClosing]);

	// Cancel close handler (with delay for complete reset)
	const handleCancelClose = useCallback(() => {
		if (!isClosing) {
			setIsClosing(true);
			setTimeout(() => {
				resetAllStates();
				onOpenChange(false);
				setIsClosing(false);
			}, 150); // Small delay to allow animation
		}
	}, [onOpenChange, resetAllStates, isClosing]);

	const closeDrawer = useCallback(
		() => handleCancelClose(),
		[handleCancelClose],
	);

	// Reset form data when dialog opens or mode/suit changes
	useEffect(() => {
		if (open) {
			resetAllStates();
		} else {
			// 关闭时清理查询缓存，防止状态残留
			setTimeout(() => {
				queryClient.removeQueries({
					queryKey: ["configSuitDrawerServers"],
					exact: true,
				});
				if (mode === "edit" && suit?.id) {
					queryClient.removeQueries({
						queryKey: ["configSuitDrawerProfileServers", suit.id],
						exact: true,
					});
				}
				queryClient.removeQueries({
					queryKey: ["configSuitClonePreview"],
					exact: false,
				});
			}, 200);
		}
	}, [open, mode, suit, resetAllStates, queryClient]);

	// Fetch all suits for cloning option
	const { data: suitsResponse } = useQuery({
		queryKey: ["configSuits"],
		queryFn: configSuitsApi.getAll,
		enabled: open,
	});

	const { data: allServersResponse, isLoading: isLoadingAllServers } = useQuery(
		{
			queryKey: ["configSuitDrawerServers"],
			queryFn: serversApi.getAll,
			enabled: open,
			staleTime: 30_000,
		},
	);

	const { data: suitServersResponse, isLoading: isLoadingSuitServers } =
		useQuery({
			queryKey: ["configSuitDrawerProfileServers", suit?.id],
			queryFn: () =>
				suit?.id
					? configSuitsApi.getServers(suit.id)
					: Promise.resolve(undefined),
			enabled: open && mode === "edit" && !!suit?.id,
			staleTime: 15_000,
		});

	const availableSuits = useMemo(
		() => suitsResponse?.suits ?? [],
		[suitsResponse],
	);
	const defaultSuitId = useMemo(
		() => availableSuits.find((profile) => profile.is_default)?.id,
		[availableSuits],
	);

	const cloneableSuits = useMemo(
		() => availableSuits.filter((profile) => profile.suit_type === "shared"),
		[availableSuits],
	);
	const clonePreviewEnabled = formData.clone_from_id !== "none";
	const { data: cloneContent, isFetching: clonePreviewLoading } = useQuery<{
		servers: ConfigSuitServer[];
		tools: ConfigSuitTool[];
		resources: ConfigSuitResource[];
		prompts: ConfigSuitPrompt[];
	}>({
		queryKey: ["configSuitClonePreview", formData.clone_from_id],
		queryFn: async () => {
			const [serversResult, toolsResult, resourcesResult, promptsResult] =
				await Promise.allSettled([
					configSuitsApi.getServers(formData.clone_from_id),
					configSuitsApi.getTools(formData.clone_from_id),
					configSuitsApi.getResources(formData.clone_from_id),
					configSuitsApi.getPrompts(formData.clone_from_id),
				]);
			return {
				servers:
					serversResult.status === "fulfilled"
						? serversResult.value.servers || []
						: [],
				tools:
					toolsResult.status === "fulfilled"
						? toolsResult.value.tools || []
						: [],
				resources:
					resourcesResult.status === "fulfilled"
						? resourcesResult.value.resources || []
						: [],
				prompts:
					promptsResult.status === "fulfilled"
						? promptsResult.value.prompts || []
						: [],
			};
		},
		enabled: clonePreviewEnabled,
		staleTime: 30_000,
	});

	useEffect(() => {
		if (!open) {
			return;
		}
		if (mode === "edit") {
			if (!suitServersResponse?.servers || selectionInitialized) {
				return;
			}
			// 选择所有归属于Profile的服务器（不考虑启用状态）
			// 这里管理的是归属关系，不是启用状态
			const allProfileServers = suitServersResponse.servers.map(
				(server) => server.id,
			);
			setSelectedServerIds(allProfileServers);
			setSelectionInitialized(true);
		} else if (mode === "create" && !selectionInitialized) {
			setSelectionInitialized(true);
		}
	}, [open, mode, selectionInitialized, suitServersResponse]);

	useEffect(() => {
		if (!open || mode !== "create") {
			return;
		}
		if (formData.clone_from_id === "none") {
			setCloneSelectionApplied(false);
			return;
		}
		if (!cloneContent || clonePreviewLoading) {
			return;
		}
		if (serverSelectionTouched && cloneSelectionApplied) {
			return;
		}
		const serverIds = (cloneContent.servers || [])
			.filter((server) => server.enabled)
			.map((server) => server.id);
		setSelectedServerIds(serverIds);
		setCloneSelectionApplied(true);
	}, [
		open,
		mode,
		formData.clone_from_id,
		cloneContent,
		clonePreviewLoading,
		serverSelectionTouched,
		cloneSelectionApplied,
	]);

	// Create mutation
	const createMutation = useMutation({
		mutationFn: async ({
			data,
			selectedServers,
		}: {
			data: CreateConfigSuitRequest;
			selectedServers: string[];
		}) => {
			const result = await configSuitsApi.createSuit(data);
			const createdId = result.data?.id;
			if (createdId) {
				await syncSuitServers(createdId, [], selectedServers);
			}
			return { createdId };
		},
		onSuccess: ({ createdId }) => {
			queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			if (createdId) {
				queryClient.invalidateQueries({ queryKey: ["configSuit", createdId] });
				queryClient.invalidateQueries({
					queryKey: ["configSuitServers", createdId],
				});
			}
			notifySuccess(
				t("profiles:form.messages.created", { defaultValue: "Created" }),
				t("profiles:form.messages.createdDescription", {
					defaultValue: "Profile created successfully",
				}),
			);
			closeDrawer();
			onSuccess?.();
		},
		onError: (error: Error) => {
			notifyError(
				t("profiles:form.messages.createFailed", {
					defaultValue: "Create failed",
				}),
				error.message ||
					t("profiles:form.messages.createFailed", {
						defaultValue: "Create failed",
					}),
			);
		},
	});

	// Update mutation
	const updateMutation = useMutation({
		mutationFn: async ({
			id,
			data,
			previousServers,
			nextServers,
		}: {
			id: string;
			data: UpdateConfigSuitRequest;
			previousServers: string[];
			nextServers: string[];
		}) => {
			const hasFieldUpdates = Object.values(data).some(
				(value) => value !== undefined,
			);
			if (hasFieldUpdates) {
				await configSuitsApi.updateSuit(id, data);
			}
			await syncSuitServers(id, previousServers, nextServers);
			return { id };
		},
		onSuccess: ({ id }) => {
			queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			if (id) {
				queryClient.invalidateQueries({ queryKey: ["configSuit", id] });
				queryClient.invalidateQueries({
					queryKey: ["configSuitServers", id],
				});
			}
			notifySuccess(
				t("profiles:form.messages.updated", { defaultValue: "Updated" }),
				t("profiles:form.messages.updatedDescription", {
					defaultValue: "Profile updated successfully",
				}),
			);
			closeDrawer();
			onSuccess?.();
		},
		onError: (error: Error) => {
			notifyError(
				t("profiles:form.messages.updateFailed", {
					defaultValue: "Update failed",
				}),
				error.message ||
					t("profiles:form.messages.updateFailed", {
						defaultValue: "Update failed",
					}),
			);
		},
	});

	// Handle form submission
	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();

		if (!formData.name.trim()) {
			notifyError(
				t("profiles:form.messages.validationFailed", {
					defaultValue: "Validation failed",
				}),
				t("profiles:form.messages.nameRequired", {
					defaultValue: "Name is required",
				}),
			);
			return;
		}

		if (step === "details") {
			setStep("servers");
			return;
		}

		if (mode === "create") {
			const createData: CreateConfigSuitRequest = {
				name: formData.name,
				description: formData.description || undefined,
				suit_type: formData.suit_type,
				multi_select: formData.multi_select,
				priority: formData.priority,
				is_active: formData.is_active,
				is_default: formData.is_default,
				clone_from_id:
					formData.clone_from_id && formData.clone_from_id !== "none"
						? formData.clone_from_id
						: undefined,
			};
			createMutation.mutate({
				data: createData,
				selectedServers: selectedServerIds,
			});
		} else if (suit) {
			const updateData: UpdateConfigSuitRequest = {
				name: formData.name !== suit.name ? formData.name : undefined,
				description:
					formData.description !== (suit.description || "")
						? formData.description
						: undefined,
				suit_type:
					formData.suit_type !== suit.suit_type
						? formData.suit_type
						: undefined,
				multi_select:
					formData.multi_select !== suit.multi_select
						? formData.multi_select
						: undefined,
				priority:
					formData.priority !== suit.priority ? formData.priority : undefined,
				is_active:
					formData.is_active !== suit.is_active
						? formData.is_active
						: undefined,
				is_default:
					formData.is_default !== suit.is_default
						? formData.is_default
						: undefined,
			};
			const currentTargetKeys = targetServerKeys;
			const selectionChanged = !arraysEqual(
				selectedServerIds,
				currentTargetKeys,
			);
			const hasFieldUpdates = Object.values(updateData).some(
				(value) => value !== undefined,
			);
			if (!selectionChanged && !hasFieldUpdates) {
				closeDrawer();
				return;
			}
			updateMutation.mutate({
				id: suit.id,
				data: updateData,
				previousServers: currentTargetKeys,
				nextServers: selectedServerIds,
			});
		}
	};

	const isMutating = createMutation.isPending || updateMutation.isPending;
	const detailsStepValid = formData.name.trim().length > 0;

	const profileServers = suitServersResponse?.servers ?? [];
	const allServers = allServersResponse?.servers ?? [];

	const transferDataSource = useMemo((): TransferItem[] => {
		const serverMap = new Map<string, TransferItem>();

		// 首先处理所有可用的服务器
		allServers.forEach((server) => {
			const serverType =
				server.server_type || t("status.unknown", { defaultValue: "Unknown" });
			serverMap.set(server.id, {
				id: server.id,
				name: server.name || server.id,
				description: `${serverType} • ${t("status.status", { defaultValue: "Status" })}: ${server.status || t("status.unknown", { defaultValue: "Unknown" })}`,
				type: serverType,
				status: server.status,
			});
		});

		// 然后处理配置文件中的服务器（如果不在全量列表中）
		profileServers.forEach((server) => {
			if (!serverMap.has(server.id)) {
				serverMap.set(server.id, {
					id: server.id,
					name: server.name || server.id,
					description: `${t("status.status", { defaultValue: "Status" })}: ${server.enabled ? t("status.enabled", { defaultValue: "Enabled" }) : t("status.disabled", { defaultValue: "Disabled" })}`,
					status: server.enabled ? "enabled" : "disabled",
				});
			}
		});

		return Array.from(serverMap.values()).sort((a, b) =>
			a.name.localeCompare(b.name),
		);
	}, [allServers, profileServers, i18n.language]);

	// 获取当前已经纳入管理的服务器 ID 列表
	const targetServerKeys = useMemo(() => {
		return profileServers.map((server) => server.id); // 包含所有在Profile中的服务器（无论启用停用）
	}, [profileServers]);

	const selectedServerCount = targetServerKeys.length;
	const totalServerCount = transferDataSource.length;
	const isServersStepLoading =
		step === "servers" &&
		(isLoadingAllServers ||
			(mode === "edit" && !selectionInitialized && isLoadingSuitServers));

	const primaryDisabled =
		isMutating ||
		(step === "details" && !detailsStepValid) ||
		(step === "servers" && isServersStepLoading);
	const primaryLabel = isMutating
		? t("profiles:form.buttons.saving", { defaultValue: "Saving..." })
		: step === "details"
			? t("profiles:form.buttons.next", { defaultValue: "Next" })
			: mode === "create"
				? t("profiles:form.buttons.create", { defaultValue: "Create Profile" })
				: t("profiles:form.buttons.save", { defaultValue: "Save Changes" });

	const showDefaultToggle =
		mode === "edit"
			? !defaultSuitId || defaultSuitId === suit?.id
			: !defaultSuitId;

	useEffect(() => {
		if (!showDefaultToggle) {
			setFormData((prev) =>
				prev.is_default ? { ...prev, is_default: false } : prev,
			);
		}
	}, [showDefaultToggle]);

	const clonePreview = useMemo(() => {
		if (formData.clone_from_id === "none") {
			return null;
		}
		const selected = cloneableSuits.find(
			(profile) => profile.id === formData.clone_from_id,
		);
		if (!selected) {
			return null;
		}

		const serverCount = cloneContent?.servers?.length ?? 0;
		const toolCount = cloneContent?.tools?.length ?? 0;
		const resourceCount = cloneContent?.resources?.length ?? 0;
		const promptCount = cloneContent?.prompts?.length ?? 0;

		const typeValue = selected.suit_type
			? t(`profiles:suitTypes.${selected.suit_type}`, {
					defaultValue: formatProfileTypeLabel(selected.suit_type),
				})
			: formatProfileTypeLabel(selected.suit_type ?? "");

		const details = [
			{
				label: t("profiles:form.clonePreview.labels.type", {
					defaultValue: "Type",
				}),
				value: typeValue,
			},
			{
				label: t("profiles:form.clonePreview.labels.priority", {
					defaultValue: "Priority",
				}),
				value: selected.priority ?? "—",
			},
			{
				label: t("profiles:form.clonePreview.labels.multiSelect", {
					defaultValue: "Multi-select",
				}),
				value: selected.multi_select
					? t("yes", { defaultValue: "Yes" })
					: t("no", { defaultValue: "No" }),
			},
			{
				label: t("profiles:form.clonePreview.labels.status", {
					defaultValue: "Status",
				}),
				value: selected.is_active
					? t("profiles:form.clonePreview.values.active", {
							defaultValue: "Active",
						})
					: t("profiles:form.clonePreview.values.inactive", {
							defaultValue: "Inactive",
						}),
			},
			{
				label: t("profiles:form.clonePreview.labels.servers", {
					defaultValue: "Servers",
				}),
				value: serverCount,
			},
			{
				label: t("profiles:form.clonePreview.labels.tools", {
					defaultValue: "Tools",
				}),
				value: toolCount,
			},
			{
				label: t("profiles:form.clonePreview.labels.resources", {
					defaultValue: "Resources",
				}),
				value: resourceCount,
			},
			{
				label: t("profiles:form.clonePreview.labels.prompts", {
					defaultValue: "Prompts",
				}),
				value: promptCount || "—",
			},
		];

		return (
			<div>
				<div className="grid grid-cols-1 gap-2 text-sm md:grid-cols-2">
					{details.map(({ label, value }) => (
						<div
							key={label}
							className="flex items-center justify-between rounded-md bg-slate-100 px-3 py-2 text-sm font-medium text-slate-700 dark:bg-slate-900/60 dark:text-slate-200"
						>
							<span className="text-xs tracking-wide text-muted-foreground">
								{label}
							</span>
							<span className="text-xs text-slate-900 dark:text-slate-100">
								{value}
							</span>
						</div>
					))}
				</div>
				{clonePreviewLoading && (
					<p className="mt-3 text-xs text-muted-foreground">
						{t("profiles:form.clonePreview.loading", {
							defaultValue: "Loading capabilities…",
						})}
					</p>
				)}
			</div>
		);
	}, [
		cloneableSuits,
		cloneContent,
		formData.clone_from_id,
		clonePreviewLoading,
		i18n.language,
	]);

	const selectedCloneProfile = useMemo(
		() =>
			formData.clone_from_id === "none"
				? undefined
				: cloneableSuits.find(
						(profile) => profile.id === formData.clone_from_id,
					),
		[cloneableSuits, formData.clone_from_id],
	);

	// Transfer 组件的处理函数
	const handleTransferChange = useCallback((targetKeys: string[]) => {
		setServerSelectionTouched(true);
		setSelectedServerIds(targetKeys);
	}, []);

	const handleServerInfo = useCallback(
		(item: TransferItem) => {
			// 跳转到服务器详情页面
			navigate(`/servers/${item.id}`);
		},
		[navigate],
	);

	const createModeSection = (
		<div className="space-y-4">
			<div className="flex items-center gap-4">
				<span className="w-32 text-sm font-medium text-slate-600 dark:text-slate-300">
					{t("profiles:form.fields.concurrentActivation", {
						defaultValue: "Concurrent Activation",
					})}
				</span>
				<div className="flex items-center gap-2">
					<Switch
						id={multiSelectId}
						checked={formData.multi_select}
						onCheckedChange={(checked) =>
							setFormData((prev) => ({
								...prev,
								multi_select: checked,
							}))
						}
					/>
					<Label htmlFor={multiSelectId} className="text-sm">
						{t("profiles:form.labels.concurrentActivationDescription", {
							defaultValue: "Allow concurrent activation with other profiles",
						})}
					</Label>
				</div>
			</div>

			<div className="flex items-center gap-4">
				<span className="w-32 text-sm font-medium text-slate-600 dark:text-slate-300">
					{t("profiles:form.fields.status", { defaultValue: "Status" })}
				</span>
				<div className="flex flex-wrap items-center gap-6">
					<div className="flex items-center gap-2">
						<Switch
							id={isActiveId}
							checked={formData.is_active}
							onCheckedChange={(checked) =>
								setFormData((prev) => ({
									...prev,
									is_active: checked,
								}))
							}
						/>
						<Label htmlFor={isActiveId} className="text-sm">
							{t("profiles:form.labels.activateImmediately", {
								defaultValue: "Activate immediately",
							})}
						</Label>
					</div>
					{showDefaultToggle && (
						<div className="flex items-center gap-2 hidden">
							<Switch
								id={isDefaultId}
								checked={formData.is_default}
								onCheckedChange={(checked) =>
									setFormData((prev) => ({
										...prev,
										is_default: checked,
									}))
								}
							/>
							<Label htmlFor={isDefaultId} className="text-sm">
								{t("profiles:form.labels.setAsDefault", {
									defaultValue: "Set as default profile",
								})}
							</Label>
						</div>
					)}
				</div>
			</div>

			<div className="mt-6 space-y-4">
				<div className="flex items-center gap-4">
					<Label
						htmlFor={cloneFromId}
						className="w-32 text-sm font-medium text-slate-600 dark:text-slate-300"
					>
						{t("profiles:form.fields.cloneFrom", {
							defaultValue: "Clone From",
						})}
					</Label>
					<Select
						value={formData.clone_from_id}
						onValueChange={(value) =>
							setFormData((prev) => ({
								...prev,
								clone_from_id: value,
							}))
						}
					>
						<SelectTrigger id={cloneFromId} className="flex-1">
							<SelectValue
								placeholder={t("profiles:form.labels.none", {
									defaultValue: "None",
								})}
							>
								{selectedCloneProfile?.name ??
									t("profiles:form.labels.none", { defaultValue: "None" })}
							</SelectValue>
						</SelectTrigger>
						<SelectContent>
							<SelectItem value="none">
								{t("profiles:form.labels.none", { defaultValue: "None" })}
							</SelectItem>
							{cloneableSuits.map((profile) => {
								const rawDescription = profile.description?.trim() ?? "";
								const truncatedDescription =
									rawDescription.length > 80
										? `${rawDescription.slice(0, 77).trimEnd()}…`
										: rawDescription;
								return (
									<SelectItem key={profile.id} value={profile.id}>
										<div className="flex w-full items-center justify-between gap-3">
											<span className="truncate text-sm font-medium text-slate-900 dark:text-slate-100">
												{profile.name}
											</span>
											{rawDescription ? (
												<span className="hidden text-xs text-muted-foreground sm:block sm:max-w-[220px] sm:truncate sm:text-right">
													{truncatedDescription}
												</span>
											) : (
												<span className="hidden text-xs text-muted-foreground sm:block sm:text-right">
													No description
												</span>
											)}
										</div>
									</SelectItem>
								);
							})}
						</SelectContent>
					</Select>
				</div>
				{clonePreview && (
					<div className="flex items-start gap-4">
						<span className="w-32" />
						<div className="flex-1">{clonePreview}</div>
					</div>
				)}
				<div className="flex items-start gap-4 pt-0">
					<span className="w-32" />
					<p className="flex-1 pl-1 text-xs text-muted-foreground">
						{t("profiles:form.clonePreview.description", {
							defaultValue:
								"Cloning copies enabled servers, tools, and resources from the source profile.",
						})}
					</p>
				</div>
			</div>
		</div>
	);

	const editModeSection = (
		<div className="space-y-4">
			<div className="flex items-center gap-4">
				<span className="w-32 text-sm font-medium text-slate-600 dark:text-slate-300">
					{t("profiles:form.fields.concurrentActivation", {
						defaultValue: "Concurrent Activation",
					})}
				</span>
				<div className="flex items-center gap-2">
					<Switch
						id={multiSelectId}
						checked={formData.multi_select}
						onCheckedChange={(checked) =>
							setFormData((prev) => ({
								...prev,
								multi_select: checked,
							}))
						}
					/>
					<Label htmlFor={multiSelectId} className="text-sm">
						{t("profiles:form.labels.concurrentActivationDescription", {
							defaultValue: "Allow concurrent activation with other profiles",
						})}
					</Label>
				</div>
			</div>

			<div className="flex items-center gap-4">
				<span className="w-32 text-sm font-medium text-slate-600 dark:text-slate-300">
					{t("profiles:form.fields.status", { defaultValue: "Status" })}
				</span>
				<div className="flex flex-wrap items-center gap-6">
					<div className="flex items-center gap-2">
						<Switch
							id={isActiveId}
							checked={formData.is_active}
							onCheckedChange={(checked) =>
								setFormData((prev) => ({
									...prev,
									is_active: checked,
								}))
							}
						/>
						<Label htmlFor={isActiveId} className="text-sm">
							{t("profiles:form.labels.activateImmediately", {
								defaultValue: "Activate immediately",
							})}
						</Label>
					</div>
					{showDefaultToggle && (
						<div className="flex items-center gap-2 hidden">
							<Switch
								id={isDefaultId}
								checked={formData.is_default}
								onCheckedChange={(checked) =>
									setFormData((prev) => ({
										...prev,
										is_default: checked,
									}))
								}
							/>
							<Label htmlFor={isDefaultId} className="text-sm">
								{t("profiles:form.labels.setAsDefault", {
									defaultValue: "Set as default profile",
								})}
							</Label>
						</div>
					)}
				</div>
			</div>
		</div>
	);

	const detailsModeContent =
		mode === "create" ? createModeSection : editModeSection;

	// 使用组合键确保每次打开时组件完全重新渲染
	const drawerKey = `suit-form-drawer-${mode}-${suit?.id || "new"}-${open ? "open" : "closed"}`;

	return (
		<Drawer
			key={drawerKey}
			open={open}
			onOpenChange={(open) => !open && handleOverlayClose()}
		>
			<DrawerContent className="h-full flex flex-col">
				<DrawerHeader>
					<DrawerTitle>
						{mode === "create"
							? t("profiles:form.title.create", {
									defaultValue: "Create New Profile",
								})
							: t("profiles:form.title.edit", { defaultValue: "Edit Profile" })}
					</DrawerTitle>
					<DrawerDescription>
						{mode === "create"
							? t("profiles:form.description.create", {
									defaultValue:
										"Create a new profile to organize your MCP servers and tools.",
								})
							: t("profiles:form.description.edit", {
									defaultValue: "Update the profile settings.",
								})}
					</DrawerDescription>
				</DrawerHeader>

				<form onSubmit={handleSubmit} className="flex h-full flex-col">
					{/* Content area - scrollable */}
					<div className="flex-1 overflow-y-auto overflow-x-hidden p-4 space-y-4">
						<div className="flex flex-wrap items-center gap-4">
							{steps.map((item, index) => {
								const isActive = step === item.id;
								const canNavigate =
									item.id === "details" ||
									(item.id === "servers" && detailsStepValid);

								return (
									<div key={item.id} className="flex items-center gap-2">
										<button
											type="button"
											onClick={() => {
												if (canNavigate && !isMutating) {
													setStep(item.id);
												}
											}}
											disabled={!canNavigate || isMutating}
											className={`flex h-7 w-7 items-center justify-center rounded-full text-xs font-semibold transition-colors ${
												isActive
													? "bg-primary text-primary-foreground"
													: canNavigate
														? "bg-slate-200 text-slate-600 hover:bg-slate-300 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-slate-700 cursor-pointer"
														: "bg-slate-100 text-slate-400 dark:bg-slate-900 dark:text-slate-500 cursor-not-allowed"
											}`}
										>
											{index + 1}
										</button>
										<button
											type="button"
											onClick={() => {
												if (canNavigate && !isMutating) {
													setStep(item.id);
												}
											}}
											disabled={!canNavigate || isMutating}
											className="flex flex-col text-left transition-colors hover:opacity-80 disabled:cursor-not-allowed disabled:opacity-50"
										>
											<span
												className={`text-sm font-medium ${
													isActive
														? "text-primary"
														: canNavigate
															? "text-slate-600 dark:text-slate-300"
															: "text-slate-400 dark:text-slate-500"
												}`}
											>
												{item.label}
											</span>
											<span className="text-xs text-muted-foreground">
												{item.hint}
											</span>
										</button>
										{index < steps.length - 1 && (
											<span className="hidden h-px w-10 bg-slate-200 md:block dark:bg-slate-800" />
										)}
									</div>
								);
							})}
						</div>

						{step === "details" && (
							<div className="space-y-4">
								<div className="flex items-center gap-4">
									<Label
										htmlFor={nameId}
										className="w-32 text-sm font-medium text-slate-600 dark:text-slate-300"
									>
										{t("profiles:form.fields.nameRequired", {
											defaultValue: "Name *",
										})}
									</Label>
									<Input
										id={nameId}
										value={formData.name}
										onChange={(e) =>
											setFormData((prev) => ({ ...prev, name: e.target.value }))
										}
										placeholder={t("profiles:form.placeholders.profileName", {
											defaultValue: "Enter profile name",
										})}
										required
										className="flex-1"
									/>
								</div>

								<div className="flex items-start gap-4">
									<Label
										htmlFor={descriptionId}
										className="w-32 text-sm font-medium text-slate-600 dark:text-slate-300"
									>
										{t("profiles:form.fields.description", {
											defaultValue: "Description",
										})}
									</Label>
									<Textarea
										id={descriptionId}
										value={formData.description}
										onChange={(e) =>
											setFormData((prev) => ({
												...prev,
												description: e.target.value,
											}))
										}
										placeholder={t(
											"profiles:form.placeholders.profileDescription",
											{ defaultValue: "Provide a short summary" },
										)}
										rows={3}
										className="flex-1"
									/>
								</div>

								{detailsModeContent}
							</div>
						)}
						{step === "servers" && (
							<div className="flex flex-col flex-1 space-y-4">
								<div>
									<p className="text-xs text-muted-foreground">
										{t("profiles:form.serverSelection.title", {
											defaultValue:
												"Choose which servers belong to this profile. Server enable/disable status is managed separately.",
										})}{" "}
										{selectedServerCount}{" "}
										{t("profiles:form.serverSelection.assigned", {
											defaultValue: "servers assigned",
										})}
										, {totalServerCount}{" "}
										{t("profiles:form.serverSelection.available", {
											defaultValue: "available servers",
										})}
									</p>
								</div>

								<div className="flex-1 flex">
									{isServersStepLoading ? (
										<div className="flex-1 flex items-center justify-center rounded-lg border border-dashed border-slate-200 text-sm text-muted-foreground dark:border-slate-800">
											<div className="flex items-center gap-2">
												<Loader2 className="h-4 w-4 animate-spin" />
												{t("profiles:form.serverSelection.loading", {
													defaultValue: "Loading server list…",
												})}
											</div>
										</div>
									) : totalServerCount === 0 ? (
										<div className="flex-1 flex items-center justify-center rounded-lg border border-dashed border-slate-200 text-center text-sm text-muted-foreground dark:border-slate-800">
											{t("profiles:form.serverSelection.noAvailable", {
												defaultValue: "No available servers",
											})}
										</div>
									) : (
										<Transfer
											dataSource={transferDataSource}
											targetKeys={selectedServerIds}
											onChange={handleTransferChange}
											onItemInfo={handleServerInfo}
											leftTitle={t(
												"profiles:form.serverSelection.availableServers",
												{ defaultValue: "Available Servers" },
											)}
											rightTitle={t(
												"profiles:form.serverSelection.profileServers",
												{ defaultValue: "Profile Servers" },
											)}
											searchPlaceholder={t(
												"profiles:form.placeholders.searchServers",
												{ defaultValue: "Search servers..." },
											)}
											emptyText={t("profiles:form.serverSelection.noData", {
												defaultValue: "No data",
											})}
											disabled={isMutating}
											loading={isServersStepLoading}
											className="flex-1"
										/>
									)}
								</div>
							</div>
						)}
					</div>

					<DrawerFooter className="border-t bg-background">
						<div className="flex w-full flex-wrap items-center justify-between gap-2">
							<div className="flex gap-2">
								<Button
									type="button"
									variant="outline"
									onClick={() => {
										if (step === "details") {
											closeDrawer();
										} else {
											setStep("details");
										}
									}}
									disabled={isMutating}
								>
									{step === "details"
										? t("profiles:form.buttons.cancel", {
												defaultValue: "Cancel",
											})
										: t("profiles:form.buttons.back", { defaultValue: "Back" })}
								</Button>
								{step === "servers" && (
									<Button
										type="button"
										variant="ghost"
										onClick={closeDrawer}
										disabled={isMutating}
									>
										{t("profiles:form.buttons.cancel", {
											defaultValue: "Cancel",
										})}
									</Button>
								)}
							</div>
							<Button
								type="submit"
								disabled={primaryDisabled}
								className="min-w-[140px]"
							>
								{primaryLabel}
							</Button>
						</div>
					</DrawerFooter>
				</form>
			</DrawerContent>
		</Drawer>
	);
}
