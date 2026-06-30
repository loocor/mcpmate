import { useState, useCallback, useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
	Plus,
	Pencil,
	RefreshCw,
	Trash2,
	Zap,
	Loader2,
	CheckCircle,
	XCircle,
	Star,
	HelpCircle,
} from "lucide-react";

import { llmApi } from "../../lib/api";
import type {
	LlmConnectivityResult,
	LlmProviderConfig,
	LlmProviderCreateInput,
	LlmProviderThinkingMode,
	SecretOrigin,
} from "../../lib/types";
import { REDACTED_FULL, sanitizeStringForSave } from "../../lib/secure-field";
import {
	InlineSecretCreate,
	useInlineSecretCreateField,
} from "../../components/secrets/inline-secret-create";
import { Button } from "../../components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { Badge } from "../../components/ui/badge";
import { SecureStringField } from "../../components/server-install/secure-string-field";
import {
	Drawer,
	DrawerContent,
	DrawerDescription,
	DrawerFooter,
	DrawerHeader,
	DrawerTitle,
} from "../../components/ui/drawer";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../../components/ui/tooltip";

type ProviderType = "openai_chat" | "anthropic";

type SettingsT = ReturnType<typeof useTranslation>["t"];

const PROVIDER_TYPE_OPTIONS: Array<{
	value: ProviderType;
	labelKey: string;
	fallbackLabel: string;
}> = [
		{
			value: "openai_chat",
			labelKey: "settings:providers.providerTypes.openaiChat",
			fallbackLabel: "OpenAI Chat Completions",
		},
		{
			value: "anthropic",
			labelKey: "settings:providers.providerTypes.anthropic",
			fallbackLabel: "Anthropic Messages",
		},
	];

const DEFAULT_BASE_URLS: Record<ProviderType, string> = {
	openai_chat: "https://api.openai.com/v1",
	anthropic: "https://api.anthropic.com",
};

const DEFAULT_MODEL_IDS: Record<ProviderType, string> = {
	openai_chat: "gpt-4o",
	anthropic: "claude-sonnet-4-20250514",
};

const DEFAULT_MAX_TOKENS = 4096;
const ANTHROPIC_THINKING_TOKEN_RESERVE = 1000;

const PROVIDER_TYPE_BADGE_LABELS: Record<string, { key: string; fallback: string }> = {
	openai_chat: {
		key: "settings:providers.providerTypeBadges.openaiChat",
		fallback: "OpenAI Chat",
	},
	openai_compatible: {
		key: "settings:providers.providerTypeBadges.openaiChat",
		fallback: "OpenAI Chat",
	},
	openai_responses: {
		key: "settings:providers.providerTypeBadges.openaiResponses",
		fallback: "OpenAI Responses",
	},
	anthropic: {
		key: "settings:providers.providerTypeBadges.anthropic",
		fallback: "Anthropic",
	},
};

const THINKING_MODE_OPTIONS: Array<{
	value: LlmProviderThinkingMode;
	labelKey: string;
	fallbackLabel: string;
	descriptionKey: string;
	fallbackDescription: string;
}> = [
		{
			value: "default",
			labelKey: "settings:providers.thinkingModes.default.label",
			fallbackLabel: "Provider default",
			descriptionKey: "settings:providers.thinkingModes.default.description",
			fallbackDescription: "Do not send a thinking control field.",
		},
		{
			value: "disabled",
			labelKey: "settings:providers.thinkingModes.disabled.label",
			fallbackLabel: "Disabled",
			descriptionKey: "settings:providers.thinkingModes.disabled.description",
			fallbackDescription: 'Send Anthropic thinking: { type: "disabled" }.',
		},
		{
			value: "enabled",
			labelKey: "settings:providers.thinkingModes.enabled.label",
			fallbackLabel: "Enabled",
			descriptionKey: "settings:providers.thinkingModes.enabled.description",
			fallbackDescription: 'Send Anthropic thinking: { type: "enabled", budget_tokens }.',
		},
	];

const PROVIDER_ACTION_BUTTON_CLASS = "h-8 w-8 p-0";
const PROVIDER_ACTION_ICON_CLASS = "h-4 w-4";
const PROVIDER_ACTION_SLOT_CLASS = "inline-flex h-8 w-8 items-center justify-center";

function providerTestErrorMessage(error: unknown): string {
	if (error instanceof Error) return error.message;
	return String(error);
}

function providerTypeBadgeLabel(providerType: string, t: SettingsT): string {
	const label = PROVIDER_TYPE_BADGE_LABELS[providerType];
	if (!label) return providerType;
	return t(label.key, label.fallback);
}

function providerTestFailureMessage(
	result: LlmConnectivityResult | undefined,
	error: unknown,
	t: ReturnType<typeof useTranslation>["t"],
): string | null {
	if (result?.success === false) {
		return (
			result.error || t("settings:providers.testFailed", "Provider test failed")
		);
	}
	if (error) {
		return providerTestErrorMessage(error);
	}
	return null;
}

export function ProvidersSettings() {
	const { t } = useTranslation();
	const queryClient = useQueryClient();
	const [editingProvider, setEditingProvider] = useState<LlmProviderConfig | null>(null);
	const [isDrawerOpen, setIsDrawerOpen] = useState(false);

	const { data: providers = [], isLoading } = useQuery({
		queryKey: ["llm-providers"],
		queryFn: () => llmApi.listProviders(),
	});

	const deleteMutation = useMutation({
		mutationFn: (id: string) => llmApi.deleteProvider(id),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["llm-providers"] });
		},
	});

	const setDefaultMutation = useMutation({
		mutationFn: (id: string) => llmApi.setDefaultProvider(id),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["llm-providers"] });
		},
	});

	const handleAdd = useCallback(() => {
		setEditingProvider(null);
		setIsDrawerOpen(true);
	}, []);

	const handleEdit = useCallback((provider: LlmProviderConfig) => {
		setEditingProvider(provider);
		setIsDrawerOpen(true);
	}, []);

	const handleDelete = useCallback(
		(id: string, closeDrawer = false) => {
			if (confirm(t("settings:providers.confirmDelete", "Delete this provider?"))) {
				deleteMutation.mutate(id, {
					onSuccess: () => {
						if (closeDrawer) {
							setIsDrawerOpen(false);
							setEditingProvider(null);
						}
					},
				});
			}
		},
		[deleteMutation, t],
	);

	return (
		<Card className="h-full">
			<CardHeader>
				<div className="space-y-1.5">
					<CardTitle>
						{t("settings:providers.title", "LLM Providers")}
					</CardTitle>
					<CardDescription>
						{t(
							"settings:providers.description",
							"Configure LLM providers for LLM-powered workflows",
						)}
					</CardDescription>
				</div>
			</CardHeader>
			<CardContent>
				{isLoading ? (
					<div className="flex items-center justify-center py-8">
						<Loader2 className="h-6 w-6 animate-spin" />
					</div>
				) : providers.length === 0 ? (
					<div className="flex flex-col items-center justify-center py-12 text-center">
						<Zap className="h-10 w-10 text-muted-foreground/50 mb-3" />
						<p className="text-sm text-muted-foreground mb-4">
							{t(
								"settings:providers.empty",
								"No providers configured yet. Add an LLM provider to enable LLM-powered workflows.",
							)}
						</p>
						<Button variant="outline" onClick={handleAdd} size="sm">
							<Plus className="mr-2 h-4 w-4" />
							{t("settings:providers.addFirst", "Add Your First Provider")}
						</Button>
					</div>
				) : (
					<div className="space-y-4">
						<div className="space-y-3">
							{providers.map((provider) => (
								<ProviderCard
									key={provider.id}
									provider={provider}
									onEdit={handleEdit}
									onSetDefault={(id) => setDefaultMutation.mutate(id)}
								/>
							))}
						</div>
						<div className="flex justify-end">
							<Button variant="outline" onClick={handleAdd} size="sm">
								<Plus className="mr-2 h-4 w-4" />
								{t("settings:providers.add", "Add Provider")}
							</Button>
						</div>
					</div>
				)}
			</CardContent>

			<ProviderDrawer
				open={isDrawerOpen}
				onOpenChange={setIsDrawerOpen}
				provider={editingProvider}
				isDeleting={deleteMutation.isPending}
				onDelete={(id) => handleDelete(id, true)}
				onSuccess={() => {
					queryClient.invalidateQueries({ queryKey: ["llm-providers"] });
					setIsDrawerOpen(false);
				}}
			/>
		</Card>
	);
}

function ProviderCard({
	provider,
	onEdit,
	onSetDefault,
}: {
	provider: LlmProviderConfig;
	onEdit: (p: LlmProviderConfig) => void;
	onSetDefault: (id: string) => void;
}) {
	const { t } = useTranslation();
	const queryClient = useQueryClient();

	const testMutation = useMutation({
		mutationFn: () => llmApi.testProvider(provider.id),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["llm-providers"] });
		},
	});
	const testError = providerTestFailureMessage(
		testMutation.data,
		testMutation.isError ? testMutation.error : null,
		t,
	);
	const successfulTestResult =
		testMutation.isSuccess && testMutation.data?.success
			? testMutation.data
			: null;
	const showEmptyTestResult = !successfulTestResult && !testError;

	return (
		<div className="flex items-center justify-between p-4 border rounded-lg hover:bg-muted/30 transition-colors">
			<div className="flex-1 min-w-0">
				<div className="flex items-center gap-2">
					<span className="font-medium truncate">{provider.name}</span>
					<Badge variant="outline" className="shrink-0 text-xs">
						{providerTypeBadgeLabel(provider.provider_type, t)}
					</Badge>
					{provider.is_default && (
						<Badge variant="secondary" className="shrink-0 text-xs gap-1">
							<Star className="h-3 w-3 fill-current" />
							{t("settings:providers.default", "Default")}
						</Badge>
					)}
					{!provider.has_api_key && (
						<Badge variant="secondary" className="shrink-0 text-xs">
							{t("settings:providers.noKey", "No API Key")}
						</Badge>
					)}
				</div>
				<div className="text-sm text-muted-foreground mt-1 truncate">
					{provider.base_url}
					<span className="mx-1.5">·</span>
					{provider.model_id}
				</div>
			</div>
			<div className="ml-4 flex items-center gap-1">
				{successfulTestResult && (
					<span className="inline-flex h-8 items-center justify-end text-xs tabular-nums text-green-600">
						{successfulTestResult.latency_ms}ms
					</span>
				)}
				{successfulTestResult && (
					<TooltipProvider delayDuration={200}>
						<Tooltip>
							<TooltipTrigger asChild>
								<span className={PROVIDER_ACTION_SLOT_CLASS}>
									<CheckCircle className={`${PROVIDER_ACTION_ICON_CLASS} text-green-500`} />
								</span>
							</TooltipTrigger>
							<TooltipContent side="top">
								{t("settings:providers.connected", "Connected")}
							</TooltipContent>
						</Tooltip>
					</TooltipProvider>
				)}
				{testError ? (
					<TooltipProvider delayDuration={200}>
						<Tooltip>
							<TooltipTrigger asChild>
								<span className={PROVIDER_ACTION_SLOT_CLASS}>
									<XCircle className={`${PROVIDER_ACTION_ICON_CLASS} text-red-500`} />
								</span>
							</TooltipTrigger>
							<TooltipContent side="top" className="max-w-sm break-words">
								{testError}
							</TooltipContent>
						</Tooltip>
					</TooltipProvider>
				) : null}
				{showEmptyTestResult && <span aria-hidden className={PROVIDER_ACTION_SLOT_CLASS} />}
				{!provider.is_default && (
					<Button
						variant="ghost"
						size="sm"
						className={PROVIDER_ACTION_BUTTON_CLASS}
						onClick={() => onSetDefault(provider.id)}
						title={t("settings:providers.setDefault", "Set as default")}
					>
						<Star className={PROVIDER_ACTION_ICON_CLASS} />
					</Button>
				)}
				<Button
					variant="ghost"
					size="sm"
					className={PROVIDER_ACTION_BUTTON_CLASS}
					onClick={() => testMutation.mutate()}
					disabled={testMutation.isPending}
					title={t("settings:providers.testConnection", "Test connection")}
				>
					{testMutation.isPending ? (
						<Loader2 className={`${PROVIDER_ACTION_ICON_CLASS} animate-spin`} />
					) : (
						<Zap className={PROVIDER_ACTION_ICON_CLASS} />
					)}
				</Button>
				<Button
					variant="ghost"
					size="sm"
					className={PROVIDER_ACTION_BUTTON_CLASS}
					onClick={() => onEdit(provider)}
					title={t("settings:providers.edit", "Edit")}
				>
					<Pencil className={PROVIDER_ACTION_ICON_CLASS} />
				</Button>
			</div>
		</div>
	);
}

function ProviderDrawer({
	open,
	onOpenChange,
	provider,
	isDeleting,
	onDelete,
	onSuccess,
}: {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	provider: LlmProviderConfig | null;
	isDeleting: boolean;
	onDelete: (id: string) => void;
	onSuccess: () => void;
}) {
	const { t } = useTranslation();
	const isEditing = !!provider;

	const [name, setName] = useState("");
	const [providerType, setProviderType] = useState<ProviderType>("openai_chat");
	const [baseUrl, setBaseUrl] = useState("");
	const [modelId, setModelId] = useState("");
	const [apiKey, setApiKey] = useState("");
	const [thinkingMode, setThinkingMode] = useState<LlmProviderThinkingMode>("default");
	const [thinkingBudgetTokens, setThinkingBudgetTokens] = useState("1024");
	const [modelOptions, setModelOptions] = useState<string[]>([]);
	const [modelError, setModelError] = useState<string | null>(null);
	const [testState, setTestState] = useState<"idle" | "loading" | "success" | "error">("idle");
	const [drawerTestError, setDrawerTestError] = useState<string | null>(null);
	const [isLoadingModels, setIsLoadingModels] = useState(false);
	const resetTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
	const { onCreateSecret, controller: secretCreateController } = useInlineSecretCreateField(
		(fieldName, placeholder) => {
			if (fieldName === "api_key") {
				setApiKey(placeholder);
			}
		},
	);

	// Clean up timer on unmount
	useEffect(() => {
		return () => {
			if (resetTimerRef.current) clearTimeout(resetTimerRef.current);
		};
	}, []);

	useEffect(() => {
		if (open) {
			if (provider) {
				setName(provider.name);
				setProviderType(
					provider.provider_type === "openai_compatible"
						? "openai_chat"
						: (provider.provider_type as ProviderType),
				);
				setBaseUrl(provider.base_url);
				setModelId(provider.model_id);
				setApiKey(provider.has_api_key ? REDACTED_FULL : "");
				setThinkingMode(provider.default_params.thinking?.mode ?? "default");
				setThinkingBudgetTokens(String(provider.default_params.thinking?.budget_tokens ?? 1024));
			} else {
				setName("");
				setProviderType("openai_chat");
				setBaseUrl(DEFAULT_BASE_URLS.openai_chat);
				setModelId("");
				setApiKey("");
				setThinkingMode("default");
				setThinkingBudgetTokens("1024");
			}
			setModelOptions([]);
			setModelError(null);
			setTestState("idle");
			setDrawerTestError(null);
			if (resetTimerRef.current) clearTimeout(resetTimerRef.current);
		}
	}, [open, provider]);

	const createMutation = useMutation({
		mutationFn: (input: LlmProviderCreateInput) => llmApi.createProvider(input),
		onSuccess,
	});

	const updateMutation = useMutation({
		mutationFn: (input: Parameters<typeof llmApi.updateProvider>[0]) => llmApi.updateProvider(input),
		onSuccess,
	});

	const handleTest = useCallback(async () => {
		if (!provider?.id || testState === "loading") return;
		setTestState("loading");
		setDrawerTestError(null);
		try {
			const result = await llmApi.testProvider(provider.id);
			setTestState(result.success ? "success" : "error");
			setDrawerTestError(result.success ? null : (result.error ?? null));
		} catch (error) {
			setTestState("error");
			setDrawerTestError(providerTestErrorMessage(error));
		}
		// Reset to idle after 2 seconds
		resetTimerRef.current = setTimeout(() => {
			setTestState("idle");
			setDrawerTestError(null);
		}, 2000);
	}, [provider, testState]);

	const handleDelete = useCallback(() => {
		if (!provider?.id || isDeleting) return;
		onDelete(provider.id);
	}, [isDeleting, onDelete, provider]);

	const apiKeyOrigin = useMemo<SecretOrigin>(
		() => ({
			source: "llm_provider",
			server_name: name.trim() || provider?.name || t("settings:providers.originName", "LLM Provider"),
			field_group: "provider",
			field_key: "api_key",
		}),
		[name, provider?.name, t],
	);

	const handleCreateApiKeySecret = useCallback(
		(origin: SecretOrigin) => {
			onCreateSecret("api_key", origin);
		},
		[onCreateSecret],
	);

	const canFetchModels = Boolean(providerType && baseUrl.trim() && apiKey.trim());
	const modelFetchButtonVisibilityClass =
		canFetchModels || isLoadingModels
			? "opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
			: "pointer-events-none invisible opacity-0";

	const handleFetchModels = useCallback(async () => {
		if (isLoadingModels || !canFetchModels) return;
		setIsLoadingModels(true);
		setModelError(null);
		try {
			const trimmedApiKey = apiKey.trim();
			const trimmedBaseUrl = baseUrl.trim();
			const sanitizedApiKey = sanitizeStringForSave(trimmedApiKey);
			const providerIdForStoredKey =
				isEditing && provider?.id && trimmedApiKey === REDACTED_FULL
					? provider.id
					: undefined;
			const models = await llmApi.listModelsForConfig({
				provider_id: providerIdForStoredKey,
				provider_type: providerType,
				base_url: trimmedBaseUrl || DEFAULT_BASE_URLS[providerType],
				model_id: modelId.trim() || DEFAULT_MODEL_IDS[providerType],
				api_key: sanitizedApiKey,
			});
			setModelOptions(models);
			if (!modelId.trim() && models[0]) {
				setModelId(models[0]);
			}
		} catch (error) {
			setModelOptions([]);
			setModelError(error instanceof Error ? error.message : String(error));
		} finally {
			setIsLoadingModels(false);
		}
	}, [apiKey, baseUrl, canFetchModels, isEditing, isLoadingModels, modelId, provider, providerType]);

	const handleSubmit = useCallback(() => {
		const trimmedName = name.trim();
		const trimmedBaseUrl = baseUrl.trim();
		const trimmedModelId = modelId.trim();
		const trimmedApiKey = apiKey.trim();
		const sanitizedApiKey = sanitizeStringForSave(trimmedApiKey);
		const submittedApiKey =
			isEditing && trimmedApiKey.length === 0 && provider?.has_api_key
				? null
				: sanitizedApiKey;
		const supportsThinking = providerType === "anthropic";
		const submittedThinkingBudget = Number.parseInt(thinkingBudgetTokens, 10);
		const thinkingBudget =
			supportsThinking && thinkingMode === "enabled" && Number.isFinite(submittedThinkingBudget)
				? submittedThinkingBudget
				: undefined;
		const defaultParams = {
			thinking: {
				mode: supportsThinking ? thinkingMode : ("default" as const),
				budget_tokens: thinkingBudget,
			},
		};

		if (isEditing && provider) {
			updateMutation.mutate({
				id: provider.id,
				name: trimmedName || undefined,
				provider_type: providerType,
				base_url: trimmedBaseUrl || undefined,
				model_id: trimmedModelId || undefined,
				api_key: submittedApiKey,
				default_params: defaultParams,
			});
		} else {
			createMutation.mutate({
				name: trimmedName,
				provider_type: providerType,
				base_url: trimmedBaseUrl || DEFAULT_BASE_URLS[providerType],
				model_id: trimmedModelId,
				api_key: sanitizedApiKey,
				default_params: defaultParams,
			});
		}
	}, [
		name,
		providerType,
		baseUrl,
		modelId,
		apiKey,
		thinkingMode,
		thinkingBudgetTokens,
		isEditing,
		provider,
		createMutation,
		updateMutation,
	]);

	const isPending = createMutation.isPending || updateMutation.isPending;
	const supportsThinking = providerType === "anthropic";
	const effectiveMaxTokens = provider?.default_params.max_tokens ?? DEFAULT_MAX_TOKENS;
	const parsedThinkingBudget = Number.parseInt(thinkingBudgetTokens, 10);
	const hasValidThinkingBudget =
		!supportsThinking ||
		thinkingMode !== "enabled" ||
		(Number.isFinite(parsedThinkingBudget) &&
			parsedThinkingBudget > 0 &&
			parsedThinkingBudget + ANTHROPIC_THINKING_TOKEN_RESERVE <= effectiveMaxTokens);
	const canSubmit = name.trim() && modelId.trim() && hasValidThinkingBudget;
	const testButton = (
		<Button
			type="button"
			variant="ghost"
			size="icon"
			className="-mr-1 -mt-1 h-5 w-5 shrink-0 rounded-md border-0 bg-transparent p-0 text-muted-foreground shadow-none transition-colors hover:bg-transparent hover:text-foreground focus-visible:ring-1 focus-visible:ring-offset-0"
			onClick={handleTest}
			disabled={testState === "loading"}
			aria-label={t("settings:providers.testConnection", "Test connection")}
			title={t("settings:providers.testConnection", "Test connection")}
		>
			{testState === "loading" ? (
				<Loader2 className="h-4 w-4 animate-spin" />
			) : testState === "success" ? (
				<CheckCircle className="h-4 w-4 text-green-600" />
			) : testState === "error" ? (
				<XCircle className="h-4 w-4 text-red-600" />
			) : (
				<Zap className="h-4 w-4" />
			)}
		</Button>
	);

	return (
		<Drawer open={open} onOpenChange={onOpenChange}>
			<DrawerContent className="flex h-full flex-col overflow-hidden">
				<DrawerHeader className="shrink-0 pb-2 text-left">
					<div className="flex items-start justify-between gap-3">
						<div className="min-w-0 flex-1 space-y-1 text-left">
							<DrawerTitle className="text-left">
								{isEditing
									? t("settings:providers.editTitle", "Edit Provider")
									: t("settings:providers.addTitle", "Add Provider")}
							</DrawerTitle>
							<DrawerDescription className="text-left">
								{t(
									"settings:providers.dialogDescription",
									"Configure an LLM provider for provider-backed LLM workflows",
								)}
							</DrawerDescription>
						</div>
						{isEditing && drawerTestError ? (
							<TooltipProvider delayDuration={200}>
								<Tooltip>
									<TooltipTrigger asChild>{testButton}</TooltipTrigger>
									<TooltipContent side="left" className="max-w-sm break-words">
										{drawerTestError}
									</TooltipContent>
								</Tooltip>
							</TooltipProvider>
						) : isEditing ? (
							testButton
						) : null}
					</div>
				</DrawerHeader>
				<div className="flex min-h-0 flex-1 flex-col space-y-4 overflow-y-auto px-4 pb-4 pt-2">
					<div className="flex items-center gap-4">
						<Label htmlFor="name" className="w-20 shrink-0 text-right">
							{t("settings:providers.name", "Name")}
						</Label>
						<div className="flex-1">
							<Input
								id="name"
								value={name}
								onChange={(e) => setName(e.target.value)}
								placeholder={t(
									"settings:providers.namePlaceholder",
									"My OpenAI Provider",
								)}
							/>
						</div>
					</div>

					<div className="flex items-center gap-4">
						<Label className="w-20 shrink-0 text-right">
							{t("settings:providers.type", "API Type")}
						</Label>
						<div className="flex-1">
							<Select
								value={providerType}
								onValueChange={(v) => setProviderType(v as ProviderType)}
							>
								<SelectTrigger>
									<SelectValue />
								</SelectTrigger>
								<SelectContent>
									{PROVIDER_TYPE_OPTIONS.map((opt) => (
										<SelectItem key={opt.value} value={opt.value}>
											<span>{t(opt.labelKey, opt.fallbackLabel)}</span>
										</SelectItem>
									))}
								</SelectContent>
							</Select>
						</div>
					</div>

					<div className="flex items-center gap-4">
						<Label htmlFor="baseUrl" className="w-20 shrink-0 text-right">
							{t("settings:providers.baseUrl", "Base URL")}
						</Label>
						<div className="flex-1">
							<Input
								id="baseUrl"
								value={baseUrl}
								onChange={(e) => setBaseUrl(e.target.value)}
								placeholder={DEFAULT_BASE_URLS[providerType]}
							/>
						</div>
					</div>

					<div className="flex items-center gap-4">
						<Label htmlFor="apiKey" className="w-20 shrink-0 text-right">
							{t("settings:providers.apiKey", "API Key")}
						</Label>
						<div className="flex-1">
							<SecureStringField
								id="apiKey"
								value={apiKey}
								onChange={setApiKey}
								placeholder={
									isEditing
										? t(
											"settings:providers.apiKeyEditPlaceholder",
											"Leave empty to keep current key",
										)
										: "sk-..."
								}
								origin={apiKeyOrigin}
								onCreateSecret={handleCreateApiKeySecret}
							/>
						</div>
					</div>

					{supportsThinking ? (
						<div className="flex items-center gap-4">
							<div className="flex w-20 shrink-0 justify-end">
								<TooltipProvider delayDuration={200}>
									<Tooltip>
										<TooltipTrigger asChild>
											<button
												type="button"
												className="inline-flex items-center gap-1 text-right text-sm font-medium leading-none text-foreground outline-none focus-visible:ring-1 focus-visible:ring-ring"
											>
												{t("settings:providers.thinking", "Thinking")}
												<HelpCircle className="h-3.5 w-3.5 text-muted-foreground" />
											</button>
										</TooltipTrigger>
										<TooltipContent side="left" className="max-w-xs space-y-1 text-left">
											{THINKING_MODE_OPTIONS.map((option) => (
												<div key={option.value}>
													<span className="font-medium">
														{t(option.labelKey, option.fallbackLabel)}
													</span>
													<span className="text-muted-foreground">
														: {t(option.descriptionKey, option.fallbackDescription)}
													</span>
												</div>
											))}
										</TooltipContent>
									</Tooltip>
								</TooltipProvider>
							</div>
							<div className="min-w-0 flex-1">
								<div className="flex gap-2">
									<Select
										value={thinkingMode}
										onValueChange={(value) => setThinkingMode(value as LlmProviderThinkingMode)}
									>
										<SelectTrigger className="min-w-0 flex-1">
											<SelectValue />
										</SelectTrigger>
										<SelectContent>
											{THINKING_MODE_OPTIONS.map((option) => (
												<SelectItem key={option.value} value={option.value}>
													<span>{t(option.labelKey, option.fallbackLabel)}</span>
												</SelectItem>
											))}
										</SelectContent>
									</Select>
									{thinkingMode === "enabled" ? (
										<Input
											type="number"
											min={1}
											step={1}
											value={thinkingBudgetTokens}
											onChange={(event) => setThinkingBudgetTokens(event.target.value)}
											className="w-32"
											aria-label={t("settings:providers.thinkingBudget", "Thinking budget tokens")}
										/>
									) : null}
								</div>
								{!hasValidThinkingBudget ? (
									<p className="mt-2 text-xs text-destructive">
										{t(
											"settings:providers.thinkingBudgetRequired",
											"Budget tokens must be positive and less than max tokens.",
										)}
									</p>
								) : null}
							</div>
						</div>
					) : null}

					<div className="flex items-center gap-4">
						<Label htmlFor="modelId" className="w-20 shrink-0 text-right">
							{t("settings:providers.model", "Model")}
						</Label>
						<div className="min-w-0 flex-1 space-y-2">
							<div className="group relative">
								<Input
									id="modelId"
									value={modelId}
									onChange={(e) => setModelId(e.target.value)}
									className="w-full pr-10"
									placeholder={
										providerType === "anthropic"
											? "claude-sonnet-4-20250514"
											: "gpt-4o"
									}
								/>
								<Button
									type="button"
									variant="ghost"
									size="icon"
									disabled={isLoadingModels || !canFetchModels}
									onClick={handleFetchModels}
									className={`absolute right-1 top-1/2 h-8 w-8 -translate-y-1/2 ${isLoadingModels
										? "opacity-100"
										: modelFetchButtonVisibilityClass
										}`}
									title={t("settings:providers.fetchModels", "Fetch models")}
								>
									{isLoadingModels ? (
										<Loader2 className="h-4 w-4 animate-spin" />
									) : (
										<RefreshCw className="h-4 w-4" />
									)}
								</Button>
							</div>
						</div>
					</div>

					{modelOptions.length > 0 || modelError ? (
						<div className="flex items-start gap-4">
							<div className="w-20 shrink-0" />
							<div className="min-w-0 flex-1 space-y-2">
								{modelOptions.length > 0 ? (
									<div className="flex flex-wrap gap-1.5">
										{modelOptions.slice(0, 8).map((model) => (
											<Button
												key={model}
												type="button"
												variant={modelId === model ? "secondary" : "outline"}
												size="sm"
												className="h-7 px-2 text-xs"
												onClick={() => setModelId(model)}
											>
												{model}
											</Button>
										))}
									</div>
								) : null}
								{modelError ? (
									<p className="text-xs text-destructive">{modelError}</p>
								) : null}
							</div>
						</div>
					) : null}
				</div>
				<DrawerFooter className="mt-auto shrink-0 border-t px-6 py-4">
					<div className="flex w-full items-center justify-between gap-3">
						<Button variant="outline" onClick={() => onOpenChange(false)}>
							{t("settings:providers.cancel", "Cancel")}
						</Button>
						<div className="flex items-center gap-3">
							{isEditing && (
								<Button
									type="button"
									variant="destructive"
									onClick={handleDelete}
									disabled={isDeleting || isPending}
								>
									{isDeleting ? (
										<Loader2 className="mr-2 h-4 w-4 animate-spin" />
									) : (
										<Trash2 className="mr-2 h-4 w-4" />
									)}
									{t("settings:providers.delete", "Delete")}
								</Button>
							)}
							<Button onClick={handleSubmit} disabled={isPending || !canSubmit}>
								{isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
								{isEditing
									? t("settings:providers.save", "Save")
									: t("settings:providers.create", "Create")}
							</Button>
						</div>
					</div>
				</DrawerFooter>
				<InlineSecretCreate controller={secretCreateController} nested />
			</DrawerContent>
		</Drawer>
	);
}
