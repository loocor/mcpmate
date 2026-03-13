import {
	ArrowRight,
	Check,
	Copy,
	Download,
	ShieldAlert,
	Terminal,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { trackMCPMateEvents } from "../../utils/analytics";
import {
	detectPlatform,
	getCountdown,
	getInstallScriptUrl,
	getMacVariantSha256,
	getMacVariantUrl,
	getPreviewExpiry,
	getPreviewVersion,
	isPreviewSuspended,
	type MacVariant,
} from "../../utils/downloads";
import { useLanguage } from "../LanguageProvider";
import { useNavigate } from "react-router-dom";
import Button from "../ui/Button";
import Section from "../ui/Section";

const CONTACT_EMAIL = "info@mcpmate.io";
const DISCORD_URL = "https://discord.com/channels/1369086293933559838";

const DownloadSection = () => {
	const [selectedPlatform] = useState(() => detectPlatform());
	const [copiedKey, setCopiedKey] = useState<string | null>(null);

	const version = useMemo(() => getPreviewVersion(), []);
	const installUrl = useMemo(() => getInstallScriptUrl(), []);
	const expiry = useMemo(() => getPreviewExpiry(), []);
	const previewSuspended = useMemo(() => isPreviewSuspended(), []);
	const [now, setNow] = useState<Date>(new Date());

	useEffect(() => {
		if (!expiry) return;
		const timer = setInterval(() => setNow(new Date()), 1000);
		return () => clearInterval(timer);
	}, [expiry]);

	const countdown = expiry ? getCountdown(expiry, now) : null;
    const { t, language } = useLanguage();
    const navigate = useNavigate();

	return (
		<Section className="bg-gradient-to-b from-blue-50 to-white dark:from-slate-800/50 dark:to-slate-900 border-t border-slate-200/70 dark:border-slate-700/50">
			<div className="max-w-4xl mx-auto">
				<div className="text-center mb-12">
					<h2 className="text-3xl md:text-4xl font-bold mb-2">
						{t("download.title")}
					</h2>
					{expiry && countdown && (
						<div
							className={`inline-flex items-center gap-2 text-sm px-3 py-1 rounded-full ${countdown.expired ? "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-200" : "bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-200"}`}
						>
							<ShieldAlert className="h-4 w-4" />
							{countdown.expired ? (
								<span>{t("download.expired")}</span>
							) : (
								<span>
									{t("download.expires_in")} {countdown.days}d {countdown.hours}
									h {countdown.minutes}m {countdown.seconds}s
								</span>
							)}
						</div>
					)}
					<p className="text-lg text-slate-600 dark:text-slate-400 mt-3">
						{t("download.subtitle")}
					</p>
				</div>

				<div className="grid grid-cols-1 md:grid-cols-2 gap-8 mb-12">
					<div>
						<div className="flex items-center justify-between mb-4">
							<h3 className="text-lg font-semibold">
								{t("download.for")}{" "}
								{selectedPlatform === "mac"
									? "macOS"
									: selectedPlatform === "windows"
										? t("download.windows")
										: t("download.linux")}
							</h3>
							<Download className="h-5 w-5 text-blue-600 dark:text-blue-400" />
						</div>

						{/* macOS variants (arm64, x64) */}
						<div className="space-y-3">
						{([
							[t("download.macos.arm64"), "arm64"],
							[t("download.macos.x64"), "x64"],
						] as const).map(([label, key]) => {
							const url = getMacVariantUrl(key as MacVariant);
							const sha = getMacVariantSha256(key as MacVariant);
							const disabled = previewSuspended || !url;
							const buttonText = disabled
								? previewSuspended
									? `${label} — ${t("download.preview_paused")}`
									: `${label} — ${t("download.coming_soon")}`
								: `${t("download.btn")} ${label}`;
								return (
									<div key={key} className="border rounded-lg p-3">
										<div className="flex items-center gap-3">
											<Button
												size="lg"
										className="flex-1 cursor-pointer disabled:cursor-not-allowed"
										disabled={disabled}
										onClick={() => {
											if (!url || previewSuspended) return;
											trackMCPMateEvents.downloadClick(`mac-${key}`);
											window.open(url, "_blank");
										}}
									>
										{buttonText}
											</Button>
										</div>
							{!previewSuspended && sha && (
											<div className="mt-1 text-[10px] leading-3 text-slate-500 dark:text-slate-400 flex items-center justify-between gap-2">
												<span className="truncate">
													{t("download.sha256")}: {sha}
												</span>
												<button
													className="inline-flex items-center gap-1 px-2 py-1 rounded hover:bg-slate-100 dark:hover:bg-slate-800"
													onClick={async () => {
														try {
															await navigator.clipboard.writeText(sha);
															setCopiedKey(key);
															setTimeout(() => setCopiedKey(null), 1200);
														} catch {
															/* noop */
														}
													}}
												>
													{copiedKey === key ? (
														<Check className="h-3 w-3" />
													) : (
														<Copy className="h-3 w-3" />
													)}
												</button>
											</div>
										)}
									</div>
								);
							})}
						</div>
						{/* Windows/Linux buttons removed for initial release */}
					</div>

					<div className="space-y-6">
						{installUrl && (
							<div>
								<h3 className="text-lg font-semibold mb-2">
									{t("download.quick_start")}
								</h3>
								<div className="bg-slate-900 rounded-lg p-4 font-mono text-sm text-white">
									<div className="flex items-center gap-2 mb-2">
										<Terminal size={16} />
										<span className="text-slate-400">
											{t("download.install_cli")}
										</span>
									</div>
									<code>curl -fsSL {installUrl} | sh</code>
								</div>
							</div>
						)}

						<div>
							<h3 className="text-lg font-semibold mb-2">
								{t("download.getting_started")}
							</h3>
							<p className="text-slate-600 dark:text-slate-400 mb-4">
								{t("download.getting_started.desc")}
							</p>
							{/* Move preview expiry/upgrade notice above the guide button and style as body text */}
						<p className="text-slate-600 dark:text-slate-400 mb-4 space-y-1">
							<span className="block">{t("download.notarize_notice")}</span>
							<span className="block">
								{t("download.contact_intro")}{" "}
								<a
									href={`mailto:${CONTACT_EMAIL}`}
									className="text-blue-600 dark:text-blue-400 underline underline-offset-2"
								>
									{CONTACT_EMAIL}
								</a>{" "}
								{t("download.contact_or")}{" "}
								<a
									href={DISCORD_URL}
									target="_blank"
									rel="noopener noreferrer"
									className="text-blue-600 dark:text-blue-400 underline underline-offset-2"
								>
									{t("download.contact_discord")}
								</a>{" "}
								{t("download.contact_suffix")}
							</span>
						</p>
							<Button
								variant="outline"
								className="w-full flex items-center justify-center gap-2"
                        onClick={() => navigate(language === 'zh' ? '/docs/zh/quickstart' : '/docs/en/quickstart')}
							>
								<span>{t("download.read_guide")}</span>
								<ArrowRight size={16} />
							</Button>
						</div>
					</div>
				</div>
			</div>
		</Section>
	);
};

export default DownloadSection;
