import { useState, useCallback, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
	Plus,
	Pencil,
	Trash2,
	Zap,
	Loader2,
	CheckCircle,
	XCircle,
	Star,
} from "lucide-react";

import { llmApi } from "../../lib/api";
import type { LlmProviderConfig, LlmProviderCreateInput } from "../../lib/types";
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
import {
	Drawer,
	DrawerClose,
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

type ProviderType = "openai_chat" | "anthropic";

const PROVIDER_TYPE_OPTIONS: { value: ProviderType; label: string; desc: string }[] = [
	{
		value: "openai_chat",
		label: "OpenAI Chat Completions",
		desc: "/v1/chat/completions — OpenAI, Ollama, vLLM, Groq, Together, etc.",
	},
	{
		value: "anthropic",
		label: "Anthropic Messages",
		desc: "/v1/messages — Claude via Anthropic API",
	},
];

const DEFAULT_BASE_URLS: Record<ProviderType, string> = {
	openai_chat: "https://api.openai.com/v1",
	anthropic: "https://api.anthropic.com",
};

const PROVIDER_TYPE_LABELS: Record<string, string> = {
	openai_chat: "OpenAI Chat",
	openai_compatible: "OpenAI Chat",
	openai_responses: "OpenAI Responses",
	anthropic: "Anthropic",
};

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
		(id: string) => {
			if (confirm(t("settings.providers.confirmDelete", "Delete this provider?"))) {
				deleteMutation.mutate(id);
			}
		},
		[deleteMutation, t],
	);

	return (
		<Card className="h-full">
			<CardHeader>
				<div className="flex items-center justify-between">
					<div>
						<CardTitle>
							{t("settings.providers.title", "LLM Providers")}
						</CardTitle>
						<CardDescription>
							{t(
								"settings.providers.description",
								"Configure AI providers for intelligent test generation and analysis",
							)}
						</CardDescription>
					</div>
					<Button onClick={handleAdd} size="sm">
						<Plus className="mr-2 h-4 w-4" />
						{t("settings.providers.add", "Add Provider")}
					</Button>
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
								"settings.providers.empty",
								"No providers configured yet. Add an LLM provider to enable intelligent test generation.",
							)}
						</p>
						<Button variant="outline" onClick={handleAdd} size="sm">
							<Plus className="mr-2 h-4 w-4" />
							{t("settings.providers.addFirst", "Add Your First Provider")}
						</Button>
					</div>
				) : (
					<div className="space-y-3">
						{providers.map((provider) => (
							<ProviderCard
								key={provider.id}
								provider={provider}
								onEdit={handleEdit}
								onDelete={handleDelete}
								onSetDefault={(id) => setDefaultMutation.mutate(id)}
							/>
						))}
					</div>
				)}
			</CardContent>

			<ProviderDrawer
				open={isDrawerOpen}
				onOpenChange={setIsDrawerOpen}
				provider={editingProvider}
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
	onDelete,
	onSetDefault,
}: {
	provider: LlmProviderConfig;
	onEdit: (p: LlmProviderConfig) => void;
	onDelete: (id: string) => void;
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

	return (
		<div className="flex items-center justify-between p-4 border rounded-lg hover:bg-muted/30 transition-colors">
			<div className="flex-1 min-w-0">
				<div className="flex items-center gap-2">
					<span className="font-medium truncate">{provider.name}</span>
					<Badge variant="outline" className="shrink-0 text-xs">
						{PROVIDER_TYPE_LABELS[provider.provider_type] || provider.provider_type}
					</Badge>
					{provider.is_default && (
						<Badge variant="secondary" className="shrink-0 text-xs gap-1">
							<Star className="h-3 w-3 fill-current" />
							{t("settings.providers.default", "Default")}
						</Badge>
					)}
					{!provider.has_api_key && (
						<Badge variant="secondary" className="shrink-0 text-xs">
							{t("settings.providers.noKey", "No API Key")}
						</Badge>
					)}
				</div>
				<div className="text-sm text-muted-foreground mt-1 truncate">
					{provider.base_url}
					<span className="mx-1.5">·</span>
					{provider.model_id}
				</div>
			</div>
			<div className="flex items-center gap-1.5 ml-4">
				{testMutation.isSuccess && testMutation.data?.success && (
					<span className="text-xs text-green-600 mr-1">
						{testMutation.data.latency_ms}ms
					</span>
				)}
				{testMutation.isSuccess && testMutation.data?.success && (
					<CheckCircle className="h-4 w-4 text-green-500" />
				)}
				{testMutation.isSuccess && !testMutation.data?.success && (
					<XCircle className="h-4 w-4 text-red-500" />
				)}
				{!provider.is_default && (
					<Button
						variant="ghost"
						size="sm"
						className="h-8 w-8 p-0"
						onClick={() => onSetDefault(provider.id)}
						title={t("settings.providers.setDefault", "Set as default")}
					>
						<Star className="h-4 w-4" />
					</Button>
				)}
				<Button
					variant="ghost"
					size="sm"
					className="h-8 w-8 p-0"
					onClick={() => testMutation.mutate()}
					disabled={testMutation.isPending}
					title={t("settings.providers.testConnection", "Test connection")}
				>
					{testMutation.isPending ? (
						<Loader2 className="h-4 w-4 animate-spin" />
					) : (
						<Zap className="h-4 w-4" />
					)}
				</Button>
				<Button
					variant="ghost"
					size="sm"
					className="h-8 w-8 p-0"
					onClick={() => onEdit(provider)}
					title={t("common.edit", "Edit")}
				>
					<Pencil className="h-4 w-4" />
				</Button>
				<Button
					variant="ghost"
					size="sm"
					className="h-8 w-8 p-0 text-muted-foreground hover:text-red-500"
					onClick={() => onDelete(provider.id)}
					title={t("common.delete", "Delete")}
				>
					<Trash2 className="h-4 w-4" />
				</Button>
			</div>
		</div>
	);
}

function ProviderDrawer({
	open,
	onOpenChange,
	provider,
	onSuccess,
}: {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	provider: LlmProviderConfig | null;
	onSuccess: () => void;
}) {
	const { t } = useTranslation();
	const isEditing = !!provider;

	const [name, setName] = useState("");
	const [providerType, setProviderType] = useState<ProviderType>("openai_chat");
	const [baseUrl, setBaseUrl] = useState("");
	const [modelId, setModelId] = useState("");
	const [apiKey, setApiKey] = useState("");
	const [temperature, setTemperature] = useState("0.7");
	const [maxTokens, setMaxTokens] = useState("4096");
	const [testState, setTestState] = useState<"idle" | "loading" | "success" | "error">("idle");
	const resetTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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
				setApiKey("");
				setTemperature(String(provider.default_params.temperature));
				setMaxTokens(String(provider.default_params.max_tokens));
			} else {
				setName("");
				setProviderType("openai_chat");
				setBaseUrl(DEFAULT_BASE_URLS.openai_chat);
				setModelId("");
				setApiKey("");
				setTemperature("0.7");
				setMaxTokens("4096");
			}
			setTestState("idle");
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
		try {
			await llmApi.testProvider(provider.id);
			setTestState("success");
		} catch {
			setTestState("error");
		}
		// Reset to idle after 2 seconds
		resetTimerRef.current = setTimeout(() => setTestState("idle"), 2000);
	}, [provider, testState]);

	const handleSubmit = useCallback(() => {
		const temp = parseFloat(temperature) || 0.7;
		const tokens = parseInt(maxTokens) || 4096;

		if (isEditing && provider) {
			updateMutation.mutate({
				id: provider.id,
				name: name || undefined,
				provider_type: providerType,
				base_url: baseUrl || undefined,
				model_id: modelId || undefined,
				api_key: apiKey || undefined,
				default_params: { temperature: temp, max_tokens: tokens },
			});
		} else {
			createMutation.mutate({
				name,
				provider_type: providerType,
				base_url: baseUrl || DEFAULT_BASE_URLS[providerType],
				model_id: modelId,
				api_key: apiKey || undefined,
				default_params: { temperature: temp, max_tokens: tokens },
			});
		}
	}, [
		name,
		providerType,
		baseUrl,
		modelId,
		apiKey,
		temperature,
		maxTokens,
		isEditing,
		provider,
		createMutation,
		updateMutation,
	]);

	const isPending = createMutation.isPending || updateMutation.isPending;
	const canSubmit = name.trim() && modelId.trim();

	return (
		<Drawer open={open} onOpenChange={onOpenChange}>
			<DrawerContent className="flex h-full flex-col overflow-hidden">
				<DrawerHeader className="shrink-0 pb-2 text-left">
					<DrawerTitle className="text-left">
						{isEditing
							? t("settings.providers.editTitle", "Edit Provider")
							: t("settings.providers.addTitle", "Add Provider")}
					</DrawerTitle>
					<DrawerDescription className="text-left">
						{t(
							"settings.providers.dialogDescription",
							"Configure an LLM provider for intelligent test generation",
						)}
					</DrawerDescription>
				</DrawerHeader>
				<div className="flex min-h-0 flex-1 flex-col space-y-4 overflow-y-auto px-6 pb-4 pt-2">
					<div className="flex items-center gap-4">
						<Label htmlFor="name" className="w-20 shrink-0 text-right">
							{t("settings.providers.name", "Name")}
						</Label>
						<div className="flex-1">
							<Input
								id="name"
								value={name}
								onChange={(e) => setName(e.target.value)}
								placeholder={t(
									"settings.providers.namePlaceholder",
									"My OpenAI Provider",
								)}
							/>
						</div>
					</div>

					<div className="flex items-center gap-4">
						<Label className="w-20 shrink-0 text-right">
							{t("settings.providers.type", "API Type")}
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
											<span>{opt.label}</span>
										</SelectItem>
									))}
								</SelectContent>
							</Select>
						</div>
					</div>

					<div className="flex items-center gap-4">
						<Label htmlFor="baseUrl" className="w-20 shrink-0 text-right">
							{t("settings.providers.baseUrl", "Base URL")}
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
							{t("settings.providers.apiKey", "API Key")}
						</Label>
						<div className="flex-1">
							<Input
								id="apiKey"
								type="password"
								value={apiKey}
								onChange={(e) => setApiKey(e.target.value)}
								placeholder={
									isEditing
										? t(
												"settings.providers.apiKeyEditPlaceholder",
												"Leave empty to keep current key",
											)
										: "sk-..."
								}
							/>
						</div>
					</div>

					<div className="flex items-center gap-4">
						<Label htmlFor="modelId" className="w-20 shrink-0 text-right">
							{t("settings.providers.model", "Model")}
						</Label>
						<div className="flex-1">
							<Input
								id="modelId"
								value={modelId}
								onChange={(e) => setModelId(e.target.value)}
								placeholder={
									providerType === "anthropic"
										? "claude-sonnet-4-20250514"
										: "gpt-4o"
								}
							/>
						</div>
					</div>

					<div className="flex items-center gap-4">
						<Label className="w-20 shrink-0 text-right">
							{t("settings.providers.temperature", "Temperature")}
						</Label>
						<div className="flex-1">
							<Input
								type="number"
								min="0"
								max="2"
								step="0.1"
								value={temperature}
								onChange={(e) => setTemperature(e.target.value)}
							/>
						</div>
					</div>

					<div className="flex items-center gap-4">
						<Label className="w-20 shrink-0 text-right">
							{t("settings.providers.maxTokens", "Max Tokens")}
						</Label>
						<div className="flex-1">
							<Input
								type="number"
								min="1"
								max="128000"
								value={maxTokens}
								onChange={(e) => setMaxTokens(e.target.value)}
							/>
						</div>
					</div>

					</div>
					<DrawerFooter className="mt-auto shrink-0 border-t px-6 py-4">
					<div className="flex w-full items-center justify-between gap-3">
						<DrawerClose asChild>
							<Button variant="outline">{t("common.cancel", "Cancel")}</Button>
						</DrawerClose>
						<div className="flex items-center gap-3">
							{isEditing && (
								<Button
									variant="outline"
									size="sm"
									onClick={handleTest}
									disabled={testState === "loading"}
									className={
										testState === "success"
											? "border-green-500 text-green-600"
											: testState === "error"
												? "border-red-500 text-red-600"
												: ""
									}
								>
									{testState === "loading" ? (
										<Loader2 className="mr-2 h-4 w-4 animate-spin" />
									) : testState === "success" ? (
										<CheckCircle className="mr-2 h-4 w-4" />
									) : testState === "error" ? (
										<XCircle className="mr-2 h-4 w-4" />
									) : (
										<Zap className="mr-2 h-4 w-4" />
									)}
									{testState === "loading"
										? t("settings.providers.testing", "Testing…")
										: testState === "success"
											? t("settings.providers.connected", "Connected")
											: testState === "error"
												? t("settings.providers.failed", "Failed")
												: t("settings.providers.testConnection", "Test")}
								</Button>
							)}
							<Button onClick={handleSubmit} disabled={isPending || !canSubmit}>
								{isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
								{isEditing
									? t("common.save", "Save")
									: t("common.create", "Create")}
							</Button>
						</div>
					</div>
				</DrawerFooter>
			</DrawerContent>
		</Drawer>
	);
}
