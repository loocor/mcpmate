import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, CheckCircle2, ChevronDown, Link2, Loader2, ShieldCheck, Unplug } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { notifyError, notifySuccess } from "../../lib/notify";
import {
	bindDesktopOAuthCallback,
	getOAuthRedirectUriForForm,
} from "../../lib/oauth-callback-access";
import { serversApi } from "../../lib/api";
import { useSecretStoreStatusQuery } from "../../lib/hooks/use-secret-store-status";
import { resolveOAuthReadiness } from "../../lib/oauth-readiness";
import { isTauriEnvironmentSync } from "../../lib/platform";
import { cn } from "../../lib/utils";
import type {
	OAuthCallbackNotificationPayload,
	OAuthConfigRequest,
	OAuthStatus,
} from "../../lib/types";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Segment } from "../ui/segment";
import { Label } from "../ui/label";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../ui/tooltip";
import { SERVER_INSTALL_FORM_ROW_LABEL_CLASS } from "./field-list";

type OAuthProgressState = "idle" | "preparing" | "awaiting_callback" | "connected" | "error";

interface ServerAuthSectionProps {
	serverId?: string;
	isStdio: boolean;
	viewMode: "form" | "json";
	isNewServer?: boolean;
	suggestedAuthMode?: "header" | "oauth";
	onInitiateOAuth: (config: OAuthConfigRequest) => Promise<void>;
	onAuthModeChange?: (mode: "header" | "oauth") => void;
	onOAuthConnected?: (serverId: string) => void;
	className?: string;
	segmentListClassName?: string;
	segmentTriggerClassName?: string;
	segmentDotClassName?: string;
	oauthPanelClassName?: string;
	labelClassName?: string;
	panelOffsetClassName?: string;
}

function toFormState(status: OAuthStatus | null): OAuthConfigRequest {
	return {
		authorization_endpoint: status?.authorization_endpoint ?? "",
		token_endpoint: status?.token_endpoint ?? "",
		client_id: status?.client_id ?? "",
		client_secret: "",
		scopes: status?.scopes ?? "",
		redirect_uri: getOAuthRedirectUriForForm(status?.redirect_uri),
	};
}

function oauthStateTextClass(state: OAuthStatus["state"]): string {
	switch (state) {
		case "connected":
			return "text-emerald-700 dark:text-emerald-300";
		case "expired":
			return "text-red-600 dark:text-red-400";
		default:
			return "text-slate-600 dark:text-slate-300";
	}
}

export function ServerAuthSection({
	serverId,
	isStdio,
	viewMode,
	isNewServer,
	suggestedAuthMode,
	onInitiateOAuth,
	onAuthModeChange,
	onOAuthConnected,
	className,
	segmentListClassName,
	segmentTriggerClassName,
	segmentDotClassName,
	oauthPanelClassName,
	labelClassName,
	panelOffsetClassName,
}: ServerAuthSectionProps) {
	const { t, i18n } = useTranslation("servers");
	const queryClient = useQueryClient();
	const [authMode, setAuthMode] = useState<"oauth" | "header">("header");
	const [formState, setFormState] = useState<OAuthConfigRequest>(() => toFormState(null));
	const [isDirty, setIsDirty] = useState(false);
	const [showAdvanced, setShowAdvanced] = useState(false);
	const [progressState, setProgressState] = useState<OAuthProgressState>("idle");
	const [desktopListenerReady, setDesktopListenerReady] = useState(() => !isTauriEnvironmentSync());
	const completedServerRef = useRef<string | null>(null);
	const serverIdRef = useRef<string | undefined>(serverId);
	const onOAuthConnectedRef = useRef(onOAuthConnected);
	const translateRef = useRef(t);
	const isDesktopEnvironment = isTauriEnvironmentSync();

	serverIdRef.current = serverId;
	onOAuthConnectedRef.current = onOAuthConnected;
	translateRef.current = t;

	const authorizationEndpointId = useId();
	const tokenEndpointId = useId();
	const clientIdInputId = useId();
	const clientSecretInputId = useId();
	const scopesInputId = useId();
	const redirectUriInputId = useId();

	useEffect(() => {
		onAuthModeChange?.(authMode);
	}, [authMode, onAuthModeChange]);

	useEffect(() => {
		completedServerRef.current = null;
	}, [serverId]);

	useEffect(() => {
		setDesktopListenerReady(!isDesktopEnvironment);

		const completeOAuthConnection = async (completedServerId?: string) => {
			const activeServerId = serverIdRef.current;

			if (!completedServerId) {
				return;
			}

			if (activeServerId && completedServerId !== activeServerId) {
				return;
			}

			if (completedServerRef.current === completedServerId) {
				return;
			}

			completedServerRef.current = completedServerId;

			setProgressState("connected");
			onOAuthConnectedRef.current?.(completedServerId);
			notifySuccess(
				translateRef.current("manual.auth.oauth.connectedTitle", {
					defaultValue: "OAuth connected",
				}),
				translateRef.current("manual.auth.oauth.connectedMessage", {
					defaultValue: "Successfully authorized.",
				}),
			);
		};

		const handleError = (errorMessage?: string) => {
			setProgressState("error");
			notifyError(
				translateRef.current("manual.auth.oauth.connectFailedTitle", {
					defaultValue: "Unable to start OAuth",
				}),
				errorMessage ||
				translateRef.current("manual.auth.oauth.unknownError", {
					defaultValue: "Unknown error",
				}),
			);
		};

		const handleCallbackPayload = async (payload: OAuthCallbackNotificationPayload) => {
			const activeServerId = serverIdRef.current;
			const callbackServerId = payload.serverId ?? activeServerId;

			if (payload.type === "OAUTH_CALLBACK_SUCCESS") {
				if (!callbackServerId) {
					return;
				}

				await queryClient.refetchQueries({ queryKey: ["server-oauth", callbackServerId] });
				await completeOAuthConnection(callbackServerId);
				return;
			}

			if (payload.type === "OAUTH_CALLBACK_ERROR") {
				if (payload.serverId && activeServerId && payload.serverId !== activeServerId) {
					return;
				}
				handleError(payload.error);
			}
		};

		const handleMessage = async (event: MessageEvent<OAuthCallbackNotificationPayload>) => {
			if (event.origin !== window.location.origin) {
				return;
			}

			await handleCallbackPayload(event.data ?? {});
		};

		let desktopCallbackCleanup: (() => void) | undefined;
		let desktopBindingCancelled = false;
		const bindDesktopListener = async () => {
			try {
				desktopCallbackCleanup = await bindDesktopOAuthCallback(handleCallbackPayload);
				if (!desktopBindingCancelled) {
					setDesktopListenerReady(true);
				}
			} catch (error) {
				if (!desktopBindingCancelled) {
					setDesktopListenerReady(false);
				}
				if (!desktopBindingCancelled && import.meta.env.DEV) {
					console.warn("[ServerAuthSection] desktop oauth bind failed", error);
				}
			}
		};
		void bindDesktopListener();

		const oauthChannel =
			typeof window !== "undefined" && "BroadcastChannel" in window
				? new BroadcastChannel("mcpmate-oauth")
				: null;

		const handleChannelMessage = async (event: MessageEvent<OAuthCallbackNotificationPayload>) => {
			await handleCallbackPayload(event.data ?? {});
		};

		const handleStorage = async (event: StorageEvent) => {
			if (event.key !== "mcpmate.oauth.callback" || !event.newValue) {
				return;
			}

			try {
				const payload = JSON.parse(event.newValue) as OAuthCallbackNotificationPayload;
				await handleCallbackPayload(payload);
			} catch (error) {
				void error;
			}
		};

		window.addEventListener("message", handleMessage);
		oauthChannel?.addEventListener("message", handleChannelMessage);
		window.addEventListener("storage", handleStorage);

		return () => {
			desktopBindingCancelled = true;
			desktopCallbackCleanup?.();
			window.removeEventListener("message", handleMessage);
			oauthChannel?.removeEventListener("message", handleChannelMessage);
			oauthChannel?.close();
			window.removeEventListener("storage", handleStorage);
		};
	}, [isDesktopEnvironment, queryClient]);

	const oauthStatusQ = useQuery({
		queryKey: ["server-oauth", serverId],
		queryFn: () => serversApi.getOAuthStatus(serverId!),
		enabled: !isStdio && !!serverId,
		refetchInterval:
			!isStdio && !!serverId && progressState === "awaiting_callback" ? 1500 : false,
		refetchIntervalInBackground: progressState === "awaiting_callback",
		refetchOnWindowFocus: progressState === "awaiting_callback",
		retry: false,
	});
	const secretStoreStatusQ = useSecretStoreStatusQuery({
		enabled: !isStdio,
		retry: false,
	});

	useEffect(() => {
		if (!serverId || progressState !== "awaiting_callback") {
			return;
		}

		const refetchOAuthStatus = () => {
			void queryClient.refetchQueries({ queryKey: ["server-oauth", serverId] });
		};

		const handleVisibilityChange = () => {
			if (document.visibilityState === "visible") {
				refetchOAuthStatus();
			}
		};

		window.addEventListener("focus", refetchOAuthStatus);
		document.addEventListener("visibilitychange", handleVisibilityChange);

		return () => {
			window.removeEventListener("focus", refetchOAuthStatus);
			document.removeEventListener("visibilitychange", handleVisibilityChange);
		};
	}, [progressState, queryClient, serverId]);

	useEffect(() => {
		if (oauthStatusQ.data?.configured && !isDirty) {
			setAuthMode("oauth");
			return;
		}
		if (!isDirty && suggestedAuthMode) {
			setAuthMode(suggestedAuthMode);
		}
	}, [oauthStatusQ.data?.configured, isDirty, suggestedAuthMode]);

	useEffect(() => {
		if (oauthStatusQ.data?.state === "connected") {
			setProgressState("connected");
			return;
		}
		if (oauthStatusQ.data?.state === "expired") {
			setProgressState("error");
			return;
		}
		if (
			oauthStatusQ.data?.state === "disconnected" ||
			oauthStatusQ.data?.state === "not_configured"
		) {
			setProgressState((current) =>
				current === "preparing" || current === "awaiting_callback" ? current : "idle",
			);
		}
	}, [oauthStatusQ.data?.state]);

	useEffect(() => {
		if (
			!serverId ||
			progressState !== "awaiting_callback" ||
			oauthStatusQ.data?.state !== "connected" ||
			completedServerRef.current === serverId
		) {
			return;
		}

		completedServerRef.current = serverId;
		setProgressState("connected");
		onOAuthConnected?.(serverId);
		notifySuccess(
			t("manual.auth.oauth.connectedTitle", { defaultValue: "OAuth connected" }),
			t("manual.auth.oauth.connectedMessage", { defaultValue: "Successfully authorized." }),
		);
	}, [oauthStatusQ.data?.state, onOAuthConnected, progressState, serverId, t, i18n.language]);

	useEffect(() => {
		if (!isDirty) {
			setFormState(toFormState(oauthStatusQ.data ?? null));
		}
	}, [isDirty, oauthStatusQ.data, serverId]);

	const connectMutation = useMutation({
		mutationFn: async () => {
			if (isDesktopEnvironment && !desktopListenerReady) {
				throw new Error(
					t("manual.auth.oauth.listenerNotReady", {
						defaultValue: "OAuth callback listener is still initializing. Please try again in a moment.",
					}),
				);
			}

			setProgressState("preparing");
			completedServerRef.current = null;
			const payload = showAdvanced
				? formState
				: {
					authorization_endpoint: "",
					token_endpoint: "",
					client_id: "",
					client_secret: formState.client_secret,
					scopes: formState.scopes,
					redirect_uri: formState.redirect_uri,
				};
			await onInitiateOAuth(payload);
		},
		onSuccess: () => {
			setProgressState("awaiting_callback");
		},
		onError: (error) => {
			setProgressState("error");
			notifyError(
				t("manual.auth.oauth.connectFailedTitle", { defaultValue: "Unable to start OAuth" }),
				error instanceof Error ? error.message : String(error)
			);
		},
	});

	const revokeMutation = useMutation({
		mutationFn: async () => {
			if (!serverId) return;
			return serversApi.revokeOAuth(serverId);
		},
		onSuccess: async () => {
			completedServerRef.current = null;
			setProgressState("idle");
			await queryClient.invalidateQueries({ queryKey: ["server-oauth", serverId] });
			notifySuccess(
				t("manual.auth.oauth.revokedTitle", { defaultValue: "OAuth token revoked" }),
				t("manual.auth.oauth.revokedMessage", { defaultValue: "Stored OAuth credentials were removed for this server." })
			);
		},
		onError: (error) => {
			notifyError(
				t("manual.auth.oauth.revokeFailedTitle", { defaultValue: "Failed to revoke OAuth" }),
				error instanceof Error ? error.message : String(error)
			);
		},
	});

	const status = oauthStatusQ.data ?? {
		server_id: serverId ?? "",
		configured: false,
		state: "not_configured",
	} satisfies OAuthStatus;
	const oauthReadiness = resolveOAuthReadiness({
		secretStoreStatus: secretStoreStatusQ.data,
		oauthStatus: status,
	});

	const stateLabel = (() => {
		switch (status.state) {
			case "connected":
				return t("manual.auth.oauth.state.connected", { defaultValue: "Connected" });
			case "expired":
				return t("manual.auth.oauth.state.expired", { defaultValue: "Expired" });
			case "disconnected":
				return t("manual.auth.oauth.state.disconnected", { defaultValue: "Disconnected" });
			default:
				return t("manual.auth.oauth.state.notConfigured", { defaultValue: "Not configured" });
		}
	})();
	const showManualOverride = Boolean(status.manual_authorization_override);
	const isSecureOAuthCustody =
		status.state === "connected" && status.custody_state === "secure";
	const manualOverrideTitle = t("manual.auth.oauth.manualOverride.title", {
		defaultValue: "Manual Authorization header is active",
	});
	const manualOverrideDescription = t("manual.auth.oauth.manualOverride.description", {
		defaultValue:
			"This server already has an Authorization header in its transport settings, which will take precedence over the stored OAuth token.",
	});
	const secureOAuthCustodyLabel = t("manual.auth.oauth.secureStoreStored", {
		defaultValue: "OAuth credentials are stored in Secure Store",
	});
	const oauthStatusLabel = (
		<span
			className={`inline-flex items-center gap-1.5 text-xs font-semibold ${oauthStateTextClass(status.state)}`}
		>
			{isSecureOAuthCustody ? (
				<ShieldCheck className="h-3.5 w-3.5" aria-hidden="true" />
			) : null}
			{stateLabel}
		</span>
	);
	const oauthStatusBadge =
		showManualOverride || isSecureOAuthCustody ? (
			<TooltipProvider delayDuration={200}>
				<Tooltip>
					<TooltipTrigger asChild>
						<span className="inline-flex cursor-help items-center gap-1.5">
							{showManualOverride ? (
								<AlertTriangle
									className="h-3.5 w-3.5 flex-none text-amber-600 dark:text-amber-400"
									aria-hidden="true"
								/>
							) : null}
							{oauthStatusLabel}
						</span>
					</TooltipTrigger>
					<TooltipContent side="top">
						<div className="max-w-xs space-y-1">
							{showManualOverride ? (
								<>
									<p className="font-medium">{manualOverrideTitle}</p>
									<p className="text-xs">{manualOverrideDescription}</p>
								</>
							) : null}
							{isSecureOAuthCustody ? (
								<p className={showManualOverride ? "text-xs" : undefined}>
									{secureOAuthCustodyLabel}
								</p>
							) : null}
						</div>
					</TooltipContent>
				</Tooltip>
			</TooltipProvider>
		) : (
			oauthStatusLabel
		);

	const progressItems = [
		{
			key: "discover",
			label: t("manual.auth.oauth.progress.discover", {
				defaultValue: "Metadata discovery",
			}),
			active:
				progressState === "preparing" ||
				progressState === "awaiting_callback" ||
				progressState === "connected",
		},
		{
			key: "register",
			label: t("manual.auth.oauth.progress.register", {
				defaultValue: "Client registration",
			}),
			active: progressState === "awaiting_callback" || progressState === "connected",
		},
		{
			key: "authorize",
			label: t("manual.auth.oauth.progress.authorize", {
				defaultValue: "Authorization",
			}),
			active: progressState === "awaiting_callback" || progressState === "connected",
		},
		{
			key: "complete",
			label: t("manual.auth.oauth.progress.complete", {
				defaultValue: "Authentication complete",
			}),
			active: progressState === "connected",
		},
	];

	const progressMessage = (() => {
		if (connectMutation.isPending) {
			return t("manual.auth.oauth.progress.preparingMessage", {
				defaultValue: "Preparing OAuth flow and opening the authorization page…",
			});
		}
		if (isDesktopEnvironment && !desktopListenerReady) {
			return t("manual.auth.oauth.progress.listenerPreparingMessage", {
				defaultValue: "Preparing the desktop callback listener…",
			});
		}
		if (progressState === "awaiting_callback") {
			return t("manual.auth.oauth.progress.awaitingMessage", {
				defaultValue: "Waiting for authorization to complete in the popup window…",
			});
		}
		if (progressState === "connected") {
			return t("manual.auth.oauth.progress.connectedMessage", {
				defaultValue: "OAuth is connected for this server.",
			});
		}
		if (progressState === "error" || status.state === "expired") {
			return t("manual.auth.oauth.progress.errorMessage", {
				defaultValue: "OAuth needs attention. Try reconnecting to refresh the authorization.",
			});
		}
		return "";
	})();

	if (viewMode !== "form" || isStdio) {
		return null;
	}

	const isBusy = connectMutation.isPending || revokeMutation.isPending;
	const isConnectDisabled = isBusy || oauthReadiness.actionDisabled || (isDesktopEnvironment && !desktopListenerReady);
	// Revoke is allowed even when the secure store is unavailable — the backend
	// can delete plaintext OAuth tokens without the store and will return an
	// error if the token actually requires store access.
	const isRevokeDisabled = isBusy;

	return (
		<div className={cn("space-y-4", className)}>
			<div className="flex items-center gap-3">
				<Label className={cn(SERVER_INSTALL_FORM_ROW_LABEL_CLASS, labelClassName)}>
					{t("manual.auth.label", { defaultValue: "AUTH" })}
				</Label>
				<div className="flex-1 min-w-0">
					<Segment
						options={[
							{ label: t("manual.auth.mode.header", { defaultValue: "Header-based" }), value: "header" },
							{ label: t("manual.auth.mode.oauth", { defaultValue: "OAuth" }), value: "oauth" },
						]}
						value={authMode}
						onValueChange={(val) => setAuthMode(val as "header" | "oauth")}
						listClassName={segmentListClassName}
						triggerClassName={segmentTriggerClassName}
						dotClassName={segmentDotClassName}
					/>
				</div>
			</div>

			{authMode === "oauth" && (
				<div
					className={cn(
						"ml-24 space-y-4 rounded-md border p-4 bg-slate-50/50 dark:bg-slate-900/20",
						panelOffsetClassName,
						oauthPanelClassName,
					)}
				>
					<div className="flex flex-wrap items-center justify-between gap-3">
						{progressMessage ? (
							<p className="text-xs text-slate-500 dark:text-slate-400">
								{progressMessage}
							</p>
						) : (
							<span className="text-xs text-slate-500 dark:text-slate-400">
								{t("manual.auth.oauth.statusLabel", { defaultValue: "OAuth status" })}
							</span>
						)}
						<div className="flex items-center gap-2">
							{oauthStatusBadge}
							<Button
								type="button"
								variant="outline"
								size="icon"
								className="h-7 w-7"
								onClick={() => setShowAdvanced((value) => !value)}
								aria-label={t("manual.auth.oauth.actions.configure", { defaultValue: "Configure" })}
								title={t("manual.auth.oauth.actions.configure", { defaultValue: "Configure" })}
							>
								<ChevronDown className={`h-3.5 w-3.5 transition-transform ${showAdvanced ? "rotate-180" : "rotate-0"}`} />
							</Button>
						</div>
					</div>

					{oauthReadiness.notice ? (
						<div className="rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-900 dark:text-amber-100">
							<div className="flex items-start gap-2">
								<AlertTriangle className="mt-0.5 h-4 w-4 flex-none" />
								<div>
									<div className="font-medium">
										{oauthReadiness.notice.kind === "secure-store-unavailable"
											? t("manual.auth.oauth.secureStoreUnavailable.title", { defaultValue: "Secure Store needs attention" })
											: t("manual.auth.oauth.legacyReconnect.title", { defaultValue: "Reconnect OAuth to secure credentials" })}
									</div>
									<div>{t(oauthReadiness.notice.messageKey, { defaultValue: oauthReadiness.notice.defaultMessage })}</div>
								</div>
							</div>
						</div>
					) : null}

					<div className="rounded-md border border-dashed border-slate-200 bg-white/70 p-3 dark:border-slate-700 dark:bg-slate-950/20">
						<div className="grid gap-2 md:grid-cols-2">
							{progressItems.map((item) => (
								<div key={item.key} className="flex items-center gap-2 text-sm">
									<span
										className={`h-2.5 w-2.5 rounded-full ${item.active ? "bg-emerald-500" : "bg-slate-300 dark:bg-slate-700"}`}
									/>
									<span className={item.active ? "text-slate-900 dark:text-slate-100" : "text-slate-500 dark:text-slate-400"}>
										{item.label}
									</span>
								</div>
							))}
						</div>
					</div>

					<div className="flex flex-wrap items-center gap-2 pt-1">
						<Button
							type="button"
							size="sm"
							onClick={() => connectMutation.mutate()}
							disabled={isConnectDisabled}
						>
							{connectMutation.isPending ? <Loader2 className="mr-2 h-3 w-3 animate-spin" /> : <Link2 className="mr-2 h-3 w-3" />}
							{status.state === "connected"
								? t("manual.auth.oauth.actions.reconnect", { defaultValue: "Reconnect OAuth" })
								: t("manual.auth.oauth.actions.connect", { defaultValue: "Connect with OAuth" })
							}
						</Button>
						{serverId && !isNewServer && status.configured && (
							<Button
								type="button"
								variant="outline"
								size="sm"
								onClick={() => revokeMutation.mutate()}
								disabled={isRevokeDisabled}
							>
								<Unplug className="mr-2 h-3 w-3" />
								{t("manual.auth.oauth.actions.revoke", { defaultValue: "Revoke token" })}
							</Button>
						)}
					</div>

					{showAdvanced ? (
						<>
							<div className="border-t border-border" />
							<div className="grid gap-3 md:grid-cols-2">
								<div className="space-y-1.5">
									<label className="text-xs font-medium" htmlFor={authorizationEndpointId}>
										{t("manual.auth.oauth.fields.authorizationEndpoint", { defaultValue: "Authorization endpoint" })}
									</label>
									<Input
										id={authorizationEndpointId}
										value={formState.authorization_endpoint}
										onChange={(e) => {
											setIsDirty(true);
											setFormState((cur) => ({ ...cur, authorization_endpoint: e.target.value }));
										}}
										placeholder={t("manual.auth.oauth.fields.placeholderAuthorizationEndpoint", {
											defaultValue: "https://issuer.example.com/authorize",
										})}
										className="h-8 text-sm"
									/>
								</div>
								<div className="space-y-1.5">
									<label className="text-xs font-medium" htmlFor={tokenEndpointId}>
										{t("manual.auth.oauth.fields.tokenEndpoint", { defaultValue: "Token endpoint" })}
									</label>
									<Input
										id={tokenEndpointId}
										value={formState.token_endpoint}
										onChange={(e) => {
											setIsDirty(true);
											setFormState((cur) => ({ ...cur, token_endpoint: e.target.value }));
										}}
										placeholder={t("manual.auth.oauth.fields.placeholderTokenEndpoint", {
											defaultValue: "https://issuer.example.com/token",
										})}
										className="h-8 text-sm"
									/>
								</div>
								<div className="space-y-1.5">
									<label className="text-xs font-medium" htmlFor={clientIdInputId}>
										{t("manual.auth.oauth.fields.clientId", { defaultValue: "Client ID" })}
									</label>
									<Input
										id={clientIdInputId}
										value={formState.client_id}
										onChange={(e) => {
											setIsDirty(true);
											setFormState((cur) => ({ ...cur, client_id: e.target.value }));
										}}
										className="h-8 text-sm"
									/>
								</div>
								<div className="space-y-1.5">
									<label className="text-xs font-medium" htmlFor={clientSecretInputId}>
										{t("manual.auth.oauth.fields.clientSecret", { defaultValue: "Client secret" })}
									</label>
									<Input
										id={clientSecretInputId}
										type="password"
										value={formState.client_secret ?? ""}
										onChange={(e) => {
											setIsDirty(true);
											setFormState((cur) => ({ ...cur, client_secret: e.target.value }));
										}}
										placeholder={status.has_client_secret
											? t("manual.auth.oauth.fields.clientSecretPlaceholderExisting", { defaultValue: "Leave blank to keep the stored secret" })
											: t("manual.auth.oauth.fields.clientSecretPlaceholderNew", { defaultValue: "Optional for public clients" })
										}
										className="h-8 text-sm"
									/>
								</div>
								<div className="space-y-1.5 md:col-span-2">
									<label className="text-xs font-medium" htmlFor={scopesInputId}>
										{t("manual.auth.oauth.fields.scopes", { defaultValue: "Scopes" })}
									</label>
									<Input
										id={scopesInputId}
										value={formState.scopes ?? ""}
										onChange={(e) => {
											setIsDirty(true);
											setFormState((cur) => ({ ...cur, scopes: e.target.value }));
										}}
										placeholder={t("manual.auth.oauth.fields.placeholderScopes", {
											defaultValue: "read write",
										})}
										className="h-8 text-sm"
									/>
								</div>
								<div className="space-y-1.5 md:col-span-2">
									<label className="text-xs font-medium" htmlFor={redirectUriInputId}>
										{t("manual.auth.oauth.fields.redirectUri", { defaultValue: "Redirect URI" })}
									</label>
									<Input
										id={redirectUriInputId}
										value={formState.redirect_uri}
										onChange={(e) => {
											setIsDirty(true);
											setFormState((cur) => ({ ...cur, redirect_uri: e.target.value }));
										}}
										className="h-8 text-sm"
									/>
								</div>
							</div>
						</>
					) : null}

					{oauthStatusQ.isFetching ? (
						<div className="flex items-center gap-2 text-xs text-slate-500 pt-1">
							<CheckCircle2 className="h-3 w-3 opacity-0" />
							{t("manual.auth.oauth.loading", { defaultValue: "Refreshing OAuth status…" })}
						</div>
					) : null}
				</div>
			)}
		</div>
	);
}
