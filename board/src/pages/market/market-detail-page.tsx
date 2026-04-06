import {
	Code,
	Download,
	ExternalLink,
	Globe,
	ShieldCheck,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import ReactMarkdown from "react-markdown";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import remarkGfm from "remark-gfm";
import { ErrorDisplay } from "../../components/error-display";
import { Avatar, AvatarFallback, AvatarImage } from "../../components/ui/avatar";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { ServerInstallWizard } from "../../components/uniimport/server-install-wizard";
import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";
import { useServerInstallPipeline } from "../../hooks/use-server-install-pipeline";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { serversApi } from "../../lib/api";
import {
	fetchCachedRegistryServerByKey,
	getCanonicalRegistryServerId,
	getOfficialMeta,
	matchesInstalledRegistryServer,
} from "../../lib/registry";
import type { RegistryServerEntry } from "../../lib/types";
import { formatLocalDateTime } from "../../lib/utils";
import type { RemoteOption } from "./types";
import {
	buildDraftFromRemoteOption,
	formatServerName,
	getRemoteTypeLabel,
	normalizeRemoteKind,
	slugifyForConfig,
} from "./utils";

function buildRemoteOptions(server: RegistryServerEntry): RemoteOption[] {
	const options: RemoteOption[] = [];

	(server.remotes ?? []).forEach((remote, idx) => {
		const kind = normalizeRemoteKind(remote.type);
		if (!kind || !remote?.url) return;
		options.push({
			id: `${server.name}-remote-${idx}`,
			label: `${getRemoteTypeLabel(kind)} • ${remote.url}`,
			kind,
			source: "remote",
			url: remote.url,
			headers: remote.headers ?? null,
			envVars: null,
			packageIdentifier: null,
			packageMeta: null,
		});
	});

	(server.packages ?? []).forEach((pkg, idx) => {
		const kind = normalizeRemoteKind(pkg.transport?.type);
		if (!kind) return;
		const identifier = pkg.identifier ?? pkg.registryType ?? `package-${idx + 1}`;
		options.push({
			id: `${server.name}-package-${idx}`,
			label: `${getRemoteTypeLabel(kind)} • ${identifier}`,
			kind,
			source: "package",
			url: null,
			headers: null,
			envVars: pkg.environmentVariables ?? null,
			packageIdentifier: identifier,
			packageMeta: pkg,
		});
	});

	return options;
}

function MetadataGridRow({
	label,
	value,
	valueClassName,
}: {
	label: string;
	value: ReactNode;
	valueClassName?: string;
}) {
	return (
		<>
			<span className="shrink-0 text-xs uppercase text-slate-500">{label}</span>
			<div
				className={
					valueClassName ?? "min-w-0 text-sm text-slate-600 break-words dark:text-slate-300"
				}
			>
				{value}
			</div>
		</>
	);
}

function summarizeTransportTypes(server: RegistryServerEntry): string {
	const kindSet = new Set<string>();

	(server.remotes ?? []).forEach((remote) => {
		const kind = normalizeRemoteKind(remote.type);
		if (kind) {
			kindSet.add(kind);
		}
	});

	(server.packages ?? []).forEach((pkg) => {
		const kind = normalizeRemoteKind(pkg.transport?.type);
		if (kind) {
			kindSet.add(kind);
		}
	});

	if (!kindSet.size) {
		return "—";
	}

	return Array.from(kindSet).map((kind) => getRemoteTypeLabel(kind)).join(" / ");
}

function parseGitHubRepositoryUrl(repositoryUrl: string): { owner: string; repo: string } | null {
	const matched = repositoryUrl.match(/^https?:\/\/github\.com\/([^/]+)\/([^/#?]+)/i);
	if (!matched) return null;
	const owner = matched[1];
	const repo = matched[2].replace(/\.git$/i, "");
	if (!owner || !repo) return null;
	return { owner, repo };
}

function decodeBase64Utf8(content: string): string {
	const normalized = content.replace(/\n/g, "");
	const binary = atob(normalized);
	const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
	return new TextDecoder().decode(bytes);
}

async function fetchRepositoryReadmeMarkdown(repositoryUrl: string, subfolder?: string | null): Promise<string> {
	const parsed = parseGitHubRepositoryUrl(repositoryUrl);
	if (!parsed) {
		throw new Error("unsupported-repository");
	}

	const normalizedSubfolder = (subfolder ?? "").trim().replace(/^\/+|\/+$/g, "");
	const readmePath = normalizedSubfolder ? `${normalizedSubfolder}/README.md` : "README.md";
	const encodedPath = readmePath
		.split("/")
		.map((segment) => encodeURIComponent(segment))
		.join("/");
	const endpoint = `https://api.github.com/repos/${parsed.owner}/${parsed.repo}/contents/${encodedPath}`;
	const response = await fetch(endpoint, {
		headers: {
			Accept: "application/vnd.github+json",
		},
	});

	if (!response.ok) {
		throw new Error(`readme-fetch-failed-${response.status}`);
	}

	const payload = (await response.json()) as { content?: string; encoding?: string };
	if (!payload.content || payload.encoding !== "base64") {
		throw new Error("readme-content-invalid");
	}

	return decodeBase64Utf8(payload.content);
}

function getReadmeErrorMessage(
	error: unknown,
	t: ReturnType<typeof useTranslation>["t"],
): string {
	const errorMessage = String(error);
	if (errorMessage.includes("unsupported-repository")) {
		return t("market:detail.readmeUnsupported", {
			defaultValue: "README preview currently supports GitHub repositories only.",
		});
	}

	return t("market:detail.readmeFetchFailed", {
		defaultValue: "Failed to load README content.",
	});
}

const MARKET_DETAIL_SPLIT_STORAGE_KEY = "marketDetail.registryReadmeSplitPct";

function clampMarketDetailSplitPct(n: number): number {
	if (!Number.isFinite(n)) return 33.33;
	return Math.min(78, Math.max(22, n));
}

function readInitialMarketDetailSplitPct(): number {
	if (typeof window === "undefined") return 33.33;
	try {
		const raw = localStorage.getItem(MARKET_DETAIL_SPLIT_STORAGE_KEY);
		if (raw) return clampMarketDetailSplitPct(parseFloat(raw));
	} catch {
		/* noop */
	}
	return 33.33;
}

export function MarketDetailPage() {
	const { t } = useTranslation("market");
	usePageTranslations("market");
	const navigate = useNavigate();
	const { registryKey } = useParams();
	const decodedKey = useMemo(
		() => decodeURIComponent(registryKey ?? ""),
		[registryKey],
	);
	const serverQuery = useQuery({
		queryKey: ["market", "detail", decodedKey],
		queryFn: () => fetchCachedRegistryServerByKey(decodedKey),
		enabled: Boolean(decodedKey),
	});
	const installedServersQuery = useQuery({
		queryKey: ["servers"],
		queryFn: () => serversApi.getAll(),
		staleTime: 30_000,
	});

	const [drawerOpen, setDrawerOpen] = useState(false);
	const [selectedTransportId, setSelectedTransportId] = useState("");
	const server = serverQuery.data ?? null;
	const repositoryUrl = server?.repository?.url ?? "";
	const repositorySubfolder = server?.repository?.subfolder ?? "";
	const readmeQuery = useQuery({
		queryKey: ["market", "detail", "readme", repositoryUrl, repositorySubfolder],
		queryFn: () => fetchRepositoryReadmeMarkdown(repositoryUrl, repositorySubfolder),
		enabled: Boolean(repositoryUrl),
		retry: false,
		staleTime: 5 * 60 * 1000,
	});

	const remoteOptions = useMemo(() => (server ? buildRemoteOptions(server) : []), [server]);
	const selectedRemote = useMemo(
		() => remoteOptions.find((option) => option.id === selectedTransportId) ?? remoteOptions[0] ?? null,
		[selectedTransportId, remoteOptions],
	);

	const initialDraft = useMemo<ServerInstallDraft | undefined>(() => {
		if (!selectedRemote || !server) return undefined;
		const draft = buildDraftFromRemoteOption(selectedRemote, slugifyForConfig(server.name));
		return {
			...draft,
			registryServerId: server.name,
			meta: {
				description: server.description || "",
				version: server.version || "",
				websiteUrl: server.websiteUrl || "",
				repository: server.repository
					? {
						url: server.repository.url || "",
						source: server.repository.source || "",
						subfolder: server.repository.subfolder || "",
						id: server.repository.id || "",
					}
					: undefined,
				icons: server.icons ? server.icons.map((icon) => ({ ...icon })) : undefined,
				_meta: server._meta,
				extras: {
					...(server.extras ?? {}),
					packages: server.packages ?? [],
					remotes: server.remotes ?? [],
					status: server.status ?? null,
				},
			},
		};
	}, [selectedRemote, server]);

	const installPipeline = useServerInstallPipeline({
		onImported: () => {
			setDrawerOpen(false);
			void installedServersQuery.refetch();
		},
	});

	const [leftSplitPct, setLeftSplitPct] = useState(readInitialMarketDetailSplitPct);
	const leftSplitPctRef = useRef(leftSplitPct);
	leftSplitPctRef.current = leftSplitPct;
	const splitContainerRef = useRef<HTMLDivElement>(null);

	const [isLgLayout, setIsLgLayout] = useState(
		() => typeof window !== "undefined" && window.matchMedia("(min-width: 1024px)").matches,
	);
	useEffect(() => {
		const mq = window.matchMedia("(min-width: 1024px)");
		const onChange = () => setIsLgLayout(mq.matches);
		mq.addEventListener("change", onChange);
		return () => mq.removeEventListener("change", onChange);
	}, []);

	const onSplitPointerDown = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
		if (!isLgLayout) return;
		e.preventDefault();
		document.body.style.cursor = "col-resize";
		document.body.style.userSelect = "none";
		const onMove = (ev: PointerEvent) => {
			const el = splitContainerRef.current;
			if (!el) return;
			const rect = el.getBoundingClientRect();
			const pct = clampMarketDetailSplitPct(((ev.clientX - rect.left) / rect.width) * 100);
			leftSplitPctRef.current = pct;
			setLeftSplitPct(pct);
		};
		const onUp = () => {
			document.body.style.cursor = "";
			document.body.style.userSelect = "";
			document.removeEventListener("pointermove", onMove);
			document.removeEventListener("pointerup", onUp);
			try {
				localStorage.setItem(MARKET_DETAIL_SPLIT_STORAGE_KEY, String(leftSplitPctRef.current));
			} catch {
				/* noop */
			}
		};
		document.addEventListener("pointermove", onMove);
		document.addEventListener("pointerup", onUp);
	}, [isLgLayout]);

	if (serverQuery.error) {
		return (
			<ErrorDisplay
				title={t("market:errors.failedToLoadRegistry", { defaultValue: "Failed to load registry" })}
				error={serverQuery.error as Error}
				onRetry={() => window.location.reload()}
			/>
		);
	}

	if (!server) {
		return (
			<ErrorDisplay
				title={t("market:detail.notFoundTitle", { defaultValue: "Registry entry not found" })}
				error={new Error(decodedKey || "Missing registry key")}
				onRetry={() => navigate("/market")}
			/>
		);
	}

	const official = getOfficialMeta(server);
	const canonicalRegistryId = getCanonicalRegistryServerId(server);
	const displayName = formatServerName(server.name);
	const installedServer =
		installedServersQuery.data?.servers.find((item) => {
			return matchesInstalledRegistryServer(server, item);
		}) ?? null;
	const isInstalled = Boolean(installedServer);
	const primaryIconSrc = server.icons?.[0]?.src;
	const transportTypeSummary = summarizeTransportTypes(server);
	const links: Array<{ label: string; url: string; icon: typeof Globe }> = [];
	if (server.websiteUrl) {
		links.push({
			label: t("market:detail.website", { defaultValue: "Website" }),
			url: server.websiteUrl,
			icon: Globe,
		});
	}
	if (repositoryUrl) {
		links.push({
			label: t("market:detail.repository", { defaultValue: "Repository" }),
			url: repositoryUrl,
			icon: Code,
		});
	}
	const splitGridStyle = isLgLayout
		? {
				gridTemplateColumns: `minmax(0,${leftSplitPct}fr) 8px minmax(0,${100 - leftSplitPct}fr)`,
			}
		: undefined;
	const readmeErrorText = getReadmeErrorMessage(readmeQuery.error, t);

	return (
		<>
			<div className="flex h-full min-h-0 flex-col gap-4 overflow-hidden">
				<div className="flex shrink-0 flex-col gap-2 md:flex-row md:items-center md:justify-between">
					<div className="flex min-w-0 items-center gap-3">
						<h2 className="min-w-0 break-words text-3xl font-bold tracking-tight">
							{displayName}
						</h2>
					</div>
				</div>
				<div className="min-h-0 flex-1 overflow-y-auto">
					<div className="space-y-6">
				<Card>
					<CardContent className="relative p-4">
						<div className="mb-3 flex flex-wrap items-center justify-end gap-2 sm:absolute sm:top-4 sm:right-4 sm:z-10 sm:mb-0">
							{links.map((link) => {
								const Icon = link.icon;
								return (
									<Button key={link.url} variant="outline" asChild>
										<a href={link.url} target="_blank" rel="noopener noreferrer">
											<Icon className="mr-2 h-4 w-4" />
											{link.label}
											<ExternalLink className="ml-2 h-4 w-4" />
										</a>
									</Button>
								);
							})}
							{isInstalled ? (
								<>
									<Button
										variant="outline"
										onClick={() =>
											navigate(`/servers/${encodeURIComponent(installedServer!.id)}`)
										}
									>
										{t("market:buttons.manage", { defaultValue: "Manage" })}
									</Button>
									<Button disabled>
										<ShieldCheck className="mr-2 h-4 w-4" />
										{t("market:buttons.installed", { defaultValue: "Installed" })}
									</Button>
								</>
							) : (
								<Button
									onClick={() => {
										if (remoteOptions.length > 0) {
											setSelectedTransportId(remoteOptions[0].id);
										}
										setDrawerOpen(true);
									}}
								>
									<Download className="mr-2 h-4 w-4" />
									{t("market:buttons.install", { defaultValue: "Install" })}
								</Button>
							)}
						</div>
						<div className="flex w-full flex-wrap items-start gap-4 sm:pr-56">
							<Avatar className="h-12 w-12 shrink-0 bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-200 text-sm font-medium">
								{primaryIconSrc ? <AvatarImage src={primaryIconSrc} alt={displayName} /> : null}
								<AvatarFallback>{displayName.charAt(0).toUpperCase()}</AvatarFallback>
							</Avatar>
							<div className="min-w-0 flex-1 grid grid-cols-[auto_minmax(0,1fr)] gap-x-5 gap-y-2 text-sm">
								<MetadataGridRow
									label={t("market:detail.service", { defaultValue: "Service" })}
									value={
										official ? (
											<Badge variant="secondary" className="justify-self-start bg-emerald-100 text-emerald-700 dark:bg-emerald-500/20 dark:text-emerald-200">
												<ShieldCheck className="mr-1 h-3 w-3" />
												{t("market:detail.officialRegistry", { defaultValue: "Official Registry" })}
											</Badge>
										) : (
											t("market:notifications.registryEntry", { defaultValue: "Registry entry" })
										)
									}
								/>
								<MetadataGridRow
									label={t("market:detail.runtime", { defaultValue: "Runtime" })}
									value={server.status ?? official?.status ?? "—"}
								/>
								<MetadataGridRow
									label={t("market:detail.installation", { defaultValue: "Installation" })}
									value={
										isInstalled
											? t("market:detail.installedValue", {
												defaultValue: "Installed in MCPMate",
											})
											: t("market:detail.notInstalledValue", {
												defaultValue: "Not installed yet",
											})
									}
								/>
								<MetadataGridRow
									label={t("market:detail.type", { defaultValue: "Type" })}
									value={transportTypeSummary}
									valueClassName="min-w-0 break-words font-mono text-sm leading-tight"
								/>
								<MetadataGridRow
									label={t("market:detail.description", { defaultValue: "Description" })}
									value={server.description || t("market:detail.noDescription", { defaultValue: "No description provided." })}
								/>
							</div>
						</div>
					</CardContent>
				</Card>

				<div
					ref={splitContainerRef}
					className="grid grid-cols-1 items-stretch gap-6 lg:grid-cols-none lg:items-stretch lg:gap-0"
					style={splitGridStyle}
				>
					<Card className="min-w-0 rounded-xl lg:h-full lg:min-h-0 lg:rounded-none lg:rounded-l-xl">
						<CardHeader>
							<CardTitle>{t("market:detail.repositoryRegistrySection", { defaultValue: "Repository & Registry" })}</CardTitle>
						</CardHeader>
						<CardContent className="space-y-4 p-4">
							<div className="space-y-2">
								<p className="text-xs uppercase tracking-wide text-slate-500 dark:text-slate-400">
									{t("market:detail.repositorySection", { defaultValue: "Repository & Project" })}
								</p>
								<div className="grid grid-cols-[auto_1fr] gap-x-5 gap-y-2 text-sm">
									<MetadataGridRow
										label={t("market:detail.repositorySource", { defaultValue: "Repository source" })}
										value={server.repository?.source ?? "—"}
									/>
									<MetadataGridRow
										label={t("market:detail.repositorySubfolder", { defaultValue: "Repository subfolder" })}
										value={server.repository?.subfolder ?? "—"}
									/>
									<MetadataGridRow
										label={t("market:detail.repositoryId", { defaultValue: "Repository Entry ID (Metadata)" })}
										value={server.repository?.id ?? "—"}
									/>
								</div>
								<p className="text-xs text-slate-500 dark:text-slate-400">
									{t("market:detail.repositoryIdHint", {
										defaultValue:
											"Repository Entry ID is optional repository metadata and is not used as the managed server linkage key.",
									})}
								</p>
							</div>

							<div className="space-y-2">
								<p className="text-xs uppercase tracking-wide text-slate-500 dark:text-slate-400">
									{t("market:detail.registryMeta", { defaultValue: "Registry metadata" })}
								</p>
								<div className="grid grid-cols-[auto_1fr] gap-x-5 gap-y-2 text-sm">
									<MetadataGridRow
										label={t("market:detail.officialStatus", { defaultValue: "Official status" })}
										value={official?.status ?? "—"}
									/>
									<MetadataGridRow
										label={t("market:detail.publishedAt", { defaultValue: "Published at" })}
										value={official?.publishedAt ? formatLocalDateTime(official.publishedAt) : "—"}
									/>
									<MetadataGridRow
										label={t("market:detail.updatedAt", { defaultValue: "Updated at" })}
										value={official?.updatedAt ? formatLocalDateTime(official.updatedAt) : "—"}
									/>
									<MetadataGridRow
										label={t("market:detail.versionId", { defaultValue: "Version ID" })}
										value={official?.versionId ?? "—"}
									/>
									<MetadataGridRow
										label={t("market:detail.serverId", { defaultValue: "Registry ID (Canonical Key)" })}
										value={canonicalRegistryId}
									/>
								</div>
								<p className="text-xs text-slate-500 dark:text-slate-400">
									{t("market:detail.registryIdHint", {
										defaultValue:
											"MCPMate links managed servers by official server.name (official.serverId is treated as an alias only when equivalent).",
									})}
								</p>
							</div>
						</CardContent>
					</Card>

					<div
						className="hidden h-full min-h-0 w-2 shrink-0 cursor-col-resize touch-none self-stretch bg-transparent lg:block"
						role="separator"
						aria-orientation="vertical"
						aria-valuemin={22}
						aria-valuemax={78}
						aria-valuenow={Math.round(leftSplitPct)}
						aria-label={t("market:detail.splitResize", { defaultValue: "Resize registry and README panels" })}
						onPointerDown={onSplitPointerDown}
					/>

					<Card className="min-w-0 rounded-xl lg:h-full lg:min-h-0 lg:rounded-none lg:rounded-r-xl">
						<CardHeader>
							<CardTitle>{t("market:detail.readme", { defaultValue: "README" })}</CardTitle>
						</CardHeader>
						<CardContent className="p-4">
							{!repositoryUrl ? (
								<p className="text-sm text-muted-foreground">
									{t("market:detail.readmeUnavailable", { defaultValue: "README is unavailable because repository URL is missing." })}
								</p>
							) : readmeQuery.isLoading ? (
								<p className="text-sm text-muted-foreground">
									{t("market:detail.readmeLoading", { defaultValue: "Loading README..." })}
								</p>
							) : readmeQuery.isError ? (
								<p className="text-sm text-muted-foreground">
									{readmeErrorText}
								</p>
							) : readmeQuery.data ? (
								<ReactMarkdown
									remarkPlugins={[remarkGfm]}
									components={{
										a: ({ href, children }) => {
											const url = href ?? "";
											return (
												<a href={url} target="_blank" rel="noopener noreferrer" className="text-blue-600 hover:underline dark:text-blue-400">
													{children}
												</a>
											);
										},
										p: ({ children }) => <p className="mb-3 text-sm leading-6 text-slate-700 dark:text-slate-300">{children}</p>,
										h1: ({ children }) => <h1 className="mb-3 text-xl font-semibold">{children}</h1>,
										h2: ({ children }) => <h2 className="mb-3 text-lg font-semibold">{children}</h2>,
										h3: ({ children }) => <h3 className="mb-2 text-base font-semibold">{children}</h3>,
										ul: ({ children }) => <ul className="mb-3 list-disc space-y-1 pl-5 text-sm text-slate-700 dark:text-slate-300">{children}</ul>,
										ol: ({ children }) => <ol className="mb-3 list-decimal space-y-1 pl-5 text-sm text-slate-700 dark:text-slate-300">{children}</ol>,
										code: ({ children }) => <code className="rounded bg-slate-100 px-1 py-0.5 text-xs dark:bg-slate-800">{children}</code>,
										pre: ({ children }) => <pre className="mb-3 overflow-x-auto rounded-lg bg-slate-100 p-3 text-xs dark:bg-slate-900">{children}</pre>,
										blockquote: ({ children }) => <blockquote className="mb-3 border-l-2 border-slate-300 pl-3 text-sm text-slate-600 dark:border-slate-700 dark:text-slate-400">{children}</blockquote>,
									}}
								>
									{readmeQuery.data}
								</ReactMarkdown>
							) : (
								<p className="text-sm text-muted-foreground">
									{t("market:detail.readmeEmpty", { defaultValue: "README content is empty." })}
								</p>
							)}
						</CardContent>
					</Card>
				</div>

				<div className="border-t border-slate-200 dark:border-slate-800" />
					</div>
				</div>
			</div>

			<ServerInstallWizard
				isOpen={drawerOpen}
				onClose={() => setDrawerOpen(false)}
				mode="import"
				initialDraft={initialDraft}
				allowProgrammaticIngest
				pipeline={installPipeline}
			/>
		</>
	);
}
