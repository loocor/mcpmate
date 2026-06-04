import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	AlertTriangle,
	BookOpenText,
	Bug,
	Check,
	Edit3,
	Eye,
	FileText,
	ListChecks,
	Play,
	Plus,
	RefreshCw,
	Square,
	Trash2,
} from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useState } from "react";
import ReactMarkdown from "react-markdown";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import remarkGfm from "remark-gfm";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { useUrlTab } from "../../lib/hooks/use-url-state";
import { CachedAvatar } from "../../components/cached-avatar";
import { CardListScrollBody } from "../../components/card-list-scroll-body";
import { AuditLogsPanel } from "../../components/audit-logs-panel";
import CapabilityList from "../../components/capability-list";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "../../components/capsule-stripe-list";
import { ProfileFormDrawer } from "../../components/profile-form-drawer";
import { DETAIL_TAB_CONTENT_CLASS } from "../../components/detail-tab-content-class";
import { ProfileTokenUsageChart } from "./components/profile-token-usage-chart";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from "../../components/ui/alert-dialog";
import { Avatar, AvatarFallback } from "../../components/ui/avatar";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { ButtonGroup } from "../../components/ui/button-group";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import { Switch } from "../../components/ui/switch";
import {
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "../../components/ui/tabs";
import { Textarea } from "../../components/ui/textarea";
import { auditApi, configSuitsApi, serversApi, useProfileTokenChartSource } from "../../lib/api";
import { DEFAULT_ANCHOR_ROLE } from "../../lib/default-profile";
import { notifyError, notifySuccess } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import type {
	ConfigSuitGuidance,
	ConfigSuitGuidanceCapabilityRef,
	ConfigSuitPrompt,
	ConfigSuitResource,
	ConfigSuitResourceTemplate,
	ConfigSuitServer,
	ConfigSuitTool,
} from "../../lib/types";

const toTitleCase = (value?: string | null) =>
	(value ?? "")
		.trim()
		.split(/[\s_-]+/)
		.filter(Boolean)
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
		.join(" ") ||
	value ||
	"";

const formatProfileTypeLabel = (value?: string | null) =>
	value
		?.split(/[\s_]+/)
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1))
		.join(" ") ?? "";

const PROFILE_DETAIL_TABS = [
	"overview",
	"guidance",
	"servers",
	"tools",
	"prompts",
	"resources",
	"templates",
];

type ProfileGuidanceFormState = {
	id?: string;
	slug: string;
	title: string;
	summary: string;
	scenario: string;
	activation: string;
	capabilityRefs: ConfigSuitGuidanceCapabilityRef[];
	validationNotes: string;
	avoid: string;
	contentMarkdown: string;
	sourceUri: string;
	enabled: boolean;
};

type GuidanceCapabilityOption = ConfigSuitGuidanceCapabilityRef & {
	key: string;
	enabled: boolean;
	label: string;
	detail: string;
};

const emptyGuidanceForm: ProfileGuidanceFormState = {
	slug: "",
	title: "",
	summary: "",
	scenario: "",
	activation: "",
	capabilityRefs: [],
	validationNotes: "",
	avoid: "",
	contentMarkdown: "",
	sourceUri: "",
	enabled: true,
};

const guidanceToForm = (guidance: ConfigSuitGuidance): ProfileGuidanceFormState => ({
	id: guidance.id,
	slug: guidance.slug,
	title: guidance.title,
	summary: guidance.summary ?? "",
	scenario: guidance.scenario ?? "",
	activation: guidance.activation ?? "",
	capabilityRefs: guidance.capability_refs ?? [],
	validationNotes: guidance.validation_notes ?? "",
	avoid: guidance.avoid ?? "",
	contentMarkdown: guidance.content_markdown,
	sourceUri: guidance.source_uri ?? "",
	enabled: guidance.enabled,
});

const guidanceCapabilityKey = (capability: Pick<ConfigSuitGuidanceCapabilityRef, "kind" | "id">) =>
	`${capability.kind}:${capability.id}`;

const emptyStringToNull = (value: string) => {
	const trimmed = value.trim();
	return trimmed.length > 0 ? trimmed : null;
};

const guidanceUpsertPayload = (
	profileId: string,
	form: ProfileGuidanceFormState,
) => ({
	id: form.id ?? null,
	profile_id: profileId,
	slug: form.slug.trim(),
	title: form.title.trim(),
	summary: emptyStringToNull(form.summary),
	scenario: emptyStringToNull(form.scenario),
	activation: emptyStringToNull(form.activation),
	capability_refs: form.capabilityRefs,
	validation_notes: emptyStringToNull(form.validationNotes),
	avoid: emptyStringToNull(form.avoid),
	content_markdown: form.contentMarkdown.trim(),
	source_uri: emptyStringToNull(form.sourceUri),
	enabled: form.enabled,
});

const fillGuidanceTemplateField = (
	currentValue: string,
	templateValue: string,
) => currentValue || templateValue;

const buildGuidanceMarkdownPreview = (form: ProfileGuidanceFormState) => {
	const lines: string[] = [];
	const title = form.title.trim();
	if (title) {
		lines.push(`# ${title}`, "");
	}
	const summary = form.summary.trim();
	if (summary) {
		lines.push(summary, "");
	}
	const sections: Array<[string, string]> = [
		["Scenario", form.scenario],
		["Activation", form.activation],
	];
	for (const [sectionTitle, value] of sections) {
		const trimmed = value.trim();
		if (trimmed) {
			lines.push(`## ${sectionTitle}`, "", trimmed, "");
		}
	}
	if (form.capabilityRefs.length > 0) {
		lines.push("## Capabilities", "");
		for (const capability of form.capabilityRefs) {
			const name = capability.name ? ` (${capability.name})` : "";
			const server = capability.server_name ? ` via ${capability.server_name}` : "";
			lines.push(`- ${capability.kind}: ${capability.id}${name}${server}`);
		}
		lines.push("");
	}
	const validationNotes = form.validationNotes.trim();
	if (validationNotes) {
		lines.push("## Validation", "", validationNotes, "");
	}
	const avoid = form.avoid.trim();
	if (avoid) {
		lines.push("## Avoid", "", avoid, "");
	}
	const sourceUri = form.sourceUri.trim();
	if (sourceUri) {
		lines.push(`Source: ${sourceUri}`, "");
	}
	const body = form.contentMarkdown.trim();
	if (body) {
		lines.push(body);
	}
	return lines.join("\n").trim();
};

const hasGuidanceFormChanged = (
	current: ProfileGuidanceFormState,
	original: ProfileGuidanceFormState,
) => JSON.stringify(current) !== JSON.stringify(original);

export function ProfileDetailPage() {
	const { t } = useTranslation();
	usePageTranslations("profiles");
	const { profileId } = useParams<{ profileId: string }>();
	const [searchParams] = useSearchParams();
	const queryClient = useQueryClient();
	const navigate = useNavigate();

	/** Refetch capability JSON payloads when server membership or live MCP definitions may have changed. */
	const invalidateProfileCapabilityLedger = useCallback(() => {
		if (profileId) {
			void queryClient.invalidateQueries({ queryKey: ["capabilityTokenLedger", profileId] });
			void queryClient.invalidateQueries({ queryKey: ["profileChartTokenEstimate", profileId] });
		}
	}, [profileId, queryClient]);

	const showProfileLiveLogs = useAppStore(
		(state) => state.dashboardSettings.showProfileLiveLogs,
	);
	const profileTokenEstimateMethod = useAppStore(
		(state) => state.dashboardSettings.profileTokenEstimateMethod,
	);
	const { activeTab, setActiveTab } = useUrlTab({
		paramName: "tab",
		defaultTab: "overview",
		validTabs: PROFILE_DETAIL_TABS,
	});

	const mode = searchParams.get("mode");
	const guidanceSlugId = useId();
	const guidanceTitleId = useId();
	const guidanceSummaryId = useId();
	const guidanceScenarioId = useId();
	const guidanceActivationId = useId();
	const guidanceValidationId = useId();
	const guidanceAvoidId = useId();
	const guidanceSourceUriId = useId();
	const guidanceInstructionsId = useId();
	const guidanceEnabledId = useId();

	// Developer toggles
	const enableServerDebug = useAppStore(
		(state) => state.dashboardSettings.enableServerDebug,
	);
	const openDebugInNewWindow = useAppStore(
		(state) => state.dashboardSettings.openDebugInNewWindow,
	);

	const openDebug = (
		targetServerId: string,
		channel: "proxy" | "native" = "proxy",
	) => {
		const url = `/servers/${encodeURIComponent(targetServerId)}?view=debug&channel=${channel}`;
		if (openDebugInNewWindow) {
			if (typeof window !== "undefined") {
				window.open(url, "_blank", "noopener,noreferrer");
			}
			return;
		}
		navigate(url);
	};
	const openBrowse = (targetServerId: string) => {
		const url = `/servers/${encodeURIComponent(targetServerId)}?view=browse`;
		if (openDebugInNewWindow) {
			if (typeof window !== "undefined") {
				window.open(url, "_blank", "noopener,noreferrer");
			}
			return;
		}
		navigate(url);
	};
	const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);
	const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
	const [isGuidanceDeleteDialogOpen, setIsGuidanceDeleteDialogOpen] = useState(false);
	// Filters: servers
	const [serverQuery, setServerQuery] = useState("");
	const [serverStatus, setServerStatus] = useState<
		"all" | "enabled" | "disabled"
	>("all");
	// Filters: tools
	const [toolQuery, setToolQuery] = useState("");
	const [toolStatus, setToolStatus] = useState<"all" | "enabled" | "disabled">(
		"all",
	);
	const [toolServer, setToolServer] = useState<string>("all");
	// Filters: resources
	const [resourceQuery, setResourceQuery] = useState("");
	const [resourceStatus, setResourceStatus] = useState<
		"all" | "enabled" | "disabled"
	>("all");
	const [resourceServer, setResourceServer] = useState<string>("all");
	// Filters: prompts
	const [promptQuery, setPromptQuery] = useState("");
	const [promptStatus, setPromptStatus] = useState<
		"all" | "enabled" | "disabled"
	>("all");
	const [promptServer, setPromptServer] = useState<string>("all");
	// Bulk selection states for lists
	const [selectedServerIds, setSelectedServerIds] = useState<string[]>([]);
	const [selectedToolIds, setSelectedToolIds] = useState<string[]>([]);
	const [selectedResourceIds, setSelectedResourceIds] = useState<string[]>([]);
	const [selectedPromptIds, setSelectedPromptIds] = useState<string[]>([]);
	const [selectedTemplateIds, setSelectedTemplateIds] = useState<string[]>([]);
	const [selectedGuidanceId, setSelectedGuidanceId] = useState<string | null>(null);
	const [guidanceForm, setGuidanceForm] =
		useState<ProfileGuidanceFormState>(emptyGuidanceForm);
	const [guidancePreviewMode, setGuidancePreviewMode] = useState<"edit" | "preview">("edit");
	const [logFilter, setLogFilter] = useState("");
	const [logPageSize, setLogPageSize] = useState<number>(10);
	const [logPageCursors, setLogPageCursors] = useState<string[]>([]);
	const [logCurrentPageIndex, setLogCurrentPageIndex] = useState(0);
	const [isLogPaginationActionLoading, setIsLogPaginationActionLoading] =
		useState(false);
	const logCurrentCursor = logPageCursors[logCurrentPageIndex];

	const profileLogsQuery = useQuery({
		queryKey: [
			"profile-audit-logs",
			profileId,
			logCurrentCursor,
			logPageSize,
			showProfileLiveLogs,
		],
		queryFn: () =>
			auditApi.list({
				limit: logPageSize,
				cursor: logCurrentCursor,
				profile_id: profileId,
			}),
		enabled: Boolean(profileId && showProfileLiveLogs),
		refetchOnWindowFocus: false,
		retry: false,
	});

	useEffect(() => {
		setLogPageCursors([]);
		setLogCurrentPageIndex(0);
	}, [profileId, logPageSize]);

	const filteredProfileLogs = useMemo(() => {
		const logs = profileLogsQuery.data?.events ?? [];
		const term = logFilter.trim().toLowerCase();
		if (!term) return logs;
		return logs.filter((event) => {
			const haystacks = [
				event.action,
				event.category,
				event.status,
				event.target,
				event.route,
				event.error_message,
				event.detail,
				event.request_id,
				event.mcp_method,
			]
				.filter(Boolean)
				.map((value) => String(value).toLowerCase());
			return haystacks.some((value) => value.includes(term));
		});
	}, [profileLogsQuery.data?.events, logFilter]);

	const handleProfileLogsNextPage = () => {
		if (!profileLogsQuery.data?.next_cursor) return;
		const nextCursor = profileLogsQuery.data.next_cursor;
		setLogPageCursors((prev) => {
			const next = [...prev];
			next[logCurrentPageIndex + 1] = nextCursor;
			return next;
		});
		setLogCurrentPageIndex((prev) => prev + 1);
	};

	const handleProfileLogsPrevPage = () => {
		if (logCurrentPageIndex > 0) {
			setLogCurrentPageIndex((prev) => prev - 1);
		}
	};

	const handleProfileLogsFirstPage = () => {
		setLogCurrentPageIndex(0);
	};

	const handleProfileLogsLastPage = async () => {
		if (!profileLogsQuery.data?.next_cursor || !profileId) return;
		setIsLogPaginationActionLoading(true);
		try {
			let nextCursor: string | undefined = profileLogsQuery.data.next_cursor;
			let targetPageIndex = logCurrentPageIndex;
			const nextPageCursors = [...logPageCursors];
			while (nextCursor) {
				targetPageIndex += 1;
				nextPageCursors[targetPageIndex] = nextCursor;
				const page = await auditApi.list({
					limit: logPageSize,
					cursor: nextCursor,
					profile_id: profileId,
				});
				nextCursor = page.next_cursor ?? undefined;
			}
			setLogPageCursors(nextPageCursors);
			setLogCurrentPageIndex(targetPageIndex);
		} finally {
			setIsLogPaginationActionLoading(false);
		}
	};

	// Bulk mutations using server-side batch manage to improve reliability
	const bulkToolsM = useMutation({
		mutationFn: ({ enable }: { enable: boolean }) =>
			configSuitsApi.bulkTools(
				profileId!,
				selectedToolIds,
				enable ? "enable" : "disable",
			),
		onSuccess: () => {
			setSelectedToolIds([]);
			refetchTools();
			notifySuccess(
				t("profiles:detail.messages.toolsUpdated", { defaultValue: "Tools updated" }),
				t("profiles:detail.messages.bulkOperationCompleted", { defaultValue: "Bulk operation completed" })
			);
		},
		onError: (e) => notifyError(
			t("profiles:detail.messages.toolsUpdateFailed", { defaultValue: "Tools update failed" }),
			String(e)
		),
	});
	const bulkResourcesM = useMutation({
		mutationFn: ({ enable }: { enable: boolean }) =>
			configSuitsApi.bulkResources(
				profileId!,
				selectedResourceIds,
				enable ? "enable" : "disable",
			),
		onSuccess: () => {
			setSelectedResourceIds([]);
			refetchResources();
			notifySuccess(
				t("profiles:detail.messages.resourcesUpdated", { defaultValue: "Resources updated" }),
				t("profiles:detail.messages.bulkOperationCompleted", { defaultValue: "Bulk operation completed" })
			);
		},
		onError: (e) => notifyError(
			t("profiles:detail.messages.resourcesUpdateFailed", { defaultValue: "Resources update failed" }),
			String(e)
		),
	});
	const bulkPromptsM = useMutation({
		mutationFn: ({ enable }: { enable: boolean }) =>
			configSuitsApi.bulkPrompts(
				profileId!,
				selectedPromptIds,
				enable ? "enable" : "disable",
			),
		onSuccess: () => {
			setSelectedPromptIds([]);
			refetchPrompts();
			notifySuccess(
				t("profiles:detail.messages.promptsUpdated", { defaultValue: "Prompts updated" }),
				t("profiles:detail.messages.bulkOperationCompleted", { defaultValue: "Bulk operation completed" })
			);
		},
		onError: (e) => notifyError(
			t("profiles:detail.messages.promptsUpdateFailed", { defaultValue: "Prompts update failed" }),
			String(e)
		),
	});

	const bulkTemplatesM = useMutation({
		mutationFn: ({ enable }: { enable: boolean }) =>
			configSuitsApi.bulkResourceTemplates(
				profileId!,
				selectedTemplateIds,
				enable ? "enable" : "disable",
			),
		onSuccess: () => {
			setSelectedTemplateIds([]);
			refetchTemplates();
			notifySuccess(
				t("profiles:detail.messages.templatesUpdated", { defaultValue: "Templates updated" }),
				t("profiles:detail.messages.bulkOperationCompleted", { defaultValue: "Bulk operation completed" })
			);
		},
		onError: (e) => notifyError(
			t("profiles:detail.messages.templatesUpdateFailed", { defaultValue: "Templates update failed" }),
			String(e)
		),
	});

	const bulkServersM = useMutation({
		mutationFn: ({ enable }: { enable: boolean }) =>
			configSuitsApi.bulkServers(
				profileId!,
				selectedServerIds,
				enable ? "enable" : "disable",
			),
		onSuccess: () => {
			setSelectedServerIds([]);
			invalidateProfileCapabilityLedger();
			refetchServers();
			notifySuccess(
				t("profiles:detail.messages.serversUpdated", { defaultValue: "Servers updated" }),
				t("profiles:detail.messages.bulkOperationCompleted", { defaultValue: "Bulk operation completed" })
			);
		},
		onError: (e) => notifyError(
			t("profiles:detail.messages.serversUpdateFailed", { defaultValue: "Servers update failed" }),
			String(e)
		),
	});

	// Force cleanup when drawer closes to prevent overlay issues
	useEffect(() => {
		if (!isEditDialogOpen) {
			// 使用 requestAnimationFrame 确保在正确时机清理
			requestAnimationFrame(() => {
				setTimeout(() => {
					// 清理所有可能的遮罩层和覆盖元素
					const overlays = document.querySelectorAll(
						"[data-radix-popper-content-wrapper], [data-radix-dialog-overlay], [data-vaul-overlay], [data-vaul-drawer-wrapper], .fixed.inset-0, [data-vaul-drawer]",
					);
					overlays.forEach((overlay) => {
						const element = overlay as HTMLElement;
						if (
							element.getAttribute("data-state") === "closed" ||
							!element.closest('[data-state="open"]') ||
							element.style.pointerEvents === "none"
						) {
							element.remove();
						}
					});

					// 确保 body 样式被正确重置
					document.body.style.removeProperty("pointer-events");
					document.body.style.removeProperty("overflow");
					document.body.removeAttribute("data-scroll-locked");
					document.body.removeAttribute("aria-hidden");
					document.body.removeAttribute("data-vaul-drawer-wrapper");
				}, 50);
			});
		}
	}, [isEditDialogOpen]);

	// Do not early-return before hooks; guard queries with `enabled`

	// Fetch config suit details
	const {
		data: suit,
		isLoading: isLoadingSuit,
		refetch: refetchSuit,
		isRefetching: isRefetchingSuit,
	} = useQuery({
		queryKey: ["configSuit", profileId],
		queryFn: async () => {
			if (!profileId) return undefined;
			console.log("Fetching profile details for:", profileId);
			const result = await configSuitsApi.getSuit(profileId);
			console.log("Profile details response:", result);
			return result;
		},
		enabled: !!profileId,
		retry: 1,
	});

	// Fetch servers in suit
	const {
		data: serversResponse,
		isLoading: isLoadingServers,
		refetch: refetchServers,
	} = useQuery({
		queryKey: ["configSuitServers", profileId],
		queryFn: async () => {
			if (!profileId) return undefined;
			console.log("Fetching servers for profile:", profileId);
			const result = await configSuitsApi.getServers(profileId);
			console.log("Profile servers response:", result);
			return result;
		},
		enabled: !!profileId,
		retry: 1,
	});
	// Fetch tools in suit
	const {
		data: toolsResponse,
		isLoading: isLoadingTools,
		refetch: refetchTools,
	} = useQuery({
		queryKey: ["configSuitTools", profileId],
		queryFn: () =>
			profileId
				? configSuitsApi.getTools(profileId)
				: Promise.resolve(undefined),
		enabled: !!profileId,
		retry: 1,
	});

	// Fetch resources in suit
	const {
		data: resourcesResponse,
		isLoading: isLoadingResources,
		refetch: refetchResources,
	} = useQuery({
		queryKey: ["configSuitResources", profileId],
		queryFn: () =>
			profileId
				? configSuitsApi.getResources(profileId)
				: Promise.resolve(undefined),
		enabled: !!profileId,
		retry: 1,
	});

	// Fetch prompts in suit
	const {
		data: promptsResponse,
		isLoading: isLoadingPrompts,
		refetch: refetchPrompts,
	} = useQuery({
		queryKey: ["configSuitPrompts", profileId],
		queryFn: () =>
			profileId
				? configSuitsApi.getPrompts(profileId)
				: Promise.resolve(undefined),
		enabled: !!profileId,
		retry: 1,
	});

	// Fetch resource templates in suit
	const {
		data: templatesResponse,
		isLoading: isLoadingTemplates,
		refetch: refetchTemplates,
	} = useQuery({
		queryKey: ["configSuitResourceTemplates", profileId],
		queryFn: () =>
			profileId
				? configSuitsApi.getResourceTemplates(profileId)
				: Promise.resolve(undefined),
		enabled: !!profileId,
		retry: 1,
	});

	const {
		data: guidanceResponse,
		isLoading: isLoadingGuidance,
		refetch: refetchGuidance,
	} = useQuery({
		queryKey: ["configSuitGuidance", profileId],
		queryFn: () =>
			profileId
				? configSuitsApi.getGuidance(profileId)
				: Promise.resolve(undefined),
		enabled: !!profileId,
		retry: 1,
	});

	const guidanceSaveMutation = useMutation({
		mutationFn: () => {
			if (!profileId) {
				return Promise.reject(
					new Error(
						t("profiles:detail.errors.noSuitId", { defaultValue: "No profile ID" }),
					),
				);
			}
			return configSuitsApi.upsertGuidance(guidanceUpsertPayload(profileId, guidanceForm));
		},
		onSuccess: (guidance) => {
			setSelectedGuidanceId(guidance.id);
			setGuidanceForm(guidanceToForm(guidance));
			void queryClient.invalidateQueries({
				queryKey: ["configSuitGuidance", profileId],
			});
			notifySuccess(
				t("profiles:detail.guidance.messages.saved", {
					defaultValue: "Guidance saved",
				}),
				t("profiles:detail.guidance.messages.savedDescription", {
					defaultValue: "Profile guidance has been updated.",
				}),
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.guidance.messages.saveFailed", {
					defaultValue: "Guidance save failed",
				}),
				error instanceof Error ? error.message : String(error),
			);
		},
	});

	const guidanceDeleteMutation = useMutation({
		mutationFn: () => {
			if (!profileId) {
				return Promise.reject(
					new Error(
						t("profiles:detail.errors.noSuitId", { defaultValue: "No profile ID" }),
					),
				);
			}
			return configSuitsApi.deleteGuidance({
				profile_id: profileId,
				slug: guidanceForm.slug.trim(),
			});
		},
		onSuccess: () => {
			setIsGuidanceDeleteDialogOpen(false);
			setSelectedGuidanceId(null);
			setGuidanceForm(emptyGuidanceForm);
			void queryClient.invalidateQueries({
				queryKey: ["configSuitGuidance", profileId],
			});
			notifySuccess(
				t("profiles:detail.guidance.messages.deleted", {
					defaultValue: "Guidance deleted",
				}),
				t("profiles:detail.guidance.messages.deletedDescription", {
					defaultValue: "Profile guidance has been removed.",
				}),
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.guidance.messages.deleteFailed", {
					defaultValue: "Guidance delete failed",
				}),
				error instanceof Error ? error.message : String(error),
			);
		},
	});

	// Activation/deactivation mutations
	const activateSuitMutation = useMutation({
		mutationFn: () => configSuitsApi.activateSuit(profileId!),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["configSuit", profileId] });
			queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			notifySuccess(
				t("profiles:detail.messages.profileActivated", { defaultValue: "Profile activated" }),
				t("profiles:detail.messages.profileActivatedDescription", { defaultValue: "Profile has been successfully activated" }),
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.activationFailed", { defaultValue: "Activation failed" }),
				`${t("profiles:detail.messages.activationFailedDescription", { defaultValue: "Failed to activate profile" })}: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	const deactivateSuitMutation = useMutation({
		mutationFn: () => configSuitsApi.deactivateSuit(profileId!),
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: ["configSuit", profileId] });
			queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			notifySuccess(
				t("profiles:detail.messages.profileDeactivated", { defaultValue: "Profile deactivated" }),
				t("profiles:detail.messages.profileDeactivatedDescription", { defaultValue: "Profile has been successfully deactivated" }),
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.deactivationFailed", { defaultValue: "Deactivation failed" }),
				`${t("profiles:detail.messages.deactivationFailedDescription", { defaultValue: "Failed to deactivate profile" })}: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	// Delete profile mutation
	const deleteSuitMutation = useMutation({
		mutationFn: () => {
			if (!profileId) return Promise.reject(t("profiles:detail.errors.noSuitId", { defaultValue: "No suit ID" }));
			return configSuitsApi.deleteSuit(profileId);
		},
		onSuccess: () => {
			// Invalidate queries to refresh the profiles list
			queryClient.invalidateQueries({ queryKey: ["configSuits"] });
			notifySuccess(
				t("profiles:detail.messages.profileDeleted", { defaultValue: "Profile deleted" }),
				t("profiles:detail.messages.profileDeletedDescription", { defaultValue: "Profile has been successfully deleted" })
			);
			navigate("/profiles");
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.deleteFailed", { defaultValue: "Delete failed" }),
				`Failed to delete profile: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	// Server toggle mutations
	const serverToggleMutation = useMutation({
		mutationFn: ({
			serverId,
			enable,
		}: {
			serverId: string;
			enable: boolean;
		}) => {
			return enable
				? configSuitsApi.enableServer(profileId!, serverId)
				: configSuitsApi.disableServer(profileId!, serverId);
		},
		onSuccess: () => {
			invalidateProfileCapabilityLedger();
			// Refetch all capability data to update counts in tabs
			refetchServers();
			refetchTools();
			refetchResources();
			refetchPrompts();
			refetchTemplates();

			// Invalidate profile statistics cache for config page
			queryClient.invalidateQueries({
				queryKey: ["configSuitStats", profileId],
			});

			notifySuccess(
				t("profiles:detail.messages.serverUpdated", { defaultValue: "Server updated" }),
				"Server status has been updated"
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.serverUpdateFailed", { defaultValue: "Server update failed" }),
				`Failed to update server: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	// Tool toggle mutations
	const toolToggleMutation = useMutation({
		mutationFn: ({ toolId, enable }: { toolId: string; enable: boolean }) => {
			return enable
				? configSuitsApi.enableTool(profileId!, toolId)
				: configSuitsApi.disableTool(profileId!, toolId);
		},
		onSuccess: () => {
			refetchTools();
			refetchTemplates();
			notifySuccess(
				t("profiles:detail.messages.toolUpdated", { defaultValue: "Tool updated" }),
				"Tool status has been updated"
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.toolUpdateFailed", { defaultValue: "Tool update failed" }),
				`Failed to update tool: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	// Resource toggle mutations
	const resourceToggleMutation = useMutation({
		mutationFn: ({
			resourceId,
			enable,
		}: {
			resourceId: string;
			enable: boolean;
		}) => {
			return enable
				? configSuitsApi.enableResource(profileId!, resourceId)
				: configSuitsApi.disableResource(profileId!, resourceId);
		},
		onSuccess: () => {
			refetchResources();
			refetchTemplates();
			notifySuccess(
				t("profiles:detail.messages.resourceUpdated", { defaultValue: "Resource updated" }),
				"Resource status has been updated"
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.resourceUpdateFailed", { defaultValue: "Resource update failed" }),
				`Failed to update resource: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	// Prompt toggle mutations
	const promptToggleMutation = useMutation({
		mutationFn: ({
			promptId,
			enable,
		}: {
			promptId: string;
			enable: boolean;
		}) => {
			return enable
				? configSuitsApi.enablePrompt(profileId!, promptId)
				: configSuitsApi.disablePrompt(profileId!, promptId);
		},
		onSuccess: () => {
			refetchPrompts();
			refetchTemplates();
			notifySuccess(
				t("profiles:detail.messages.promptUpdated", { defaultValue: "Prompt updated" }),
				"Prompt status has been updated"
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.promptUpdateFailed", { defaultValue: "Prompt update failed" }),
				`Failed to update prompt: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	const suitRole = suit?.role ?? "user";
	const isDefaultAnchor = suitRole === DEFAULT_ANCHOR_ROLE;
	const isHostApp = suit?.suit_type === "host_app";
	const isCustomMode = mode === "custom";

	const handleSuitToggle = () => {
		if (isDefaultAnchor) {
			return;
		}
		if (suit?.is_active) {
			deactivateSuitMutation.mutate();
		} else {
			activateSuitMutation.mutate();
		}
	};

	const handleRefreshAll = () => {
		refetchSuit();
		refetchServers();
		refetchTools();
		refetchResources();
		refetchPrompts();
		refetchTemplates();
		refetchGuidance();
		invalidateProfileCapabilityLedger();
	};
	const overviewActionButtonClass =
		"gap-2 rounded-none first:rounded-l-md last:rounded-r-md";

	const handleEditDrawerClose = (open: boolean) => {
		setIsEditDialogOpen(open);
	};

	const servers = useMemo<ConfigSuitServer[]>(
		() => serversResponse?.servers ?? [],
		[serversResponse?.servers],
	);
	const tools = useMemo<ConfigSuitTool[]>(
		() => toolsResponse?.tools ?? [],
		[toolsResponse?.tools],
	);
	const resources = useMemo<ConfigSuitResource[]>(
		() => resourcesResponse?.resources ?? [],
		[resourcesResponse?.resources],
	);
	const prompts = useMemo<ConfigSuitPrompt[]>(
		() => promptsResponse?.prompts ?? [],
		[promptsResponse?.prompts],
	);
	const templates = useMemo<ConfigSuitResourceTemplate[]>(
		() => templatesResponse?.templates ?? [],
		[templatesResponse?.templates],
	);
	const guidanceRecords = useMemo<ConfigSuitGuidance[]>(
		() => guidanceResponse?.guidance ?? [],
		[guidanceResponse?.guidance],
	);

	const enabledServers = servers.filter((s: ConfigSuitServer) => s.enabled);
	const enabledTools = tools.filter((t: ConfigSuitTool) => t.enabled);
	const enabledResources = resources.filter(
		(r: ConfigSuitResource) => r.enabled,
	);
	const enabledPrompts = prompts.filter((p: ConfigSuitPrompt) => p.enabled);
	const enabledTemplates = templates.filter((template) => template.enabled);
	const enabledGuidanceRecords = guidanceRecords.filter((guidance) => guidance.enabled);

	useEffect(() => {
		if (guidanceRecords.length === 0) {
			return;
		}
		if (
			selectedGuidanceId &&
			guidanceRecords.some((guidance) => guidance.id === selectedGuidanceId)
		) {
			return;
		}
		const firstGuidance = guidanceRecords[0];
		setSelectedGuidanceId(firstGuidance.id);
		setGuidanceForm(guidanceToForm(firstGuidance));
	}, [guidanceRecords, selectedGuidanceId]);

	const updateGuidanceForm = <K extends keyof ProfileGuidanceFormState>(
		key: K,
		value: ProfileGuidanceFormState[K],
	) => {
		setGuidanceForm((current) => ({ ...current, [key]: value }));
	};

	const handleSelectGuidance = (guidance: ConfigSuitGuidance) => {
		setSelectedGuidanceId(guidance.id);
		setGuidanceForm(guidanceToForm(guidance));
	};

	const handleNewGuidance = () => {
		setSelectedGuidanceId(null);
		setGuidanceForm(emptyGuidanceForm);
	};

	const guidanceResourceUri =
		profileId && guidanceForm.slug.trim().length > 0
			? `skill://profiles/${profileId}/${guidanceForm.slug.trim()}/SKILL.md`
			: "";
	const selectedGuidanceRecord = guidanceRecords.find(
		(guidance) => guidance.id === selectedGuidanceId,
	);
	const originalGuidanceForm = selectedGuidanceRecord
		? guidanceToForm(selectedGuidanceRecord)
		: emptyGuidanceForm;
	const guidanceHasChanges = hasGuidanceFormChanged(guidanceForm, originalGuidanceForm);
	const guidancePreviewMarkdown = buildGuidanceMarkdownPreview(guidanceForm);

	const guidanceCapabilityOptions = useMemo(() => {
		const options: GuidanceCapabilityOption[] = [
			...servers.map((server) => ({
				kind: "server",
				id: server.id,
				name: server.name,
				server_name: server.name,
				enabled: server.enabled,
				label: server.name,
				detail: server.id,
				key: `server:${server.id}`,
			})),
			...tools.map((tool) => ({
				kind: "tool",
				id: tool.id,
				name: tool.tool_name ?? tool.unique_name ?? tool.id,
				server_name: tool.server_name,
				enabled: tool.enabled,
				label: tool.tool_name ?? tool.unique_name ?? tool.id,
				detail: tool.server_name,
				key: `tool:${tool.id}`,
			})),
			...prompts.map((prompt) => ({
				kind: "prompt",
				id: prompt.id,
				name: prompt.prompt_name,
				server_name: prompt.server_name,
				enabled: prompt.enabled,
				label: prompt.prompt_name,
				detail: prompt.server_name,
				key: `prompt:${prompt.id}`,
			})),
			...resources.map((resource) => ({
				kind: "resource",
				id: resource.id,
				name: resource.resource_uri,
				server_name: resource.server_name,
				enabled: resource.enabled,
				label: resource.resource_uri,
				detail: resource.server_name,
				key: `resource:${resource.id}`,
			})),
			...templates.map((template) => ({
				kind: "template",
				id: template.id,
				name: template.uri_template,
				server_name: template.server_name,
				enabled: template.enabled,
				label: template.uri_template,
				detail: template.server_name,
				key: `template:${template.id}`,
			})),
		];
		return options.sort((left, right) =>
			left.kind.localeCompare(right.kind) || left.label.localeCompare(right.label),
		);
	}, [servers, tools, prompts, resources, templates]);
	const guidanceCapabilityOptionByKey = useMemo(
		() => new Map(guidanceCapabilityOptions.map((option) => [option.key, option])),
		[guidanceCapabilityOptions],
	);
	const selectedGuidanceCapabilityKeys = useMemo(
		() => new Set(guidanceForm.capabilityRefs.map(guidanceCapabilityKey)),
		[guidanceForm.capabilityRefs],
	);
	const missingGuidanceCapabilities = useMemo(
		() =>
			guidanceForm.capabilityRefs.filter((ref) => {
				const option = guidanceCapabilityOptionByKey.get(guidanceCapabilityKey(ref));
				return !option || !option.enabled;
			}),
		[guidanceCapabilityOptionByKey, guidanceForm.capabilityRefs],
	);
	const guidanceValidationIssues = useMemo(() => {
		const issues: string[] = [];
		const slug = guidanceForm.slug.trim();
		if (!slug) {
			issues.push(
				t("profiles:detail.guidance.validation.slugRequired", {
					defaultValue: "Slug is required.",
				}),
			);
		} else if (!/^[A-Za-z0-9_-]+$/.test(slug)) {
			issues.push(
				t("profiles:detail.guidance.validation.slugFormat", {
					defaultValue: "Slug may only contain letters, numbers, hyphen, or underscore.",
				}),
			);
		}
		if (!guidanceForm.title.trim()) {
			issues.push(
				t("profiles:detail.guidance.validation.titleRequired", {
					defaultValue: "Title is required.",
				}),
			);
		}
		if (!guidanceForm.contentMarkdown.trim()) {
			issues.push(
				t("profiles:detail.guidance.validation.instructionsRequired", {
					defaultValue: "Instructions are required.",
				}),
			);
		}
		if (missingGuidanceCapabilities.length > 0) {
			issues.push(
				t("profiles:detail.guidance.validation.missingCapabilities", {
					defaultValue: "{{count}} referenced capability is missing or disabled.",
					count: missingGuidanceCapabilities.length,
				}),
			);
		}
		return issues;
	}, [guidanceForm, missingGuidanceCapabilities.length, t]);

	const toggleGuidanceCapabilityRef = (optionKey: string) => {
		const option = guidanceCapabilityOptionByKey.get(optionKey);
		if (!option) return;
		setGuidanceForm((current) => {
			const currentKey = guidanceCapabilityKey(option);
			const exists = current.capabilityRefs.some((ref) => guidanceCapabilityKey(ref) === currentKey);
			return {
				...current,
				capabilityRefs: exists
					? current.capabilityRefs.filter((ref) => guidanceCapabilityKey(ref) !== currentKey)
					: [
						...current.capabilityRefs,
						{
							kind: option.kind,
							id: option.id,
							name: option.name,
							server_name: option.server_name,
						},
					],
			};
		});
	};

	const applyGuidanceTemplate = () => {
		setGuidanceForm((current) => ({
			...current,
			summary: fillGuidanceTemplateField(
				current.summary,
				t("profiles:detail.guidance.template.summary", {
					defaultValue: "Use this profile when the current task matches the scenario below.",
				}),
			),
			scenario: fillGuidanceTemplateField(
				current.scenario,
				t("profiles:detail.guidance.template.scenario", {
					defaultValue: "Describe the business scenario this profile is designed for.",
				}),
			),
			activation: fillGuidanceTemplateField(
				current.activation,
				t("profiles:detail.guidance.template.activation", {
					defaultValue:
						"Activate when the user asks for this workflow or when these capabilities are required.",
				}),
			),
			validationNotes: fillGuidanceTemplateField(
				current.validationNotes,
				t("profiles:detail.guidance.template.validation", {
					defaultValue: "Confirm required capabilities are enabled before following this guidance.",
				}),
			),
			avoid: fillGuidanceTemplateField(
				current.avoid,
				t("profiles:detail.guidance.template.avoid", {
					defaultValue: "Do not use capabilities outside this profile unless the user explicitly asks.",
				}),
			),
			contentMarkdown: fillGuidanceTemplateField(
				current.contentMarkdown,
				t("profiles:detail.guidance.template.workflow", {
					defaultValue:
						"## Workflow\n1. Confirm the user intent.\n2. Use the referenced MCP capabilities in the profile.\n3. Report any missing capability instead of substituting another tool.",
				}),
			),
		}));
	};

	const enabledByComponentId = useMemo(() => {
		const m = new Map<string, boolean>();
		for (const s of servers) {
			m.set(s.id, s.enabled);
		}
		for (const item of tools) {
			m.set(item.id, item.enabled);
		}
		for (const r of resources) {
			m.set(r.id, r.enabled);
		}
		for (const p of prompts) {
			m.set(p.id, p.enabled);
		}
		for (const tmpl of templates) {
			m.set(tmpl.id, tmpl.enabled);
		}
		return m;
	}, [servers, tools, resources, prompts, templates]);

	const tokenChartSource = useProfileTokenChartSource(profileId, enabledByComponentId);

	// Global servers for availability(connected) calculation
	const { data: globalServersResp } = useQuery({
		queryKey: ["all-servers-for-profile-overview"],
		queryFn: serversApi.getAll,
		staleTime: 30_000,
	});
	const globalServers = globalServersResp?.servers ?? [];
	// For profile counts, available = total in this profile (not global state)
	const availableServersInProfile = servers;

	// Derived server name options for filters
	const serverNameOptions = Array.from(
		new Set(
			[
				...servers.map((s: ConfigSuitServer) => s.name),
				...tools.map((t: ConfigSuitTool) => t.server_name),
				...resources.map((r: ConfigSuitResource) => r.server_name),
				...prompts.map((p: ConfigSuitPrompt) => p.server_name),
				...templates.map((r) => r.server_name),
			].filter(Boolean),
		),
	).sort();

	// Filtered datasets
	const visibleServers = servers.filter((s: ConfigSuitServer) => {
		const queryPass =
			serverQuery.trim() === "" ||
			s.name.toLowerCase().includes(serverQuery.toLowerCase());
		const statusPass =
			serverStatus === "all" ||
			(serverStatus === "enabled" ? s.enabled : !s.enabled);
		return queryPass && statusPass;
	});

	const visibleTools = tools.filter((t: ConfigSuitTool) => {
		const text =
			`${t.tool_name ?? ""} ${t.unique_name ?? ""} ${t.server_name ?? ""}`.toLowerCase();
		const queryPass =
			toolQuery.trim() === "" || text.includes(toolQuery.toLowerCase());
		const statusPass =
			toolStatus === "all" ||
			(toolStatus === "enabled" ? t.enabled : !t.enabled);
		const serverPass = toolServer === "all" || t.server_name === toolServer;
		return queryPass && statusPass && serverPass;
	});

	const visibleResources = resources.filter((r: ConfigSuitResource) => {
		const text = `${r.resource_uri ?? ""} ${r.server_name ?? ""}`.toLowerCase();
		const queryPass =
			resourceQuery.trim() === "" || text.includes(resourceQuery.toLowerCase());
		const statusPass =
			resourceStatus === "all" ||
			(resourceStatus === "enabled" ? r.enabled : !r.enabled);
		const serverPass =
			resourceServer === "all" || r.server_name === resourceServer;
		return queryPass && statusPass && serverPass;
	});

	const visiblePrompts = prompts.filter((p: ConfigSuitPrompt) => {
		const text = `${p.prompt_name ?? ""} ${p.server_name ?? ""}`.toLowerCase();
		const queryPass =
			promptQuery.trim() === "" || text.includes(promptQuery.toLowerCase());
		const statusPass =
			promptStatus === "all" ||
			(promptStatus === "enabled" ? p.enabled : !p.enabled);
		const serverPass = promptServer === "all" || p.server_name === promptServer;
		return queryPass && statusPass && serverPass;
	});

	// Filters: templates
	const [templateQuery, setTemplateQuery] = useState("");
	const [templateStatus, setTemplateStatus] = useState<
		"all" | "enabled" | "disabled"
	>("all");
	const [templateServer, setTemplateServer] = useState<string>("all");

	const visibleTemplates = templates.filter((r) => {
		const text = `${r.uri_template ?? ""} ${r.server_name ?? ""}`.toLowerCase();
		const queryPass =
			templateQuery.trim() === "" || text.includes(templateQuery.toLowerCase());
		const statusPass =
			templateStatus === "all" ||
			(templateStatus === "enabled" ? r.enabled : !r.enabled);
		const serverPass = templateServer === "all" || r.server_name === templateServer;
		return queryPass && statusPass && serverPass;
	});

	// Template toggle mutations
	const templateToggleMutation = useMutation({
		mutationFn: ({ templateId, enable }: { templateId: string; enable: boolean }) =>
			enable
				? configSuitsApi.enableResourceTemplate(profileId!, templateId)
				: configSuitsApi.disableResourceTemplate(profileId!, templateId),
		onSuccess: () => {
			refetchTemplates();
			notifySuccess(
				t("profiles:detail.messages.templateUpdated", { defaultValue: "Template updated" }),
				"Template status has been updated",
			);
		},
		onError: (error) => {
			notifyError(
				t("profiles:detail.messages.templateUpdateFailed", { defaultValue: "Template update failed" }),
				`Failed to update template: ${error instanceof Error ? error.message : String(error)}`,
			);
		},
	});

	return (
		<div className="flex h-full min-h-0 flex-col gap-4 overflow-hidden">
			<div className="flex shrink-0 items-center justify-between">
				<div className="flex items-center">
					{suit && (
						<div className="flex items-center gap-3">
							<div className="flex flex-col">
								<div className="flex items-center gap-3">
									<h2 className="text-3xl font-bold tracking-tight">
										{toTitleCase(suit.name)}
									</h2>
									<Badge variant={suit.is_active ? "default" : "secondary"}>
										{suit.suit_type}
									</Badge>
									{suit.is_active && (
										<span className="flex items-center rounded-full bg-emerald-50 px-2 py-1 text-xs font-medium text-emerald-700 dark:bg-emerald-950/50 dark:text-emerald-400">
											<Check className="mr-1 h-3 w-3" />
											{t("profiles:detail.status.active", { defaultValue: "Active" })}
										</span>
									)}
									{suitRole === DEFAULT_ANCHOR_ROLE ? (
										<Badge variant="outline">{t("profiles:badges.defaultAnchor", { defaultValue: "Default Anchor" })}</Badge>
									) : suit.is_default ? (
										<Badge variant="outline">{t("profiles:badges.inDefault", { defaultValue: "In Default" })}</Badge>
									) : null}
								</div>
								{suit.description && (
									<p className="text-sm text-muted-foreground mt-1">
										{suit.description}
									</p>
								)}
							</div>
						</div>
					)}
				</div>
				<div className="flex flex-shrink-0 items-center gap-3">
					{profileId ? (
						<ProfileTokenUsageChart
							ledgerItems={tokenChartSource.ledgerItems}
							fallbackEstimate={tokenChartSource.fallbackEstimate}
							isLoading={tokenChartSource.isLoading}
							isError={tokenChartSource.isError}
							enabledByComponentId={enabledByComponentId}
							estimateMethod={profileTokenEstimateMethod}
							profileServerCount={
								isLoadingServers ? undefined : servers.length
							}
						/>
					) : null}
				</div>
			</div>

			{!profileId ? (
				<Card>
					<CardContent className="p-4">
						<p className="text-center text-slate-500">
							{t("profiles:detail.labels.profileId", { defaultValue: "Profile ID not provided" })}
						</p>
					</CardContent>
				</Card>
			) : isLoadingSuit ? (
				<Card>
					<CardContent className="p-4">
						<div className="h-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
					</CardContent>
				</Card>
			) : suit ? (
				<Tabs
					value={activeTab}
					onValueChange={setActiveTab}
					className="flex min-h-0 flex-1 flex-col gap-4"
				>
					<div className="flex shrink-0 items-center justify-between">
						<TabsList className="flex items-center gap-2">
							<TabsTrigger value="overview">{t("profiles:detail.tabs.overview", { defaultValue: "Overview" })}</TabsTrigger>
							<TabsTrigger value="guidance">
								{t("profiles:detail.tabs.guidance", { defaultValue: "Guidance" })} ({enabledGuidanceRecords.length}/{guidanceRecords.length})
							</TabsTrigger>
							<TabsTrigger value="servers">
								{t("profiles:detail.tabs.servers", { defaultValue: "Servers" })} ({enabledServers.length}/{servers.length})
							</TabsTrigger>
							<TabsTrigger value="tools">
								{t("profiles:detail.tabs.tools", { defaultValue: "Tools" })} ({enabledTools.length}/{tools.length})
							</TabsTrigger>
							<TabsTrigger value="prompts">
								{t("profiles:detail.tabs.prompts", { defaultValue: "Prompts" })} ({enabledPrompts.length}/{prompts.length})
							</TabsTrigger>
							<TabsTrigger value="resources">
								{t("profiles:detail.tabs.resources", { defaultValue: "Resources" })} ({enabledResources.length}/{resources.length})
							</TabsTrigger>
							<TabsTrigger value="templates">
								{t("profiles:detail.tabs.templates", { defaultValue: "Templates" })} ({enabledTemplates.length}/{templates.length})
							</TabsTrigger>
						</TabsList>
						<ButtonGroup className="ml-auto flex-shrink-0 flex-nowrap self-start">
							<Button
								variant="outline"
								size="sm"
								onClick={handleRefreshAll}
								disabled={isRefetchingSuit}
								className={overviewActionButtonClass}
							>
								<RefreshCw
									className={`h-4 w-4 ${isRefetchingSuit ? "animate-spin" : ""}`}
								/>
								{t("profiles:detail.buttons.refresh", { defaultValue: "Refresh" })}
							</Button>
							<Button
								variant="outline"
								size="sm"
								onClick={() => setIsEditDialogOpen(true)}
								className={overviewActionButtonClass}
							>
								<Edit3 className="h-4 w-4" />
								{t("profiles:detail.buttons.edit", { defaultValue: "Edit" })}
							</Button>
						</ButtonGroup>
					</div>

					<TabsContent
						value="overview"
						className="mt-0 flex min-h-0 flex-1 flex-col overflow-y-auto data-[state=inactive]:hidden"
					>
						<div className="grid gap-4">
							<Card>
								<CardContent className="relative p-4">
									{!isHostApp && !isCustomMode && (
										<div className="absolute right-4 top-4">
											<Button
												variant="destructive"
												size="sm"
												onClick={() => setIsDeleteDialogOpen(true)}
												disabled={!!suit?.is_default}
												className="gap-2"
											>
												<Trash2 className="h-4 w-4" />
												{t("profiles:detail.buttons.delete", { defaultValue: "Delete" })}
											</Button>
										</div>
									)}
									<div className="flex flex-col gap-4">
										<div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_auto] xl:items-start">
											<div className="flex flex-wrap items-start gap-4">
												<Avatar className="text-sm">
													<AvatarFallback>
														{suit.name.slice(0, 1).toUpperCase()}
													</AvatarFallback>
												</Avatar>
												<div className="grid grid-cols-[auto_1fr] gap-x-5 gap-y-2 text-sm">
													<span className="text-xs uppercase text-slate-500">
														{t("profiles:detail.labels.status", { defaultValue: "Status" })}
													</span>
													<Badge
														variant="secondary"
														className={`justify-self-start border px-2.5 py-0.5 leading-none min-h-[1.5rem] ${suit.is_active
															? "border-emerald-200 bg-emerald-100 text-emerald-700 hover:bg-emerald-100 dark:border-emerald-400/50 dark:bg-emerald-500/20 dark:text-emerald-200"
															: "border-slate-200 bg-slate-50 text-slate-600 hover:bg-slate-100 dark:border-slate-700 dark:bg-slate-800/80 dark:text-slate-300"
															}`}
													>
														{suit.is_active ? t("profiles:detail.status.active", { defaultValue: "Active" }) : t("profiles:detail.status.inactive", { defaultValue: "Inactive" })}
													</Badge>

													<span className="text-xs uppercase text-slate-500">
														{t("profiles:detail.labels.type", { defaultValue: "Type" })}
													</span>
													<span className="font-mono text-sm leading-tight">
														{t(`profiles:suitTypes.${suit.suit_type}`, {
															defaultValue: formatProfileTypeLabel(suit.suit_type),
														})}
													</span>

													<span className="text-xs uppercase text-slate-500">
														{t("profiles:detail.labels.multiSelect", { defaultValue: "Multi-select" })}
													</span>
													<span className="text-sm leading-tight">
														{suit.multi_select ? t("profiles:detail.status.yes", { defaultValue: "Yes" }) : t("profiles:detail.status.no", { defaultValue: "No" })}
													</span>

													<span className="text-xs uppercase text-slate-500">
														{t("profiles:detail.labels.priority", { defaultValue: "Priority" })}
													</span>
													<span className="font-mono text-sm leading-tight">
														{suit.priority}
													</span>
												</div>
											</div>
											<ButtonGroup className="ml-auto flex-shrink-0 flex-nowrap self-start">
												{suitRole === "user" && !isHostApp && !isCustomMode && (
													<Button
														variant="outline"
														size="sm"
														onClick={handleSuitToggle}
														disabled={
															isDefaultAnchor ||
															activateSuitMutation.isPending ||
															deactivateSuitMutation.isPending
														}
														className={overviewActionButtonClass}
													>
														{suit?.is_active ? (
															<Square className="h-4 w-4" />
														) : (
															<Play className="h-4 w-4" />
														)}
														{suit?.is_active
															? t("profiles:detail.buttons.disable", { defaultValue: "Disable" })
															: t("profiles:detail.buttons.enable", { defaultValue: "Enable" })}
													</Button>
												)}
											</ButtonGroup>
										</div>
									</div>
								</CardContent>
							</Card>

							<div className="grid grid-cols-2 md:grid-cols-4 gap-4">
								<Card className="h-full">
									<CardHeader
										className="pb-2 cursor-pointer"
										onClick={() => setActiveTab("servers")}
									>
										<CardTitle className="text-sm">
											{t("profiles:detail.labels.servers", { defaultValue: "Servers" })}
										</CardTitle>
									</CardHeader>
									<CardContent>
										<div className="text-2xl font-bold">
											{enabledServers.length}/{availableServersInProfile.length}
										</div>
										<p className="text-xs text-muted-foreground">
											{t("profiles:detail.overview.enabledAvailable", {
												defaultValue: "enabled / available",
											})}
										</p>
									</CardContent>
								</Card>
								<Card className="h-full">
									<CardHeader
										className="pb-2 cursor-pointer"
										onClick={() => setActiveTab("tools")}
									>
										<CardTitle className="text-sm">
											{t("profiles:detail.labels.tools", { defaultValue: "Tools" })}
										</CardTitle>
									</CardHeader>
									<CardContent>
										<div className="text-2xl font-bold">
											{enabledTools.length}/{tools.length}
										</div>
										<p className="text-xs text-muted-foreground">
											{t("profiles:detail.overview.enabledAvailable", {
												defaultValue: "enabled / available",
											})}
										</p>
									</CardContent>
								</Card>
								<Card className="h-full">
									<CardHeader
										className="pb-2 cursor-pointer"
										onClick={() => setActiveTab("resources")}
									>
										<CardTitle className="text-sm">
											{t("profiles:detail.labels.resources", { defaultValue: "Resources" })}
										</CardTitle>
									</CardHeader>
									<CardContent>
										<div className="text-2xl font-bold">
											{enabledResources.length}/{resources.length}
										</div>
										<p className="text-xs text-muted-foreground">
											{t("profiles:detail.overview.enabledAvailable", {
												defaultValue: "enabled / available",
											})}
										</p>
									</CardContent>
								</Card>
								<Card className="h-full">
									<CardHeader
										className="pb-2 cursor-pointer"
										onClick={() => setActiveTab("prompts")}
									>
										<CardTitle className="text-sm">
											{t("profiles:detail.labels.prompts", { defaultValue: "Prompts" })}
										</CardTitle>
									</CardHeader>
									<CardContent>
										<div className="text-2xl font-bold">
											{enabledPrompts.length}/{prompts.length}
										</div>
										<p className="text-xs text-muted-foreground">
											{t("profiles:detail.overview.enabledAvailable", {
												defaultValue: "enabled / available",
											})}
										</p>
									</CardContent>
								</Card>
							</div>
							{showProfileLiveLogs ? (
								<AuditLogsPanel
									title={t("profiles:detail.logs.title", { defaultValue: "Logs" })}
									description={t("profiles:detail.logs.description", {
										defaultValue: "Runtime and activity logs related to this profile.",
									})}
									searchPlaceholder={t("profiles:detail.logs.searchPlaceholder", {
										defaultValue: "Search logs...",
									})}
									refreshLabel={t("profiles:detail.logs.refresh", {
										defaultValue: "Refresh Logs",
									})}
									loadingLabel={t("profiles:detail.logs.loading", {
										defaultValue: "Loading logs...",
									})}
									emptyLabel={t("profiles:detail.logs.empty", {
										defaultValue:
											"No log entries recorded for this profile yet.",
									})}
									headers={{
										timestamp: t("profiles:detail.logs.headers.timestamp", {
											defaultValue: "Timestamp",
										}),
										action: t("profiles:detail.logs.headers.action", {
											defaultValue: "Action",
										}),
										category: t("profiles:detail.logs.headers.category", {
											defaultValue: "Category",
										}),
										status: t("profiles:detail.logs.headers.status", {
											defaultValue: "Status",
										}),
										target: t("profiles:detail.logs.headers.target", {
											defaultValue: "Target",
										}),
									}}
									searchValue={logFilter}
									onSearchChange={setLogFilter}
									onRefresh={() => void profileLogsQuery.refetch()}
									rows={filteredProfileLogs}
									isLoading={profileLogsQuery.isLoading}
									isFetching={profileLogsQuery.isFetching}
									isPaginationActionLoading={isLogPaginationActionLoading}
									currentPage={logCurrentPageIndex + 1}
									hasPreviousPage={logCurrentPageIndex > 0}
									hasNextPage={Boolean(profileLogsQuery.data?.next_cursor)}
									itemsPerPage={logPageSize}
									onItemsPerPageChange={setLogPageSize}
									onPreviousPage={handleProfileLogsPrevPage}
									onFirstPage={handleProfileLogsFirstPage}
									onNextPage={handleProfileLogsNextPage}
									onLastPage={() => void handleProfileLogsLastPage()}
									expandLabel={t("profiles:detail.logs.expand", {
										defaultValue: "Expand Logs",
									})}
									collapseLabel={t("profiles:detail.logs.collapse", {
										defaultValue: "Collapse Logs",
									})}
								/>
							) : null}
						</div>
					</TabsContent>

					<TabsContent value="guidance" className={DETAIL_TAB_CONTENT_CLASS}>
						<Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
							<CardHeader className="shrink-0">
								<div className="flex items-center justify-between gap-3">
									<div>
										<CardTitle>
											{t("profiles:detail.guidance.title", {
												defaultValue: "Profile Guidance",
											})}
										</CardTitle>
										<CardDescription>
											{t("profiles:detail.guidance.description", {
												defaultValue:
													"Describe how agents should use this profile's MCP capabilities.",
											})}
										</CardDescription>
									</div>
									<ButtonGroup className="flex-shrink-0">
										<Button
											type="button"
											variant="outline"
											size="sm"
											onClick={applyGuidanceTemplate}
											disabled={guidanceSaveMutation.isPending}
											className="gap-2 rounded-none first:rounded-l-md last:rounded-r-md"
										>
											<FileText className="h-4 w-4" />
											{t("profiles:detail.guidance.buttons.template", {
												defaultValue: "Template",
											})}
										</Button>
										<Button
											type="button"
											variant="outline"
											size="sm"
											onClick={handleNewGuidance}
											disabled={guidanceSaveMutation.isPending}
											className="gap-2 rounded-none first:rounded-l-md last:rounded-r-md"
										>
											<Plus className="h-4 w-4" />
											{t("profiles:detail.guidance.buttons.new", {
												defaultValue: "New Guidance",
											})}
										</Button>
									</ButtonGroup>
								</div>
							</CardHeader>
							<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-4 pt-2">
								<div className="grid min-h-0 flex-1 gap-4 lg:grid-cols-[20rem_minmax(0,1fr)]">
									<div className="min-h-0 overflow-y-auto rounded-md border">
										{isLoadingGuidance ? (
											<div className="space-y-3 p-3">
												{["g1", "g2", "g3"].map((id) => (
													<div
														key={`guidance-skel-${id}`}
														className="h-20 animate-pulse rounded-md bg-slate-200 dark:bg-slate-800"
													/>
												))}
											</div>
										) : guidanceRecords.length > 0 ? (
											<div className="divide-y">
												{guidanceRecords.map((guidance) => {
													const selected = guidance.id === selectedGuidanceId;
													return (
														<button
															key={guidance.id}
															type="button"
															onClick={() => handleSelectGuidance(guidance)}
															className={`flex w-full items-start gap-3 p-3 text-left transition-colors ${selected
																? "bg-primary/10"
																: "hover:bg-muted/60"
																}`}
														>
															<BookOpenText className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
															<span className="min-w-0 flex-1">
																<span className="flex items-center gap-2">
																	<span className="truncate text-sm font-medium">
																		{guidance.title}
																	</span>
																	<Badge
																		variant={guidance.enabled ? "default" : "outline"}
																		className="shrink-0"
																	>
																		{guidance.enabled
																			? t("profiles:detail.status.enabled", {
																				defaultValue: "Enabled",
																			})
																			: t("profiles:detail.status.disabled", {
																				defaultValue: "Disabled",
																			})}
																	</Badge>
																</span>
																<span className="mt-1 block truncate font-mono text-xs text-muted-foreground">
																	{guidance.slug}
																</span>
																{guidance.summary ? (
																	<span className="mt-1 block line-clamp-2 text-xs text-muted-foreground">
																		{guidance.summary}
																	</span>
																) : null}
															</span>
														</button>
													);
												})}
											</div>
										) : (
											<div className="p-4 text-sm text-muted-foreground">
												{t("profiles:detail.guidance.empty", {
													defaultValue: "No guidance has been added yet.",
												})}
											</div>
										)}
									</div>

									<form
										className="flex min-h-0 flex-col gap-4 overflow-y-auto rounded-md border p-4"
										onSubmit={(event) => {
											event.preventDefault();
											if (guidanceValidationIssues.length > 0) {
												notifyError(
													t("profiles:detail.guidance.messages.validationFailed", {
														defaultValue: "Guidance is incomplete",
													}),
													guidanceValidationIssues.join("\n"),
												);
												return;
											}
											guidanceSaveMutation.mutate();
										}}
									>
										<div className="grid gap-4 md:grid-cols-2">
											<div className="space-y-2">
												<Label htmlFor={guidanceSlugId}>
													{t("profiles:detail.guidance.fields.slug", {
														defaultValue: "Slug",
													})}
												</Label>
												<Input
													id={guidanceSlugId}
													value={guidanceForm.slug}
													onChange={(event) =>
														updateGuidanceForm("slug", event.target.value)
													}
													disabled={guidanceSaveMutation.isPending}
												/>
											</div>
											<div className="space-y-2">
												<Label htmlFor={guidanceTitleId}>
													{t("profiles:detail.guidance.fields.title", {
														defaultValue: "Title",
													})}
												</Label>
												<Input
													id={guidanceTitleId}
													value={guidanceForm.title}
													onChange={(event) =>
														updateGuidanceForm("title", event.target.value)
													}
													disabled={guidanceSaveMutation.isPending}
												/>
											</div>
										</div>
										<div className="space-y-2">
											<Label htmlFor={guidanceSummaryId}>
												{t("profiles:detail.guidance.fields.summary", {
													defaultValue: "Summary",
												})}
											</Label>
											<Input
												id={guidanceSummaryId}
												value={guidanceForm.summary}
												onChange={(event) =>
													updateGuidanceForm("summary", event.target.value)
												}
												disabled={guidanceSaveMutation.isPending}
											/>
										</div>
										<div className="grid gap-4 md:grid-cols-2">
											<div className="space-y-2">
												<Label htmlFor={guidanceScenarioId}>
													{t("profiles:detail.guidance.fields.scenario", {
														defaultValue: "Scenario",
													})}
												</Label>
												<Textarea
													id={guidanceScenarioId}
													value={guidanceForm.scenario}
													onChange={(event) =>
														updateGuidanceForm("scenario", event.target.value)
													}
													disabled={guidanceSaveMutation.isPending}
													className="min-h-24 text-sm"
												/>
											</div>
											<div className="space-y-2">
												<Label htmlFor={guidanceActivationId}>
													{t("profiles:detail.guidance.fields.activation", {
														defaultValue: "Activation",
													})}
												</Label>
												<Textarea
													id={guidanceActivationId}
													value={guidanceForm.activation}
													onChange={(event) =>
														updateGuidanceForm("activation", event.target.value)
													}
													disabled={guidanceSaveMutation.isPending}
													className="min-h-24 text-sm"
												/>
											</div>
										</div>
										<div className="space-y-2">
											<Label htmlFor={guidanceSourceUriId}>
												{t("profiles:detail.guidance.fields.sourceUri", {
													defaultValue: "Source URI",
												})}
											</Label>
											<Input
												id={guidanceSourceUriId}
												value={guidanceForm.sourceUri}
												onChange={(event) =>
													updateGuidanceForm("sourceUri", event.target.value)
												}
												disabled={guidanceSaveMutation.isPending}
											/>
										</div>
										<div className="space-y-2">
											<div className="flex items-center justify-between gap-2">
												<Label className="flex items-center gap-2">
													<ListChecks className="h-4 w-4 text-muted-foreground" />
													{t("profiles:detail.guidance.fields.capabilities", {
														defaultValue: "Capability References",
													})}
												</Label>
												<Badge variant="outline">
													{guidanceForm.capabilityRefs.length}
												</Badge>
											</div>
											<div className="max-h-44 overflow-y-auto rounded-md border">
												{guidanceCapabilityOptions.length > 0 ? (
													<div className="grid gap-0 divide-y">
														{guidanceCapabilityOptions.map((option) => {
															const selected = selectedGuidanceCapabilityKeys.has(option.key);
															const disabled = !option.enabled;
															return (
																<button
																	key={option.key}
																	type="button"
																	onClick={() => toggleGuidanceCapabilityRef(option.key)}
																	className={`flex items-center justify-between gap-3 p-2 text-left text-sm transition-colors ${selected
																		? "bg-primary/10"
																		: "hover:bg-muted/60"
																		}`}
																>
																	<span className="min-w-0">
																		<span className="flex items-center gap-2">
																			<span className="font-mono text-xs text-muted-foreground">
																				{option.kind}
																			</span>
																			<span className="truncate font-medium">
																				{option.label}
																			</span>
																		</span>
																		<span className="mt-0.5 block truncate text-xs text-muted-foreground">
																			{option.detail}
																		</span>
																	</span>
																	<span className="flex shrink-0 items-center gap-2">
																		{disabled ? (
																			<Badge variant="outline">
																				{t("profiles:detail.status.disabled", {
																					defaultValue: "Disabled",
																				})}
																			</Badge>
																		) : null}
																		{selected ? <Check className="h-4 w-4" /> : null}
																	</span>
																</button>
															);
														})}
													</div>
												) : (
													<div className="p-3 text-sm text-muted-foreground">
														{t("profiles:detail.guidance.capabilities.empty", {
															defaultValue: "No profile capabilities are available.",
														})}
													</div>
												)}
											</div>
											{missingGuidanceCapabilities.length > 0 ? (
												<div className="flex items-start gap-2 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800 dark:border-amber-500/40 dark:bg-amber-500/10 dark:text-amber-200">
													<AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
													<div>
														<div className="font-medium">
															{t("profiles:detail.guidance.capabilities.missingTitle", {
																defaultValue: "Missing or disabled references",
															})}
														</div>
														<div className="mt-1 font-mono text-xs">
															{missingGuidanceCapabilities
																.map((capability) => `${capability.kind}:${capability.id}`)
																.join(", ")}
														</div>
													</div>
												</div>
											) : null}
										</div>
										<div className="space-y-2">
											<div className="flex items-center justify-between gap-2">
												<Label htmlFor={guidanceInstructionsId}>
													{t("profiles:detail.guidance.fields.instructions", {
														defaultValue: "Instructions",
													})}
												</Label>
												<ButtonGroup>
													<Button
														type="button"
														size="sm"
														variant={guidancePreviewMode === "edit" ? "default" : "outline"}
														onClick={() => setGuidancePreviewMode("edit")}
														className="rounded-none first:rounded-l-md last:rounded-r-md"
													>
														{t("profiles:detail.guidance.preview.edit", {
															defaultValue: "Edit",
														})}
													</Button>
													<Button
														type="button"
														size="sm"
														variant={guidancePreviewMode === "preview" ? "default" : "outline"}
														onClick={() => setGuidancePreviewMode("preview")}
														className="rounded-none first:rounded-l-md last:rounded-r-md"
													>
														{t("profiles:detail.guidance.preview.preview", {
															defaultValue: "Preview",
														})}
													</Button>
												</ButtonGroup>
											</div>
											{guidancePreviewMode === "edit" ? (
												<Textarea
													id={guidanceInstructionsId}
													value={guidanceForm.contentMarkdown}
													onChange={(event) =>
														updateGuidanceForm("contentMarkdown", event.target.value)
													}
													disabled={guidanceSaveMutation.isPending}
													className="min-h-64 font-mono text-sm"
												/>
											) : (
												<div className="min-h-64 overflow-y-auto rounded-md border bg-muted/30 p-4 text-sm">
													{guidancePreviewMarkdown ? (
														<ReactMarkdown
															remarkPlugins={[remarkGfm]}
															components={{
																h1: ({ children }) => (
																	<h1 className="mb-3 text-lg font-semibold">{children}</h1>
																),
																h2: ({ children }) => (
																	<h2 className="mb-2 mt-4 text-sm font-semibold">{children}</h2>
																),
																p: ({ children }) => (
																	<p className="mb-2 leading-6 text-muted-foreground">{children}</p>
																),
																ul: ({ children }) => (
																	<ul className="mb-2 list-disc space-y-1 pl-5">{children}</ul>
																),
																ol: ({ children }) => (
																	<ol className="mb-2 list-decimal space-y-1 pl-5">{children}</ol>
																),
																code: ({ children }) => (
																	<code className="rounded bg-muted px-1 py-0.5 font-mono text-xs">
																		{children}
																	</code>
																),
															}}
														>
															{guidancePreviewMarkdown}
														</ReactMarkdown>
													) : (
														<div className="text-muted-foreground">
															{t("profiles:detail.guidance.preview.empty", {
																defaultValue: "No preview content.",
															})}
														</div>
													)}
												</div>
											)}
										</div>
										<div className="grid gap-4 md:grid-cols-2">
											<div className="space-y-2">
												<Label htmlFor={guidanceValidationId}>
													{t("profiles:detail.guidance.fields.validationNotes", {
														defaultValue: "Validation Notes",
													})}
												</Label>
												<Textarea
													id={guidanceValidationId}
													value={guidanceForm.validationNotes}
													onChange={(event) =>
														updateGuidanceForm("validationNotes", event.target.value)
													}
													disabled={guidanceSaveMutation.isPending}
													className="min-h-24 text-sm"
												/>
											</div>
											<div className="space-y-2">
												<Label htmlFor={guidanceAvoidId}>
													{t("profiles:detail.guidance.fields.avoid", {
														defaultValue: "Avoid",
													})}
												</Label>
												<Textarea
													id={guidanceAvoidId}
													value={guidanceForm.avoid}
													onChange={(event) =>
														updateGuidanceForm("avoid", event.target.value)
													}
													disabled={guidanceSaveMutation.isPending}
													className="min-h-24 text-sm"
												/>
											</div>
										</div>
										<div className="flex flex-wrap items-center justify-between gap-3 rounded-md bg-muted/50 p-3">
											<div className="min-w-0">
												<div className="text-sm font-medium">
													{t("profiles:detail.guidance.resourceUri", {
														defaultValue: "Resource URI",
													})}
												</div>
												<div className="mt-1 break-all font-mono text-xs text-muted-foreground">
													{guidanceResourceUri || "skill://profiles/{profile_id}/{slug}/SKILL.md"}
												</div>
											</div>
											<div className="flex items-center gap-2">
												<Label htmlFor={guidanceEnabledId} className="text-sm">
													{t("profiles:detail.guidance.fields.enabled", {
														defaultValue: "Expose",
													})}
												</Label>
												<Switch
													id={guidanceEnabledId}
													checked={guidanceForm.enabled}
													onCheckedChange={(checked) =>
														updateGuidanceForm("enabled", checked)
													}
													disabled={guidanceSaveMutation.isPending}
												/>
											</div>
										</div>
										{guidanceValidationIssues.length > 0 ? (
											<div className="rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800 dark:border-amber-500/40 dark:bg-amber-500/10 dark:text-amber-200">
												<div className="flex items-center gap-2 font-medium">
													<AlertTriangle className="h-4 w-4" />
													{t("profiles:detail.guidance.validation.title", {
														defaultValue: "Fix before saving",
													})}
												</div>
												<ul className="mt-2 list-disc space-y-1 pl-5">
													{guidanceValidationIssues.map((issue) => (
														<li key={issue}>{issue}</li>
													))}
												</ul>
											</div>
										) : null}
										<div className="flex flex-wrap items-center justify-between gap-3">
											<Button
												type="button"
												variant="destructive"
												disabled={
													!selectedGuidanceRecord ||
													guidanceDeleteMutation.isPending ||
													guidanceSaveMutation.isPending
												}
												onClick={() => setIsGuidanceDeleteDialogOpen(true)}
												className="gap-2"
											>
												<Trash2 className="h-4 w-4" />
												{guidanceDeleteMutation.isPending
													? t("profiles:detail.guidance.buttons.deleting", {
														defaultValue: "Deleting...",
													})
													: t("profiles:detail.guidance.buttons.delete", {
														defaultValue: "Delete Guidance",
													})}
											</Button>
											<Button
												type="submit"
												disabled={
													guidanceSaveMutation.isPending ||
													guidanceDeleteMutation.isPending ||
													guidanceValidationIssues.length > 0
												}
												className="gap-2"
											>
												{guidanceSaveMutation.isPending
													? t("profiles:detail.guidance.buttons.saving", {
														defaultValue: "Saving...",
													})
													: t("profiles:detail.guidance.buttons.save", {
														defaultValue: "Save Guidance",
													})}
												{guidanceHasChanges ? null : (
													<span className="sr-only">
														{t("profiles:detail.guidance.status.unchanged", {
															defaultValue: "No changes",
														})}
													</span>
												)}
											</Button>
										</div>
									</form>
								</div>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="servers" className={DETAIL_TAB_CONTENT_CLASS}>
						<Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
							<CardHeader className="shrink-0">
								<div className="flex items-center justify-between gap-2">
									<div>
										<CardTitle>
											{t("profiles:detail.labels.servers", { defaultValue: "Servers" })}
										</CardTitle>
										<CardDescription>
											{t("profiles:detail.descriptions.servers", {
												defaultValue: "Manage servers included in this profile",
											})}
										</CardDescription>
									</div>
									{!isLoadingServers && (
										<div className="flex flex-wrap items-center gap-2">
											<Input
												placeholder={t("profiles:detail.placeholders.searchServers", {
													defaultValue: "Search servers...",
												})}
												value={serverQuery}
												onChange={(e) => setServerQuery(e.target.value)}
												className="w-48 h-9"
											/>
											<div className="hidden xl:block">
												<Select
													value={serverStatus}
													onValueChange={(v) =>
														setServerStatus(v as "all" | "enabled" | "disabled")
													}
												>
													<SelectTrigger className="w-36 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.status", { defaultValue: "Status" })} />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">
															{t("profiles:detail.filters.status.all", { defaultValue: "All" })}
														</SelectItem>
														<SelectItem value="enabled">
															{t("profiles:detail.filters.status.enabled", { defaultValue: "Enabled" })}
														</SelectItem>
														<SelectItem value="disabled">
															{t("profiles:detail.filters.status.disabled", { defaultValue: "Disabled" })}
														</SelectItem>
													</SelectContent>
												</Select>
											</div>
											<ButtonGroup className="hidden md:flex ml-2">
												<Button
													variant="outline"
													size="sm"
													onClick={() =>
														setSelectedServerIds(
															visibleServers.map((s: any) => s.id),
														)
													}
												>
													{t("profiles:detail.buttons.selectAll", {
														defaultValue: "Select all",
													})}
												</Button>
												<Button
													variant="outline"
													size="sm"
													onClick={() => setSelectedServerIds([])}
												>
													{t("profiles:detail.buttons.clearSelection", {
														defaultValue: "Clear",
													})}
												</Button>
												<Button
													size="sm"
													disabled={
														bulkServersM.isPending ||
														selectedServerIds.length === 0
													}
													onClick={() => bulkServersM.mutate({ enable: true })}
												>
													{t("profiles:detail.buttons.enable", { defaultValue: "Enable" })}
												</Button>
												<Button
													size="sm"
													variant="secondary"
													disabled={
														bulkServersM.isPending ||
														selectedServerIds.length === 0
													}
													onClick={() => bulkServersM.mutate({ enable: false })}
												>
													{t("profiles:detail.buttons.disable", { defaultValue: "Disable" })}
												</Button>
											</ButtonGroup>
										</div>
									)}
								</div>
							</CardHeader>
							<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-4 pt-2">
								<CardListScrollBody>
									{isLoadingServers ? (
										<div className="space-y-4">
											{["s1", "s2", "s3"].map((id) => (
												<div
													key={`servers-skel-${id}`}
													className="flex items-center justify-between rounded-lg border p-4"
												>
													<div className="h-5 w-32 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
													<div className="h-6 w-12 animate-pulse rounded bg-slate-200 dark:bg-slate-800"></div>
												</div>
											))}
										</div>
									) : visibleServers.length > 0 ? (
										<CapsuleStripeList className="rounded-none border-0 overflow-visible">
											{visibleServers.map((server) => {
												const global = (globalServers as any[]).find(
													(gs: any) => gs.name === server.name,
												);
												const globallyEnabled: boolean | undefined =
													global?.enabled;
												const globalIcon = global?.icons?.[0]?.src;
												const avatarFallback = (server.name || server.id || "S")
													.slice(0, 1)
													.toUpperCase();
												const iconAlt = global?.name || server.name || server.id;
												const globalDescription =
													global?.meta?.description?.trim();
												const selected = selectedServerIds.includes(server.id);
												return (
													<CapsuleStripeListItem
														key={server.id}
														interactive
														className={`group relative transition-colors ${selected
															? "bg-primary/10 ring-1 ring-primary/40"
															: ""
															}`}
														onClick={() =>
															setSelectedServerIds((prev) =>
																prev.includes(server.id)
																	? prev.filter((x) => x !== server.id)
																	: [...prev, server.id],
															)
														}
														onKeyDown={(e) => {
															if (e.key === "Enter" || e.key === " ") {
																e.preventDefault();
																setSelectedServerIds((prev) =>
																	prev.includes(server.id)
																		? prev.filter((x) => x !== server.id)
																		: [...prev, server.id],
																);
															}
														}}
													>
														<div className="flex w-full items-center justify-between gap-4">
															<div className="flex flex-1 items-center gap-3">
																<div
																	className={`flex h-6 w-6 items-center justify-center rounded-full border text-[0px] transition-all duration-200 ${selected
																		? "border-primary bg-primary text-white shadow-sm"
																		: "border-slate-300 text-transparent group-hover:border-primary/50 group-hover:text-primary/60 dark:border-slate-700 dark:group-hover:border-primary/50"
																		}`}
																>
																	<Check className="h-3 w-3" />
																</div>
																<CachedAvatar
																	src={globalIcon}
																	alt={iconAlt ? `${iconAlt} icon` : undefined}
																	fallback={avatarFallback}
																	size="sm"
																	shape="rounded"
																	className="border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-900/40"
																/>
																<div className="min-w-0">
																	<h3 className="font-medium text-slate-900 dark:text-slate-100">
																		{server.name}
																	</h3>
																	<p className="text-sm text-slate-500">
																		ID: {server.id}
																	</p>
																	{globalDescription ? (
																		<p className="text-xs text-slate-500 line-clamp-2">
																			{globalDescription}
																		</p>
																	) : null}
																</div>
															</div>
															<div className="ml-auto flex items-center gap-2">
																{/* Hover actions: Browse (left) + Inspect (right) */}
																{enableServerDebug && (
																	<>
																		<div className="opacity-0 group-hover:opacity-100 transition-opacity duration-200">
																			<button
																				type="button"
																				onClick={(ev) => {
																					ev.stopPropagation();
																					openBrowse(server.id);
																				}}
																				aria-label={t("profiles:detail.labels.browseServer", { defaultValue: "Browse server" })}
																				className="p-2 text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100 transition-colors"
																			>
																				<Eye size={20} />
																			</button>
																		</div>
																		<div className="opacity-0 group-hover:opacity-100 transition-opacity duration-200">
																			<button
																				type="button"
																				onClick={(ev) => {
																					ev.stopPropagation();
																					openDebug(
																						server.id,
																						server.enabled ? "proxy" : "native",
																					);
																				}}
																				aria-label={t("profiles:detail.labels.debugServer", { defaultValue: "Inspect server" })}
																				className="p-2 text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100 transition-colors"
																			>
																				<Bug size={20} />
																			</button>
																		</div>
																	</>
																)}

																{/* Global status badges and switch - positioned on the right */}
																{globallyEnabled !== undefined &&
																	(globallyEnabled ? (
																		<Badge>
																			{t("profiles:detail.globalStatus.enabled", {
																				defaultValue: "Global Enabled",
																			})}
																		</Badge>
																	) : (
																		<Badge variant="outline">
																			{t("profiles:detail.globalStatus.disabled", {
																				defaultValue: "Global Disabled",
																			})}
																		</Badge>
																	))}

																{/* Always show switch */}
																<Switch
																	checked={server.enabled}
																	onClick={(e) => e.stopPropagation()}
																	onCheckedChange={(enabled) =>
																		serverToggleMutation.mutate({
																			serverId: server.id,
																			enable: enabled,
																		})
																	}
																	disabled={serverToggleMutation.isPending}
																/>
															</div>
														</div>
													</CapsuleStripeListItem>
												);
											})}
										</CapsuleStripeList>
									) : (
										<p className="text-center text-slate-500 py-8">
											{t("profiles:detail.emptyStates.noServers", {
												defaultValue: "No servers found in this profile",
											})}
										</p>
									)}
								</CardListScrollBody>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="tools" className={DETAIL_TAB_CONTENT_CLASS}>
						<Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
							<CardHeader className="shrink-0">
								<div className="flex items-center justify-between gap-2">
									<div>
										<CardTitle>
											{t("profiles:detail.labels.tools", { defaultValue: "Tools" })}
										</CardTitle>
										<CardDescription>
											{t("profiles:detail.descriptions.tools", {
												defaultValue: "Manage tools included in this profile",
											})}
										</CardDescription>
									</div>
									{!isLoadingTools && (
										<div className="flex flex-wrap items-center gap-2">
											<Input
												placeholder={t("profiles:detail.placeholders.searchTools", {
													defaultValue: "Search tools...",
												})}
												value={toolQuery}
												onChange={(e) => setToolQuery(e.target.value)}
												className="w-48 h-9"
											/>
											<div className="hidden xl:block">
												<Select
													value={toolStatus}
													onValueChange={(v) =>
														setToolStatus(v as "all" | "enabled" | "disabled")
													}
												>
													<SelectTrigger className="w-36 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.status", { defaultValue: "Status" })} />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">
															{t("profiles:detail.filters.status.all", { defaultValue: "All" })}
														</SelectItem>
														<SelectItem value="enabled">
															{t("profiles:detail.filters.status.enabled", { defaultValue: "Enabled" })}
														</SelectItem>
														<SelectItem value="disabled">
															{t("profiles:detail.filters.status.disabled", { defaultValue: "Disabled" })}
														</SelectItem>
													</SelectContent>
												</Select>
											</div>
											<div className="hidden xl:block">
												<Select
													value={toolServer}
													onValueChange={(v) => setToolServer(v)}
												>
													<SelectTrigger className="w-40 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.server", { defaultValue: "Server" })} />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">
															{t("profiles:detail.filters.server.all", {
																defaultValue: "All Servers",
															})}
														</SelectItem>
														{serverNameOptions.map((name) => (
															<SelectItem key={`tool-sel-${name}`} value={name}>
																{name}
															</SelectItem>
														))}
													</SelectContent>
												</Select>
											</div>
											<ButtonGroup className="hidden md:flex ml-2">
												<Button
													variant="outline"
													size="sm"
													onClick={() =>
														setSelectedToolIds(
															visibleTools.map((t: any) => t.id),
														)
													}
												>
													{t("profiles:detail.buttons.selectAll", {
														defaultValue: "Select all",
													})}
												</Button>
												<Button
													variant="outline"
													size="sm"
													onClick={() => setSelectedToolIds([])}
												>
													{t("profiles:detail.buttons.clearSelection", {
														defaultValue: "Clear",
													})}
												</Button>
												<Button
													size="sm"
													disabled={
														bulkToolsM.isPending || selectedToolIds.length === 0
													}
													onClick={() => bulkToolsM.mutate({ enable: true })}
												>
													{t("profiles:detail.buttons.enable", { defaultValue: "Enable" })}
												</Button>
												<Button
													size="sm"
													variant="secondary"
													disabled={
														bulkToolsM.isPending || selectedToolIds.length === 0
													}
													onClick={() => bulkToolsM.mutate({ enable: false })}
												>
													{t("profiles:detail.buttons.disable", { defaultValue: "Disable" })}
												</Button>
											</ButtonGroup>
										</div>
									)}
								</div>
							</CardHeader>
							<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-4 pt-2">
								<CapabilityList
									asCard={false}
									scrollContainedBody
									title={t("profiles:detail.labels.tools", { defaultValue: "Tools" })}
									kind="tools"
									context="profile"
									items={visibleTools as any}
									loading={isLoadingTools}
									enableToggle
									getId={(t: any) => t.id}
									getEnabled={(t: any) => !!t.enabled}
									onToggle={(id, next) =>
										toolToggleMutation.mutate({ toolId: id, enable: next })
									}
									emptyText={t("profiles:detail.emptyStates.noTools", { defaultValue: "No tools found in this profile" })}
									filterText={toolQuery}
									selectable
									selectedIds={selectedToolIds}
									onSelectToggle={(id) => {
										setSelectedToolIds((prev) =>
											prev.includes(id)
												? prev.filter((x) => x !== id)
												: [...prev, id],
										);
									}}
									renderAction={undefined}
								/>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="prompts" className={DETAIL_TAB_CONTENT_CLASS}>
						<Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
							<CardHeader className="shrink-0">
								<div className="flex items-center justify-between gap-2">
									<div>
										<CardTitle>
											{t("profiles:detail.labels.prompts", { defaultValue: "Prompts" })}
										</CardTitle>
										<CardDescription>
											{t("profiles:detail.descriptions.prompts", {
												defaultValue: "Manage prompts included in this profile",
											})}
										</CardDescription>
									</div>
									{!isLoadingPrompts && (
										<div className="flex flex-wrap items-center gap-2">
											<Input
												placeholder={t("profiles:detail.placeholders.searchPrompts", {
													defaultValue: "Search prompts...",
												})}
												value={promptQuery}
												onChange={(e) => setPromptQuery(e.target.value)}
												className="w-48 h-9"
											/>
											<div className="hidden xl:block">
												<Select
													value={promptStatus}
													onValueChange={(v) =>
														setPromptStatus(v as "all" | "enabled" | "disabled")
													}
												>
													<SelectTrigger className="w-36 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.status", { defaultValue: "Status" })} />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">
															{t("profiles:detail.filters.status.all", { defaultValue: "All" })}
														</SelectItem>
														<SelectItem value="enabled">
															{t("profiles:detail.filters.status.enabled", { defaultValue: "Enabled" })}
														</SelectItem>
														<SelectItem value="disabled">
															{t("profiles:detail.filters.status.disabled", { defaultValue: "Disabled" })}
														</SelectItem>
													</SelectContent>
												</Select>
											</div>
											<div className="hidden xl:block">
												<Select
													value={promptServer}
													onValueChange={(v) => setPromptServer(v)}
												>
													<SelectTrigger className="w-40 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.server", { defaultValue: "Server" })} />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">
															{t("profiles:detail.filters.server.all", {
																defaultValue: "All Servers",
															})}
														</SelectItem>
														{serverNameOptions.map((name) => (
															<SelectItem key={`prm-sel-${name}`} value={name}>
																{name}
															</SelectItem>
														))}
													</SelectContent>
												</Select>
											</div>
											<ButtonGroup className="hidden md:flex ml-2">
												<Button
													variant="outline"
													size="sm"
													onClick={() =>
														setSelectedPromptIds(
															visiblePrompts.map((p: any) => p.id),
														)
													}
												>
													{t("profiles:detail.buttons.selectAll", {
														defaultValue: "Select all",
													})}
												</Button>
												<Button
													variant="outline"
													size="sm"
													onClick={() => setSelectedPromptIds([])}
												>
													{t("profiles:detail.buttons.clearSelection", {
														defaultValue: "Clear",
													})}
												</Button>
												<Button
													size="sm"
													disabled={
														bulkPromptsM.isPending ||
														selectedPromptIds.length === 0
													}
													onClick={() => bulkPromptsM.mutate({ enable: true })}
												>
													{t("profiles:detail.buttons.enable", { defaultValue: "Enable" })}
												</Button>
												<Button
													size="sm"
													variant="secondary"
													disabled={
														bulkPromptsM.isPending ||
														selectedPromptIds.length === 0
													}
													onClick={() => bulkPromptsM.mutate({ enable: false })}
												>
													{t("profiles:detail.buttons.disable", { defaultValue: "Disable" })}
												</Button>
											</ButtonGroup>
										</div>
									)}
								</div>
							</CardHeader>
							<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-4 pt-2">
								<CapabilityList
									asCard={false}
									scrollContainedBody
									title={t("profiles:detail.labels.prompts", { defaultValue: "Prompts" })}
									kind="prompts"
									context="profile"
									items={visiblePrompts as any}
									loading={isLoadingPrompts}
									enableToggle
									getId={(p: any) => p.id}
									getEnabled={(p: any) => !!p.enabled}
									onToggle={(id, next) =>
										promptToggleMutation.mutate({ promptId: id, enable: next })
									}
									emptyText={t("profiles:detail.emptyStates.noPrompts", { defaultValue: "No prompts found in this profile" })}
									filterText={promptQuery}
									selectable
									selectedIds={selectedPromptIds}
									onSelectToggle={(id) => {
										setSelectedPromptIds((prev) =>
											prev.includes(id)
												? prev.filter((x) => x !== id)
												: [...prev, id],
										);
									}}
									renderAction={undefined}
								/>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="resources" className={DETAIL_TAB_CONTENT_CLASS}>
						<Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
							<CardHeader className="shrink-0">
								<div className="flex items-center justify-between gap-2">
									<div>
										<CardTitle>
											{t("profiles:detail.labels.resources", { defaultValue: "Resources" })}
										</CardTitle>
										<CardDescription>
											{t("profiles:detail.descriptions.resources", {
												defaultValue: "Manage resources included in this profile",
											})}
										</CardDescription>
									</div>
									{!isLoadingResources && (
										<div className="flex flex-wrap items-center gap-2">
											<Input
												placeholder={t("profiles:detail.placeholders.searchResources", {
													defaultValue: "Search resources...",
												})}
												value={resourceQuery}
												onChange={(e) => setResourceQuery(e.target.value)}
												className="w-48 h-9"
											/>
											<div className="hidden xl:block">
												<Select
													value={resourceStatus}
													onValueChange={(v) =>
														setResourceStatus(
															v as "all" | "enabled" | "disabled",
														)
													}
												>
													<SelectTrigger className="w-36 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.status", { defaultValue: "Status" })} />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">
															{t("profiles:detail.filters.status.all", { defaultValue: "All" })}
														</SelectItem>
														<SelectItem value="enabled">
															{t("profiles:detail.filters.status.enabled", { defaultValue: "Enabled" })}
														</SelectItem>
														<SelectItem value="disabled">
															{t("profiles:detail.filters.status.disabled", { defaultValue: "Disabled" })}
														</SelectItem>
													</SelectContent>
												</Select>
											</div>
											<div className="hidden xl:block">
												<Select
													value={resourceServer}
													onValueChange={(v) => setResourceServer(v)}
												>
													<SelectTrigger className="w-40 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.server", { defaultValue: "Server" })} />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">
															{t("profiles:detail.filters.server.all", {
																defaultValue: "All Servers",
															})}
														</SelectItem>
														{serverNameOptions.map((name) => (
															<SelectItem key={`res-sel-${name}`} value={name}>
																{name}
															</SelectItem>
														))}
													</SelectContent>
												</Select>
											</div>
											<ButtonGroup className="hidden md:flex ml-2">
												<Button
													variant="outline"
													size="sm"
													onClick={() =>
														setSelectedResourceIds(
															visibleResources.map((r: any) => r.id),
														)
													}
												>
													{t("profiles:detail.buttons.selectAll", {
														defaultValue: "Select all",
													})}
												</Button>
												<Button
													variant="outline"
													size="sm"
													onClick={() => setSelectedResourceIds([])}
												>
													{t("profiles:detail.buttons.clearSelection", {
														defaultValue: "Clear",
													})}
												</Button>
												<Button
													size="sm"
													disabled={
														bulkResourcesM.isPending ||
														selectedResourceIds.length === 0
													}
													onClick={() =>
														bulkResourcesM.mutate({ enable: true })
													}
												>
													{t("profiles:detail.buttons.enable", { defaultValue: "Enable" })}
												</Button>
												<Button
													size="sm"
													variant="secondary"
													disabled={
														bulkResourcesM.isPending ||
														selectedResourceIds.length === 0
													}
													onClick={() =>
														bulkResourcesM.mutate({ enable: false })
													}
												>
													{t("profiles:detail.buttons.disable", { defaultValue: "Disable" })}
												</Button>
											</ButtonGroup>
										</div>
									)}
								</div>
							</CardHeader>
							<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-4 pt-2">
								<CapabilityList
									asCard={false}
									scrollContainedBody
									title={t("profiles:detail.labels.resources", { defaultValue: "Resources" })}
									kind="resources"
									context="profile"
									items={visibleResources as any}
									loading={isLoadingResources}
									enableToggle
									getId={(r: any) => r.id}
									getEnabled={(r: any) => !!r.enabled}
									onToggle={(id, next) =>
										resourceToggleMutation.mutate({
											resourceId: id,
											enable: next,
										})
									}
									emptyText={t("profiles:detail.emptyStates.noResources", { defaultValue: "No resources found in this profile" })}
									filterText={resourceQuery}
									selectable
									selectedIds={selectedResourceIds}
									onSelectToggle={(id) => {
										setSelectedResourceIds((prev) =>
											prev.includes(id)
												? prev.filter((x) => x !== id)
												: [...prev, id],
										);
									}}
									renderAction={undefined}
								/>
							</CardContent>
						</Card>
					</TabsContent>

					<TabsContent value="templates" className={DETAIL_TAB_CONTENT_CLASS}>
						<Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
							<CardHeader className="shrink-0">
								<div className="flex items-center justify-between gap-2">
									<div>
										<CardTitle>
											{t("profiles:detail.labels.templates", { defaultValue: "Templates" })}
										</CardTitle>
										<CardDescription>
											{t("profiles:detail.descriptions.templates", {
												defaultValue: "Manage resource templates included in this profile",
											})}
										</CardDescription>
									</div>
									{!isLoadingTemplates && (
										<div className="flex flex-wrap items-center gap-2">
											<Input
												placeholder={t("profiles:detail.placeholders.searchTemplates", { defaultValue: "Search templates..." })}
												value={templateQuery}
												onChange={(e) => setTemplateQuery(e.target.value)}
												className="w-48 h-9"
											/>
											<div className="hidden xl:block">
												<Select
													value={templateStatus}
													onValueChange={(v) => setTemplateStatus(v as "all" | "enabled" | "disabled")}
												>
													<SelectTrigger className="w-36 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.status", { defaultValue: "Status" })} />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">{t("profiles:detail.filters.status.all", { defaultValue: "All" })}</SelectItem>
														<SelectItem value="enabled">{t("profiles:detail.filters.status.enabled", { defaultValue: "Enabled" })}</SelectItem>
														<SelectItem value="disabled">{t("profiles:detail.filters.status.disabled", { defaultValue: "Disabled" })}</SelectItem>
													</SelectContent>
												</Select>
											</div>
											<div className="hidden xl:block">
												<Select value={templateServer} onValueChange={(v) => setTemplateServer(v)}>
													<SelectTrigger className="w-40 h-9">
														<SelectValue placeholder={t("profiles:detail.placeholders.server", { defaultValue: "Server" })} />
													</SelectTrigger>
													<SelectContent>
														<SelectItem value="all">{t("profiles:detail.filters.server.all", { defaultValue: "All Servers" })}</SelectItem>
														{serverNameOptions.map((name) => (
															<SelectItem key={`tpl-sel-${name}`} value={name}>
																{name}
															</SelectItem>
														))}
													</SelectContent>
												</Select>
											</div>
											<ButtonGroup className="hidden md:flex ml-2">
												<Button variant="outline" size="sm" onClick={() => setSelectedTemplateIds(visibleTemplates.map((p) => p.id))}>
													{t("profiles:detail.buttons.selectAll", { defaultValue: "Select all" })}
												</Button>
												<Button variant="outline" size="sm" onClick={() => setSelectedTemplateIds([])}>
													{t("profiles:detail.buttons.clearSelection", { defaultValue: "Clear" })}
												</Button>
												<Button size="sm" disabled={bulkTemplatesM.isPending || selectedTemplateIds.length === 0} onClick={() => bulkTemplatesM.mutate({ enable: true })}>
													{t("profiles:detail.buttons.enable", { defaultValue: "Enable" })}
												</Button>
												<Button size="sm" variant="secondary" disabled={bulkTemplatesM.isPending || selectedTemplateIds.length === 0} onClick={() => bulkTemplatesM.mutate({ enable: false })}>
													{t("profiles:detail.buttons.disable", { defaultValue: "Disable" })}
												</Button>
											</ButtonGroup>
										</div>
									)}
								</div>
							</CardHeader>
							<CardContent className="flex min-h-0 flex-1 flex-col overflow-hidden p-4 pt-2">
								<CapabilityList
									asCard={false}
									scrollContainedBody
									title={t("profiles:detail.labels.templates", { defaultValue: "Templates" })}
									kind="templates"
									context="profile"
									items={visibleTemplates as any}
									loading={isLoadingTemplates}
									enableToggle
									getId={(p: any) => p.id}
									getEnabled={(p: any) => !!p.enabled}
									onToggle={(id, next) => templateToggleMutation.mutate({ templateId: id, enable: next })}
									emptyText={t("profiles:detail.emptyStates.noTemplates", { defaultValue: "No templates found in this profile" })}
									filterText={templateQuery}
									selectable
									selectedIds={selectedTemplateIds}
									onSelectToggle={(id) => setSelectedTemplateIds((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]))}
									renderAction={undefined}
								/>
							</CardContent>
						</Card>
					</TabsContent>
				</Tabs>
			) : (
				<Card>
					<CardContent className="p-4">
						<p className="text-center text-slate-500">
							{t("profiles:detail.emptyStates.profileNotFound", {
								defaultValue: "Profile not found",
							})}
						</p>
					</CardContent>
				</Card>
			)}

			{/* Edit Suit Drawer */}
			<ProfileFormDrawer
				open={isEditDialogOpen}
				onOpenChange={handleEditDrawerClose}
				mode="edit"
				suit={suit}
				restrictProfileType={isHostApp ? "host_app" : undefined}
				onSuccess={() => {
					handleEditDrawerClose(false);
					handleRefreshAll();
				}}
			/>

			{/* Delete Confirmation Dialog */}
			<AlertDialog
				open={isDeleteDialogOpen}
				onOpenChange={setIsDeleteDialogOpen}
			>
				<AlertDialogContent>
					<AlertDialogHeader>
						<AlertDialogTitle>
							{t("profiles:detail.dialogs.deleteTitle", {
								defaultValue: "Delete Configuration Profile",
							})}
						</AlertDialogTitle>
						<AlertDialogDescription>
							{t("profiles:detail.dialogs.deleteDescription", {
								defaultValue:
									'Are you sure you want to delete "{{name}}"? This action cannot be undone. All associated configurations will be permanently removed.',
								name: suit?.name ?? "",
							})}
						</AlertDialogDescription>
					</AlertDialogHeader>
					<AlertDialogFooter>
						<AlertDialogCancel>
							{t("profiles:form.buttons.cancel", { defaultValue: "Cancel" })}
						</AlertDialogCancel>
						<AlertDialogAction
							onClick={() => {
								deleteSuitMutation.mutate();
								setIsDeleteDialogOpen(false);
							}}
							className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
							disabled={deleteSuitMutation.isPending}
						>
							{deleteSuitMutation.isPending ? t("profiles:detail.buttons.deleting", { defaultValue: "Deleting..." }) : t("profiles:detail.buttons.delete", { defaultValue: "Delete" })}
						</AlertDialogAction>
					</AlertDialogFooter>
				</AlertDialogContent>
			</AlertDialog>

			<AlertDialog
				open={isGuidanceDeleteDialogOpen}
				onOpenChange={setIsGuidanceDeleteDialogOpen}
			>
				<AlertDialogContent>
					<AlertDialogHeader>
						<AlertDialogTitle>
							{t("profiles:detail.guidance.dialogs.deleteTitle", {
								defaultValue: "Delete Guidance",
							})}
						</AlertDialogTitle>
						<AlertDialogDescription>
							{t("profiles:detail.guidance.dialogs.deleteDescription", {
								defaultValue:
									'Delete "{{title}}" from this profile? The skill:// resource will no longer be exposed.',
								title: guidanceForm.title || guidanceForm.slug,
							})}
						</AlertDialogDescription>
					</AlertDialogHeader>
					<AlertDialogFooter>
						<AlertDialogCancel>
							{t("profiles:form.buttons.cancel", { defaultValue: "Cancel" })}
						</AlertDialogCancel>
						<AlertDialogAction
							onClick={() => guidanceDeleteMutation.mutate()}
							className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
							disabled={guidanceDeleteMutation.isPending}
						>
							{guidanceDeleteMutation.isPending
								? t("profiles:detail.guidance.buttons.deleting", {
									defaultValue: "Deleting...",
								})
								: t("profiles:detail.guidance.buttons.delete", {
									defaultValue: "Delete Guidance",
								})}
						</AlertDialogAction>
					</AlertDialogFooter>
				</AlertDialogContent>
			</AlertDialog>
		</div>
	);
}
