import { CheckCircle2, Loader2, XCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { serversApi } from "../../lib/api";
import { PageLayout } from "../../components/page-layout";
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

		const callbackCode = code;
		const callbackState = state;

		async function processCallback() {
			try {
				const oauthStatus = await serversApi.handleOAuthCallback({
					code: callbackCode,
					state: callbackState,
				});
				setServerTarget(oauthStatus.server_id);
				setStatus("success");
				
				if (window.opener) {
					window.opener.postMessage({ type: "OAUTH_CALLBACK_SUCCESS", serverId: oauthStatus.server_id }, "*");
					setTimeout(() => window.close(), 1200);
					return;
				}

				redirectTimer = setTimeout(() => {
					navigate(`/servers/${encodeURIComponent(oauthStatus.server_id)}`, {
						replace: true,
					});
				}, 1200);
			} catch (err) {
				setStatus("error");
				const errorMsg = err instanceof Error
					? err.message
					: t("oauth.errors.callbackFailed", {
							defaultValue: "OAuth callback processing failed.",
					  });
				setErrorMessage(errorMsg);

				if (window.opener) {
					window.opener.postMessage({ type: "OAUTH_CALLBACK_ERROR", error: errorMsg }, "*");
				}
			}
		}

		processCallback();

		return () => {
			if (redirectTimer) {
				clearTimeout(redirectTimer);
			}
		};
	}, [searchParams, navigate, t, i18n.language]);

	return (
		<PageLayout title={t("oauth.callback.title", { defaultValue: "OAuth Authorization" })}>
			<div className="flex items-center justify-center h-[60vh]">
				<Card className="w-full max-w-md">
					<CardHeader className="text-center">
						<CardTitle>{t("oauth.callback.title", { defaultValue: "OAuth Authorization" })}</CardTitle>
						<CardDescription>
							{status === "processing" &&
								t("oauth.callback.processing", {
									defaultValue: "Completing authorization, please wait...",
								})}
							{status === "success" &&
								t("oauth.callback.success", {
									defaultValue:
										"Authorization successful. Returning to the server detail page...",
								})}
							{status === "error" &&
								t("oauth.callback.error", {
									defaultValue: "Authorization failed.",
								})}
						</CardDescription>
					</CardHeader>
					<CardContent className="flex flex-col items-center justify-center p-6 space-y-4">
						{status === "processing" && <Loader2 className="h-10 w-10 animate-spin text-slate-500" />}
						{status === "success" && (
							<CheckCircle2 className="h-12 w-12 text-emerald-500" />
						)}
						{status === "error" && (
							<div className="text-destructive flex flex-col items-center text-center gap-3">
								<XCircle className="h-12 w-12" />
								<p>{errorMessage}</p>
								<Button variant="outline" onClick={() => navigate(serverTarget ? `/servers/${encodeURIComponent(serverTarget)}` : "/servers") }>
									{t("oauth.callback.back", {
										defaultValue: "Back to servers",
									})}
								</Button>
							</div>
						)}
					</CardContent>
				</Card>
			</div>
		</PageLayout>
	);
}
