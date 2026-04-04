import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, CheckCircle2, ChevronDown, Link2, Loader2, Unplug } from "lucide-react";
import { useEffect, useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { notifyError, notifySuccess } from "../../lib/notify";
import { serversApi } from "../../lib/api";
import type { OAuthConfigRequest, OAuthStatus } from "../../lib/types";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Segment } from "../ui/segment";
import { Label } from "../ui/label";

type OAuthProgressState = "idle" | "preparing" | "awaiting_callback" | "connected" | "error";

interface OAuthCallbackPayload {
	type?: string;
	serverId?: string;
	error?: string;
}

interface ServerAuthSectionProps {
	serverId?: string;
	isStdio: boolean;
	viewMode: "form" | "json";
	isNewServer?: boolean;
	suggestedAuthMode?: "header" | "oauth";
	onInitiateOAuth: (config: OAuthConfigRequest) => Promise<void>;
	onAuthModeChange?: (mode: "header" | "oauth") => void;
	onOAuthConnected?: (serverId: string) => void;
}

function buildDefaultRedirectUri(): string {
	if (typeof window === "undefined") {
		return "http://127.0.0.1:5173/oauth/callback";
	}
	if (window.location.protocol === "http:" || window.location.protocol === "https:") {
		return `${window.location.origin}/oauth/callback`;
	}
	return "http://127.0.0.1:5173/oauth/callback";
}

function toFormState(status: OAuthStatus | null): OAuthConfigRequest {
	return {
		authorization_endpoint: status?.authorization_endpoint ?? "",
		token_endpoint: status?.token_endpoint ?? "",
		client_id: status?.client_id ?? "",
		client_secret: "",
		scopes: status?.scopes ?? "",
		redirect_uri: status?.redirect_uri ?? buildDefaultRedirectUri(),
	};
}

function statusVariant(state: OAuthStatus["state"]): "secondary" | "outline" | "destructive" {
	switch (state) {
		case "connected":
			return "secondary";
		case "expired":
			return "destructive";
		default:
			return "outline";
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
}: ServerAuthSectionProps) {
	const { t } = useTranslation("servers");
	const queryClient = useQueryClient();
	const [authMode, setAuthMode] = useState<"oauth" | "header">("header");
	const [formState, setFormState] = useState<OAuthConfigRequest>(() => toFormState(null));
	const [isDirty, setIsDirty] = useState(false);
	const [showAdvanced, setShowAdvanced] = useState(false);
	const [progressState, setProgressState] = useState<OAuthProgressState>("idle");

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
		const handleSuccess = async (callbackServerId?: string) => {
			if (!callbackServerId || callbackServerId !== serverId) {
				return;
			}

			setProgressState("connected");
			await queryClient.invalidateQueries({ queryKey: ["server-oauth", serverId] });
			onOAuthConnected?.(callbackServerId);
			notifySuccess(
				t("manual.auth.oauth.connectedTitle", { defaultValue: "OAuth connected" }),
				t("manual.auth.oauth.connectedMessage", { defaultValue: "Successfully authorized." }),
			);
		};

		const handleError = (errorMessage?: string) => {
			setProgressState("error");
			notifyError(
				t("manual.auth.oauth.connectFailedTitle", { defaultValue: "Unable to start OAuth" }),
				errorMessage || "Unknown error",
			);
		};

		const handleCallbackPayload = async (payload: OAuthCallbackPayload) => {
			if (payload.type === "OAUTH_CALLBACK_SUCCESS") {
				await handleSuccess(payload.serverId);
				return;
			}

			if (payload.type === "OAUTH_CALLBACK_ERROR") {
				handleError(payload.error);
			}
		};

		const handleMessage = async (event: MessageEvent<OAuthCallbackPayload>) => {
			await handleCallbackPayload(event.data ?? {});
		};

		const oauthChannel =
			typeof window !== "undefined" && "BroadcastChannel" in window
				? new BroadcastChannel("mcpmate-oauth")
				: null;

		const handleChannelMessage = async (event: MessageEvent<OAuthCallbackPayload>) => {
			await handleCallbackPayload(event.data ?? {});
		};

		const handleStorage = async (event: StorageEvent) => {
			if (event.key !== "mcpmate.oauth.callback" || !event.newValue) {
				return;
			}

			try {
				const payload = JSON.parse(event.newValue) as OAuthCallbackPayload;
				await handleCallbackPayload(payload);
			} catch (error) {
				void error;
			}
		};

		window.addEventListener("message", handleMessage);
		oauthChannel?.addEventListener("message", handleChannelMessage);
		window.addEventListener("storage", handleStorage);

		return () => {
			window.removeEventListener("message", handleMessage);
			oauthChannel?.removeEventListener("message", handleChannelMessage);
			oauthChannel?.close();
			window.removeEventListener("storage", handleStorage);
		};
	}, [serverId, queryClient, t, onOAuthConnected]);

	const oauthStatusQ = useQuery({
		queryKey: ["server-oauth", serverId],
		queryFn: () => serversApi.getOAuthStatus(serverId!),
		enabled: !isStdio && !!serverId,
		retry: false,
	});

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
		if (oauthStatusQ.data?.state === "disconnected") {
			setProgressState("idle");
		}
	}, [oauthStatusQ.data?.state]);

	useEffect(() => {
		if (!isDirty) {
			setFormState(toFormState(oauthStatusQ.data ?? null));
		}
	}, [isDirty, oauthStatusQ.data, serverId]);

	const connectMutation = useMutation({
		mutationFn: async () => {
			setProgressState("preparing");
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
		onSuccess: () => {},
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

	const progressItems = useMemo(
		() => [
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
				active:
					progressState === "awaiting_callback" || progressState === "connected",
			},
			{
				key: "authorize",
				label: t("manual.auth.oauth.progress.authorize", {
					defaultValue: "Authorization",
				}),
				active:
					progressState === "awaiting_callback" || progressState === "connected",
			},
			{
				key: "complete",
				label: t("manual.auth.oauth.progress.complete", {
					defaultValue: "Authentication complete",
				}),
				active: progressState === "connected",
			},
		],
		[progressState, t],
	);

	const progressMessage = (() => {
		if (connectMutation.isPending) {
			return t("manual.auth.oauth.progress.preparingMessage", {
				defaultValue: "Preparing OAuth flow and opening the authorization page…",
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

	return (
		<div className="space-y-4 pt-2 border-t mt-4">
			<div className="flex items-center gap-4">
				<Label className="w-20 text-right">
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
					/>
				</div>
			</div>

			{authMode === "oauth" && (
				<div className="ml-24 space-y-4 rounded-md border p-4 bg-slate-50/50 dark:bg-slate-900/20">
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
						<Badge variant={statusVariant(status.state)}>{stateLabel}</Badge>
					</div>

					{status.manual_authorization_override ? (
						<div className="rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-900 dark:text-amber-100">
							<div className="flex items-start gap-2">
								<AlertTriangle className="mt-0.5 h-4 w-4 flex-none" />
								<div>
									<div className="font-medium">
										{t("manual.auth.oauth.manualOverride.title", { defaultValue: "Manual Authorization header is active" })}
									</div>
									<div>
										{t("manual.auth.oauth.manualOverride.description", {
											defaultValue: "This server already has an Authorization header in its transport settings, which will take precedence over the stored OAuth token."
										})}
									</div>
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
							onClick={async () => {
								setProgressState("awaiting_callback");
								connectMutation.mutate();
							}} 
							disabled={isBusy}
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
								disabled={isBusy}
							>
								<Unplug className="mr-2 h-3 w-3" />
								{t("manual.auth.oauth.actions.revoke", { defaultValue: "Revoke token" })}
							</Button>
						)}
						<Button
							type="button"
							variant="outline"
							size="sm"
							onClick={() => setShowAdvanced((value) => !value)}
						>
							{t("manual.auth.oauth.actions.configure", { defaultValue: "Configure" })}
							<ChevronDown className={`ml-2 h-3 w-3 transition-transform ${showAdvanced ? "rotate-180" : "rotate-0"}`} />
						</Button>
					</div>

					{showAdvanced ? (
						<div className="rounded-md border bg-white/80 dark:bg-slate-950/10">
							<div className="grid gap-3 px-3 py-3 md:grid-cols-2">
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
								placeholder="https://issuer.example.com/authorize"
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
								placeholder="https://issuer.example.com/token"
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
								placeholder="read write"
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
						</div>
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
