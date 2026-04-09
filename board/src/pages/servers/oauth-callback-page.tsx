import { CheckCircle2, Loader2, XCircle } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { serversApi } from "../../lib/api";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "../../components/ui/card";
import { Button } from "../../components/ui/button";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";

export function OAuthCallbackPage() {
	const { t, i18n } = useTranslation("servers");
	usePageTranslations("servers");

	const [searchParams] = useSearchParams();
	const navigate = useNavigate();
	const [status, setStatus] = useState<"processing" | "success" | "error">("processing");
	const [errorMessage, setErrorMessage] = useState("");
	const [serverTarget, setServerTarget] = useState<string | null>(null);
	const hasProcessedRef = useRef(false);

	useEffect(() => {
		let redirectTimer: ReturnType<typeof setTimeout> | null = null;
		const code = searchParams.get("code");
		const state = searchParams.get("state");

		if (!code || !state) {
			setStatus("error");
			setErrorMessage(
				t("oauth.errors.missingParams", {
					defaultValue: "Missing required OAuth parameters.",
				}),
			);
			return;
		}

		const callbackCode: string = code;
		const callbackState: string = state;

		const runSafely = (action: () => void) => {
			try {
				action();
			} catch (error) {
				void error;
			}
		};

		const notifyMainWindow = (payload: Record<string, unknown>) => {
			const targetOrigin = window.location.origin;

			runSafely(() => {
				window.localStorage.setItem("mcpmate.oauth.callback", JSON.stringify(payload));
			});

			runSafely(() => {
				if (window.opener) {
					window.opener.postMessage(payload, targetOrigin);
					window.opener.focus();
				}
			});

			if (typeof window !== "undefined" && "BroadcastChannel" in window) {
				runSafely(() => {
					const channel = new BroadcastChannel("mcpmate-oauth");
					channel.postMessage(payload);
					channel.close();
				});
			}
		};

		async function processCallback() {
			if (hasProcessedRef.current) {
				return;
			}
			hasProcessedRef.current = true;

			try {
				const oauthStatus = await serversApi.handleOAuthCallback({
					code: callbackCode,
					state: callbackState,
				});
				setServerTarget(oauthStatus.server_id);
				setStatus("success");

				const successPayload = {
					type: "OAUTH_CALLBACK_SUCCESS",
					serverId: oauthStatus.server_id,
					timestamp: Date.now(),
				};

				notifyMainWindow(successPayload);

				redirectTimer = setTimeout(() => {
					window.close();
				}, 300);
			} catch (err) {
				setStatus("error");
				const errorMsg = err instanceof Error
					? err.message
					: t("oauth.errors.callbackFailed", {
							defaultValue: "OAuth callback processing failed.",
					  });
				setErrorMessage(errorMsg);

				notifyMainWindow({
					type: "OAUTH_CALLBACK_ERROR",
					error: errorMsg,
					timestamp: Date.now(),
				});
			}
		}

		processCallback();

		return () => {
			if (redirectTimer) {
				clearTimeout(redirectTimer);
			}
		};
	}, [searchParams, navigate, t, i18n.language]);

	const description = (() => {
		switch (status) {
			case "processing":
				return t("oauth.callback.processing", {
					defaultValue: "Completing authorization, please wait...",
				});
			case "success":
				return t("oauth.callback.success", {
					defaultValue: "Authorization successful. This window will close automatically.",
				});
			default:
				return t("oauth.callback.error", {
					defaultValue: "Authorization failed.",
				});
		}
	})();

	return (
		<div className="min-h-screen overflow-y-auto bg-slate-50 p-4 dark:bg-slate-950">
			<div className="flex min-h-full items-center justify-center">
			<Card className="w-full max-w-md">
				<CardHeader className="text-center">
					<CardTitle>{t("oauth.callback.title", { defaultValue: "OAuth Authorization" })}</CardTitle>
					<CardDescription>{description}</CardDescription>
				</CardHeader>
				<CardContent className="flex flex-col items-center justify-center p-6 space-y-4">
					{status === "processing" && <Loader2 className="h-10 w-10 animate-spin text-slate-500" />}
					{status === "success" && (
						<>
							<CheckCircle2 className="h-12 w-12 text-emerald-500" />
							<Button variant="outline" onClick={() => window.close()}>
								{t("oauth.callback.close", {
									defaultValue: "Close this window",
								})}
							</Button>
						</>
					)}
					{status === "error" && (
						<div className="text-destructive flex flex-col items-center text-center gap-3">
							<XCircle className="h-12 w-12" />
							<p>{errorMessage}</p>
							<Button variant="outline" onClick={() => navigate(serverTarget ? `/servers/${encodeURIComponent(serverTarget)}` : "/servers")}>
								{t("oauth.callback.back", {
									defaultValue: "Back to servers",
								})}
							</Button>
						</div>
					)}
				</CardContent>
			</Card>
			</div>
		</div>
	);
}
