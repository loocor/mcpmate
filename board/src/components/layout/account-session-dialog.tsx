import { User } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { Alert, AlertDescription, AlertTitle } from "../ui/alert";
import { Button } from "../ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogTitle,
	DialogTrigger,
} from "../ui/dialog";
import { AUTH_GITHUB_LOGIN_URL } from "../../lib/auth-config";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import {
	detectTauriEnvironment,
	isTauriEnvironmentSync,
} from "../../lib/platform";
import { cn } from "../../lib/utils";
import { websiteLangParam } from "../../lib/website-lang";

type AccountStatus = {
	loggedIn: boolean;
	deviceId: string;
	deviceName: string;
};

interface AccountSessionDialogProps {
	sidebarOpen: boolean;
}

function GitHubMark(props: React.SVGProps<SVGSVGElement>) {
	return (
		<svg
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			strokeWidth="2"
			strokeLinecap="round"
			strokeLinejoin="round"
			{...props}
		>
			<path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4" />
			<path d="M9 18c-4.51 2-5-2-7-2" />
		</svg>
	);
}

const triggerClassName = cn(
	"flex w-full items-center px-3 py-2 text-sm font-medium rounded-md transition-colors",
	"hover:bg-slate-200 dark:hover:bg-slate-800",
	"text-slate-700 dark:text-slate-400",
	"text-left",
);

export function AccountSessionDialog({ sidebarOpen }: AccountSessionDialogProps) {
	usePageTranslations("account");
	const { t, i18n } = useTranslation();
	const langParam = websiteLangParam(i18n.language);
	const termsHref = `https://mcp.umate.ai/terms?lang=${langParam}`;
	const privacyHref = `https://mcp.umate.ai/privacy?lang=${langParam}`;
	const [open, setOpen] = useState(false);
	const [tauri, setTauri] = useState(() => isTauriEnvironmentSync());
	const [status, setStatus] = useState<AccountStatus | null>(null);
	const [busy, setBusy] = useState(false);
	const [banner, setBanner] = useState<{
		variant: "default" | "destructive";
		title: string;
		description?: string;
	} | null>(null);

	const refreshStatus = useCallback(async () => {
		if (!tauri) {
			return;
		}
		try {
			const { invoke } = await import("@tauri-apps/api/core");
			const raw = (await invoke("mcp_account_get_status")) as {
				loggedIn: boolean;
				deviceId: string;
				deviceName: string;
			};
			setStatus({
				loggedIn: raw.loggedIn,
				deviceId: raw.deviceId,
				deviceName: raw.deviceName,
			});
		} catch (e) {
			console.warn("[AccountSessionDialog] mcp_account_get_status failed", e);
			setStatus(null);
		}
	}, [tauri]);

	useEffect(() => {
		if (!open) {
			return;
		}
		void refreshStatus();
	}, [open, refreshStatus]);

	useEffect(() => {
		let cancelled = false;
		void detectTauriEnvironment().then((v) => {
			if (!cancelled) {
				setTauri(v);
			}
		});
		return () => {
			cancelled = true;
		};
	}, []);

	useEffect(() => {
		if (!tauri || !open) {
			return;
		}
		let cancelled = false;
		let unlisten: (() => void) | undefined;
		const bind = async () => {
			try {
				const { listen } = await import("@tauri-apps/api/event");
				if (cancelled) {
					return;
				}
				unlisten = await listen<{ ok?: boolean; error?: string }>(
					"mcp-account/oauth-finished",
					(event) => {
						const p = event.payload;
						if (p?.ok) {
							setBanner({
								variant: "default",
								title: t("account:oauthSuccess", {
									defaultValue: "Signed in successfully.",
								}),
							});
						} else if (p?.error) {
							let description = p.error;
							if (p.error === "invalid_state") {
								description = t("account:oauthErrorInvalidState", {
									defaultValue:
										"Could not verify the sign-in after GitHub (invalid_state). Try again; the auth worker must await the KV write for OAuth state before redirecting.",
								});
							}
							setBanner({
								variant: "destructive",
								title: t("account:oauthFailed", {
									defaultValue: "Sign-in failed",
								}),
								description,
							});
						}
						void refreshStatus();
					},
				);
			} catch (err) {
				if (import.meta.env.DEV) {
					console.warn("[AccountSessionDialog] oauth event bind failed", err);
				}
			}
		};
		void bind();
		return () => {
			cancelled = true;
			if (unlisten) {
				void unlisten();
			}
		};
	}, [tauri, open, t, i18n.language, refreshStatus]);

	const onConnect = async () => {
		setBusy(true);
		try {
			const { invoke } = await import("@tauri-apps/api/core");
			await invoke("mcp_account_start_github_login");
		} catch (e) {
			const msg = e instanceof Error ? e.message : String(e);
			setBanner({
				variant: "destructive",
				title: t("account:oauthFailed", { defaultValue: "Sign-in failed" }),
				description: msg,
			});
		} finally {
			setBusy(false);
		}
	};

	const onLogout = async () => {
		setBusy(true);
		try {
			const { invoke } = await import("@tauri-apps/api/core");
			await invoke("mcp_account_logout");
			await refreshStatus();
		} catch (e) {
			const msg = e instanceof Error ? e.message : String(e);
			setBanner({
				variant: "destructive",
				title: t("account:oauthFailed", { defaultValue: "Sign-in failed" }),
				description: msg,
			});
		} finally {
			setBusy(false);
		}
	};

	const openAuthInBrowser = useCallback(() => {
		const w = window.open(
			AUTH_GITHUB_LOGIN_URL,
			"_blank",
			"noopener,noreferrer",
		);
		if (!w && import.meta.env.DEV) {
			console.warn("[AccountSessionDialog] popup blocked for auth URL");
		}
	}, []);

	const signedIn = Boolean(tauri && status?.loggedIn);
	const showSignInChrome = !signedIn;

	const onPrimaryAuth = () => {
		if (tauri) {
			void onConnect();
		} else {
			openAuthInBrowser();
		}
	};

	const linkClass =
		"font-medium text-foreground underline underline-offset-2 hover:text-foreground/90";

	return (
		<Dialog
			open={open}
			onOpenChange={(next) => {
				setOpen(next);
				if (!next) {
					setBanner(null);
				}
			}}
		>
			<DialogTrigger asChild>
				<button type="button" className={triggerClassName}>
					<span className="mr-3 flex h-5 w-5 shrink-0 items-center justify-center">
						<User size={20} aria-hidden />
					</span>
					{sidebarOpen ? (
						<span>
							{t("nav.account", { defaultValue: "Account" })}
						</span>
					) : null}
				</button>
			</DialogTrigger>
			<DialogContent
				className={cn(
					"gap-0 overflow-hidden p-0 sm:max-w-[420px]",
					"border-slate-200 bg-background shadow-xl dark:border-slate-700",
				)}
			>
				<div className="px-6 pb-6 pt-8 sm:px-8">
					{banner ? (
						<Alert variant={banner.variant} className="mb-5">
							<AlertTitle>{banner.title}</AlertTitle>
							{banner.description ? (
								<AlertDescription>{banner.description}</AlertDescription>
							) : null}
						</Alert>
					) : null}

					<div className="mb-8 flex items-center gap-2.5">
						<img
							src="https://mcp.umate.ai/logo.svg"
							alt=""
							className="h-8 w-8 object-contain dark:invert dark:brightness-0"
						/>
						<span className="text-base font-semibold tracking-tight text-foreground">
							MCPMate
						</span>
					</div>

					{showSignInChrome ? (
						<>
							<DialogTitle className="sr-only">
								{t("account:title", { defaultValue: "Account" })}
							</DialogTitle>
							<DialogDescription className="text-center text-sm text-muted-foreground">
								{t("account:welcomeSubtitle", {
									defaultValue: "Sign in with your GitHub account",
								})}
							</DialogDescription>
							<Button
								type="button"
								variant="outline"
								className="mt-6 h-11 w-full gap-2 border-slate-200 bg-background font-normal shadow-sm hover:bg-slate-50 dark:border-slate-600 dark:hover:bg-slate-900/80"
								disabled={busy || (tauri && !status)}
								onClick={() => void onPrimaryAuth()}
							>
								<GitHubMark className="h-4 w-4 shrink-0" aria-hidden />
								{t("account:connect", {
									defaultValue: "Sign in with GitHub",
								})}
							</Button>
						</>
					) : (
						<>
							<DialogTitle className="text-left text-2xl font-semibold tracking-tight text-foreground">
								{t("account:signedInTitle", {
									defaultValue: "You're signed in",
								})}
							</DialogTitle>
							<DialogDescription className="pt-2 text-left text-sm text-muted-foreground">
								{t("account:signedInSubtitle", {
									defaultValue: "Your GitHub account is connected to this app.",
								})}
							</DialogDescription>
							<p className="mt-3 text-xs leading-relaxed text-muted-foreground">
								{t("account:cloudSignedInFootnote", {
									defaultValue:
										"When backup and sync are ready, you'll see them here first.",
								})}
							</p>
							<Button
								type="button"
								variant="outline"
								className="mt-6 h-11 w-full border-slate-200 bg-background font-normal shadow-sm hover:bg-slate-50 dark:border-slate-600 dark:hover:bg-slate-900/80"
								disabled={busy}
								onClick={() => void onLogout()}
							>
								{t("account:disconnect", {
									defaultValue: "Sign out",
								})}
							</Button>
						</>
					)}

					{tauri && !status?.loggedIn && status ? (
						<p className="mt-4 text-xs leading-relaxed text-muted-foreground">
							{t("account:syncSoon", {
								defaultValue:
									"Cross-device backup and sync are not available yet. Signed-in users will get access first.",
							})}
						</p>
					) : null}

					{tauri && status ? (
						<div className="mt-6 space-y-1 rounded-md border border-slate-200 bg-slate-50 p-3 text-slate-700 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-200">
							<p className="text-xs font-medium uppercase tracking-wide text-slate-500 dark:text-slate-400">
								{t("account:localDeviceSection", {
									defaultValue: "This device",
								})}
							</p>
							<p>
								<span className="font-medium text-slate-600 dark:text-slate-400">
									{t("account:deviceLabel", {
										defaultValue: "Device ID",
									})}
									{": "}
								</span>
								<code className="rounded bg-slate-100 px-1.5 py-0.5 text-xs dark:bg-slate-800">
									{status.deviceId}
								</code>
							</p>
							<p>
								<span className="font-medium text-slate-600 dark:text-slate-400">
									{t("account:hostLabel", {
										defaultValue: "Device name",
									})}
									{": "}
								</span>
								{status.deviceName}
							</p>
						</div>
					) : null}
				</div>

				<div className="border-t border-slate-200 bg-slate-50/80 px-6 py-4 dark:border-slate-700 dark:bg-slate-900/40 sm:px-8">
					<p className="text-center text-[11px] leading-relaxed text-muted-foreground">
						<Trans
							i18nKey="account:legalNotice"
							components={{
								termsLink: (
									<a
										href={termsHref}
										target="_blank"
										rel="noopener noreferrer"
										className={linkClass}
									/>
								),
								privacyLink: (
									<a
										href={privacyHref}
										target="_blank"
										rel="noopener noreferrer"
										className={linkClass}
									/>
								),
							}}
						/>
					</p>
				</div>
			</DialogContent>
		</Dialog>
	);
}
