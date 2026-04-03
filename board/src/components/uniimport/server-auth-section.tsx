import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, CheckCircle2, Link2, ShieldAlert, Unplug } from "lucide-react";
import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { notifyError, notifySuccess } from "../../lib/notify";
import { serversApi } from "../../lib/api";
import type { OAuthConfigRequest, OAuthStatus } from "../../lib/types";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Segment } from "../ui/segment";
import { Label } from "../ui/label";

interface ServerAuthSectionProps {
	serverId?: string;
	isStdio: boolean;
	viewMode: "form" | "json";
	isNewServer?: boolean;
	onInitiateOAuth: (config: OAuthConfigRequest) => Promise<void>;
}

function buildDefaultRedirectUri(): string {
	if (typeof window === "undefined") {
		return "http://127.0.0.1:5173/oauth/callback";
	}
	return `${window.location.origin}/oauth/callback`;
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

export function ServerAuthSection({ serverId, isStdio, viewMode, isNewServer, onInitiateOAuth }: ServerAuthSectionProps) {
	const { t } = useTranslation("servers");
	const queryClient = useQueryClient();
	const [authMode, setAuthMode] = useState<"oauth" | "header">("header");
	const [formState, setFormState] = useState<OAuthConfigRequest>(() => toFormState(null));
	const [isDirty, setIsDirty] = useState(false);

	const authorizationEndpointId = useId();
	const tokenEndpointId = useId();
	const clientIdInputId = useId();
	const clientSecretInputId = useId();
	const scopesInputId = useId();
	const redirectUriInputId = useId();

	useEffect(() => {
		const handleMessage = async (event: MessageEvent) => {
			if (event.data?.type === "OAUTH_CALLBACK_SUCCESS") {
				if (event.data.serverId === serverId) {
					await queryClient.invalidateQueries({ queryKey: ["server-oauth", serverId] });
					notifySuccess(
						t("manual.auth.oauth.connectedTitle", { defaultValue: "OAuth connected" }),
						t("manual.auth.oauth.connectedMessage", { defaultValue: "Successfully authorized." })
					);
				}
			} else if (event.data?.type === "OAUTH_CALLBACK_ERROR") {
				notifyError(
					t("manual.auth.oauth.connectFailedTitle", { defaultValue: "Unable to start OAuth" }),
					event.data.error || "Unknown error"
				);
			}
		};
		window.addEventListener("message", handleMessage);
		return () => window.removeEventListener("message", handleMessage);
	}, [serverId, queryClient, t]);

	const oauthStatusQ = useQuery({
		queryKey: ["server-oauth", serverId],
		queryFn: () => serversApi.getOAuthStatus(serverId!),
		enabled: !isStdio && !!serverId,
		retry: false,
	});

	useEffect(() => {
		if (oauthStatusQ.data?.configured && !isDirty) {
			setAuthMode("oauth");
		}
	}, [oauthStatusQ.data?.configured, isDirty]);

	useEffect(() => {
		if (!isDirty) {
			setFormState(toFormState(oauthStatusQ.data ?? null));
		}
	}, [isDirty, oauthStatusQ.data, serverId]);

	const saveMutation = useMutation({
		mutationFn: async () => {
			if (!serverId) throw new Error("Server not saved yet");
			return serversApi.saveOAuthConfig(serverId, formState);
		},
		onSuccess: async () => {
			setIsDirty(false);
			await queryClient.invalidateQueries({ queryKey: ["server-oauth", serverId] });
			notifySuccess(
				t("manual.auth.oauth.savedTitle", { defaultValue: "OAuth settings saved" }),
				t("manual.auth.oauth.savedMessage", { defaultValue: "The server OAuth configuration has been updated." })
			);
		},
		onError: (error) => {
			notifyError(
				t("manual.auth.oauth.saveFailedTitle", { defaultValue: "Failed to save OAuth settings" }),
				error instanceof Error ? error.message : String(error)
			);
		},
	});

	const connectMutation = useMutation({
		mutationFn: async () => onInitiateOAuth(formState),
		onSuccess: () => {},
		onError: (error) => {
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

	if (viewMode !== "form" || isStdio) {
		return null;
	}

	const isBusy = saveMutation.isPending || connectMutation.isPending || revokeMutation.isPending;

	return (
		<div className="space-y-4 pt-2 border-t mt-4">
			<div className="flex items-center gap-4">
				<Label className="w-20 text-right">
					{t("manual.auth.label", { defaultValue: "Authentication" })}
				</Label>
				<div className="flex-1 flex items-center justify-between">
					<Segment
						options={[
							{ label: t("manual.auth.mode.header", { defaultValue: "Header-based auth" }), value: "header" },
							{ label: t("manual.auth.mode.oauth", { defaultValue: "OAuth" }), value: "oauth" },
						]}
						value={authMode}
						onValueChange={(val) => setAuthMode(val as "header" | "oauth")}
					/>
					{authMode === "oauth" && (
						<Badge variant={statusVariant(status.state)}>{stateLabel}</Badge>
					)}
				</div>
			</div>

			{authMode === "oauth" && (
				<div className="ml-24 space-y-4 rounded-md border p-4 bg-slate-50/50 dark:bg-slate-900/20">
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

					<div className="flex flex-wrap items-center gap-2 pt-2">
						{serverId && !isNewServer && (
							<Button
								type="button"
								variant="outline"
								size="sm"
								onClick={() => saveMutation.mutate()}
								disabled={isBusy}
							>
								<ShieldAlert className="mr-2 h-3 w-3" />
								{t("manual.auth.oauth.actions.save", { defaultValue: "Save settings" })}
							</Button>
						)}
						<Button 
							type="button"
							size="sm" 
							onClick={() => connectMutation.mutate()} 
							disabled={isBusy}
						>
							<Link2 className="mr-2 h-3 w-3" />
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
					</div>

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
