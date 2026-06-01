import { useState, type ReactNode } from "react";
import { ArrowRightLeft, MonitorSmartphone } from "lucide-react";
import mcpIcon from "../../assets/images/mcp-icon.svg";
import logoImage from "../../assets/images/logo.svg";
import { useLanguage } from "../LanguageProvider";
import Section from "../ui/Section";

type ActiveNode = "clients" | "proxy" | "upstream";
type NodeTone = "accent" | "indigo";

const HOW_NODE_CARD_BTN =
	"how-node-shell__btn group cursor-pointer text-left flex flex-col justify-between p-6 rounded-2xl w-full h-full transition-all duration-300 ease-out relative overflow-hidden focus:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-brand-bg hover:-translate-y-0.5";

const HOW_NODE_ICON_WRAP = "transition-transform duration-300 group-hover:scale-110";

function HowNodeShell({
	active,
	tone,
	className = "",
	children,
}: {
	active: boolean;
	tone: NodeTone;
	className?: string;
	children: ReactNode;
}) {
	const activeClass =
		tone === "indigo" ? "how-node-shell--active-indigo" : "how-node-shell--active-accent";
	const hoverClass =
		tone === "indigo"
			? "hover:border-brand-indigo/60 hover:ring-1 hover:ring-brand-indigo/20"
			: "hover:border-brand-accent/60 hover:ring-1 hover:ring-brand-accent/20";

	return (
		<div
			className={`how-node-shell glass-card h-full border shadow-lg transition-all duration-300 ease-out ${className} ${
				active
					? `${activeClass} ring-1 scale-[1.02] ${tone === "indigo" ? "ring-brand-indigo/30" : "ring-brand-accent/30"}`
					: `border-brand-border-subtle ${hoverClass}`
			}`}
		>
			{children}
		</div>
	);
}

const HowItWorks = () => {
	const { t, language } = useLanguage();
	const [activeNode, setActiveNode] = useState<ActiveNode>("proxy");

	const labels = {
		zh: {
			clientsTitle: "多端 AI 应用",
			clientsDesc: "Cursor、Claude Desktop、VS Code、Zed、命令行等",
			proxyTitle: "MCPMate 核心",
			proxyDesc: "集中式连接池、配置集裁剪、安全策略与实时日志",
			upstreamTitle: "MCP 服务库",
			upstreamDesc: "uvx、bunx、npx 本地服务，或 SSE / Streamable HTTP 远程服务",

			subClients: "多客户端接入",
			subProxy: "智能治理与路由",
			subUpstream: "服务聚合与生命周期",
			activeHint: "详情见下方",
			expandHint: "▼ 点击查看特性",

			features: {
				clients: [
					"统一配置：一次接入，所有客户端自动同步",
					"配置模式：支持 托管、统一、透明 三种配置模式",
					"按端裁剪：不同客户端可见不同工具，降低 Token 消耗"
				],
				proxy: [
					"配置集引擎：按场景（开发、写作、调研）一键切换工具",
					"安全审计：本地 redb 缓存，敏感操作实时监控与结构化日志",
					"连接池管理：复用上游连接，大幅降低本机系统资源消耗"
				],
				upstream: [
					"一键安装：集成 uv (Python)、Node.js、Bun 运行时管理",
					"协议桥接：自动将 stdio 转换为 Streamable HTTP 协议",
					"服务源集成：无缝对接官方 MCP Marketplace 市场"
				]
			},
		},
		en: {
			clientsTitle: "AI Clients",
			clientsDesc: "Cursor, Claude Desktop, VS Code, Zed, CLI, SDKs, etc.",
			proxyTitle: "MCPMate Core",
			proxyDesc: "Central connection pool, profile engine, policies & live logs",
			upstreamTitle: "MCP Servers",
			upstreamDesc: "Local uvx/bunx/npx processes, or remote SSE / Streamable HTTP",

			subClients: "Multi-Client Access",
			subProxy: "Smart Governance & Routing",
			subUpstream: "Aggregation & Lifecycle",

			features: {
				clients: [
					"Unified Config: Connect once, sync automatically across all clients",
					"Setup Modes: Supports Hosted, Unify, and Transparent modes",
					"Per-Client Trimming: Limit visible tools per app to save tokens"
				],
				proxy: [
					"Profile Engine: Switch tool sets (coding, writing, research) instantly",
					"Security & Audit: Local redb cache, structured logs, and policies",
					"Connection Pool: Share upstream connections to save system resources"
				],
				upstream: [
					"Runtime Manager: Built-in uv (Python), Node.js, and Bun runtime controls",
					"Protocol Bridging: Translates stdio to Streamable HTTP seamlessly",
					"Marketplace: Browse and install directly from the MCP registry"
				]
			},
			activeHint: "Details below",
			expandHint: "▼ View details",
		},
		ja: {
			clientsTitle: "AI クライアント",
			clientsDesc: "Cursor、Claude Desktop、VS Code、Zed、CLI、SDK など",
			proxyTitle: "MCPMate Core",
			proxyDesc: "接続プール、プロファイル切替、セキュリティポリシー、リアルタイムログ",
			upstreamTitle: "MCP サーバー",
			upstreamDesc: "ローカル uvx/bunx/npx、またはリモート SSE / Streamable HTTP",

			subClients: "マルチクライアント接続",
			subProxy: "スマートガバナンスとルーティング",
			subUpstream: "サーバー集約とライフサイクル",
			activeHint: "下に詳細を表示",
			expandHint: "▼ 詳細を見る",

			features: {
				clients: [
					"統一設定: 一度接続すれば、すべてのクライアントで自動同期",
					"設定モード: Hosted、Unify、Transparent の3つのモードに対応",
					"アプリ別制御: アプリごとに見えるツールを制限し、トークンを節約"
				],
				proxy: [
					"プロファイルエンジン: 開発、執筆、調査などのツール群を瞬時に切替",
					"セキュリティ監査: ローカル redb キャッシュ、構造化ログ、ポリシー制御",
					"接続プール: 上流接続を共有し、システムリソースの消費を大幅削減"
				],
				upstream: [
					"ランタイム管理: uv (Python)、Node.js、Bun ランタイムの内蔵管理",
					"プロトコルブリッジ: stdio を Streamable HTTP へシームレスに変換",
					"マーケットプレイス: 公式 MCP レジストリから直接検索・インストール"
				]
			},
		}
	};

	const currentLabels = labels[language] ?? labels.en;

	return (
		<Section
			id="how-it-works"
			title={t("how.title")}
			subtitle={t("how.subtitle")}
			centered
			snap
			titleClassName="text-3xl md:text-4xl text-brand-foreground"
			subtitleClassName="section-muted"
		>
			<div className="mx-auto max-w-6xl">
				<div className="relative mb-12 grid grid-cols-1 items-stretch gap-6 md:mb-6 md:grid-cols-[1fr_auto_1.2fr_auto_1fr] md:items-stretch md:gap-6">
					<HowNodeShell active={activeNode === "clients"} tone="accent">
						<button
							type="button"
							onClick={() => setActiveNode("clients")}
							className={`${HOW_NODE_CARD_BTN} focus-visible:ring-brand-accent`}
						>
							<div className="absolute top-0 right-0 w-24 h-24 bg-brand-accent/5 rounded-full blur-2xl pointer-events-none transition-opacity duration-300 group-hover:opacity-100 opacity-70" />
							<div>
								<div className="flex items-center gap-3 mb-3">
									<div
										className={`flex h-11 w-11 shrink-0 items-center justify-center rounded-xl transition-colors ${HOW_NODE_ICON_WRAP} ${
											activeNode === "clients"
												? "bg-brand-accent/20 text-brand-accent"
												: "bg-brand-accent/10 text-brand-accent/80"
										}`}
									>
										<MonitorSmartphone size={22} aria-hidden />
									</div>
									<span className="text-[10px] font-mono uppercase tracking-wider text-brand-accent/80 font-semibold">
										{currentLabels.subClients}
									</span>
								</div>
								<h3 className="text-lg font-bold text-brand-foreground mb-2">
									{currentLabels.clientsTitle}
								</h3>
								<p className="text-xs section-muted leading-relaxed">
									{currentLabels.clientsDesc}
								</p>
							</div>

							<div className="mt-6 pt-4 border-t border-brand-border-subtle/40 flex items-center justify-between w-full text-[11px] font-medium text-brand-accent">
								<span>{activeNode === "clients" ? currentLabels.activeHint : currentLabels.expandHint}</span>
								<div className="flex gap-1">
									<span className="w-1.5 h-1.5 rounded-full bg-brand-accent animate-ping" />
									<span className="w-1.5 h-1.5 rounded-full bg-brand-accent" />
								</div>
							</div>
						</button>
					</HowNodeShell>

					<div className="hidden md:flex flex-col items-center justify-center text-brand-accent/40 px-1">
						<div className="flex flex-col items-center gap-1.5">
							<span className="text-[9px] font-mono uppercase tracking-widest text-brand-muted-soft">
								{t("how.flow.proxy")}
							</span>
							<div className="relative flex items-center justify-center">
								<ArrowRightLeft
									size={20}
									aria-hidden
									className={`transition-colors duration-300 ${activeNode === "clients" || activeNode === "proxy" ? "text-brand-accent" : ""}`}
								/>
								{(activeNode === "clients" || activeNode === "proxy") && (
									<span className="absolute inset-0 bg-brand-accent/20 blur-sm rounded-full animate-ping" />
								)}
							</div>
							<span className="text-[9px] font-mono text-brand-muted-soft">
								Bridge / HTTP
							</span>
						</div>
					</div>

					<HowNodeShell
						active={activeNode === "proxy"}
						tone="indigo"
						className="md:min-h-[17.5rem]"
					>
						<button
							type="button"
							onClick={() => setActiveNode("proxy")}
							className={`${HOW_NODE_CARD_BTN} md:py-8 focus-visible:ring-brand-indigo`}
						>
							<div className="absolute top-0 right-0 w-32 h-32 bg-brand-indigo/5 rounded-full blur-3xl pointer-events-none transition-opacity duration-300 group-hover:opacity-100 opacity-70" />
							<div>
								<div className="flex items-center gap-3 mb-3">
									<div
										className={`flex h-12 w-12 shrink-0 items-center justify-center rounded-xl transition-colors ${HOW_NODE_ICON_WRAP} ${
											activeNode === "proxy"
												? "bg-brand-indigo/20 text-brand-indigo"
												: "bg-brand-indigo/10 text-brand-indigo/80"
										}`}
									>
										<img
											src={logoImage}
											alt=""
											aria-hidden
											className="h-9 w-9 object-contain dark:brightness-0 dark:invert"
										/>
									</div>
									<span className="text-[10px] font-mono uppercase tracking-wider text-brand-indigo font-semibold">
										{currentLabels.subProxy}
									</span>
								</div>
								<h3 className="mb-2 text-lg font-bold leading-tight text-brand-foreground">
									{currentLabels.proxyTitle}
								</h3>
								<p className="text-xs section-muted leading-relaxed">
									{currentLabels.proxyDesc}
								</p>
							</div>

							<div className="mt-6 pt-4 border-t border-brand-border-subtle/40 flex items-center justify-between w-full text-[11px] font-medium text-brand-indigo">
								<span>{activeNode === "proxy" ? currentLabels.activeHint : currentLabels.expandHint}</span>
								<div className="flex gap-1">
									<span className="w-1.5 h-1.5 rounded-full bg-brand-indigo animate-ping" />
									<span className="w-1.5 h-1.5 rounded-full bg-brand-indigo" />
								</div>
							</div>
						</button>
					</HowNodeShell>

					<div className="hidden md:flex flex-col items-center justify-center text-brand-indigo/40 px-1">
						<div className="flex flex-col items-center gap-1.5">
							<span className="text-[9px] font-mono uppercase tracking-widest text-brand-muted-soft">
								{t("how.flow.route")}
							</span>
							<div className="relative flex items-center justify-center">
								<ArrowRightLeft
									size={20}
									aria-hidden
									className={`transition-colors duration-300 ${activeNode === "upstream" || activeNode === "proxy" ? "text-brand-indigo" : ""}`}
								/>
								{(activeNode === "upstream" || activeNode === "proxy") && (
									<span className="absolute inset-0 bg-brand-indigo/20 blur-sm rounded-full animate-ping" />
								)}
							</div>
							<span className="text-[9px] font-mono text-brand-muted-soft">
								stdio / HTTP
							</span>
						</div>
					</div>

					<HowNodeShell active={activeNode === "upstream"} tone="accent">
						<button
							type="button"
							onClick={() => setActiveNode("upstream")}
							className={`${HOW_NODE_CARD_BTN} focus-visible:ring-brand-accent`}
						>
							<div className="absolute top-0 right-0 w-24 h-24 bg-brand-accent/5 rounded-full blur-2xl pointer-events-none transition-opacity duration-300 group-hover:opacity-100 opacity-70" />
							<div>
								<div className="flex items-center gap-3 mb-3">
									<div
										className={`flex h-11 w-11 shrink-0 items-center justify-center rounded-xl transition-colors ${HOW_NODE_ICON_WRAP} ${
											activeNode === "upstream"
												? "bg-brand-accent/20 text-brand-accent"
												: "bg-brand-accent/10 text-brand-accent/80"
										}`}
									>
										<img
											src={mcpIcon}
											alt=""
											aria-hidden
											className="h-[22px] w-[22px] object-contain dark:brightness-0 dark:invert"
										/>
									</div>
									<span className="text-[10px] font-mono uppercase tracking-wider text-brand-accent/80 font-semibold">
										{currentLabels.subUpstream}
									</span>
								</div>
								<h3 className="text-lg font-bold text-brand-foreground mb-2">
									{currentLabels.upstreamTitle}
								</h3>
								<p className="text-xs section-muted leading-relaxed">
									{currentLabels.upstreamDesc}
								</p>
							</div>

							<div className="mt-6 pt-4 border-t border-brand-border-subtle/40 flex items-center justify-between w-full text-[11px] font-medium text-brand-accent">
								<span>{activeNode === "upstream" ? currentLabels.activeHint : currentLabels.expandHint}</span>
								<div className="flex gap-1">
									<span className="w-1.5 h-1.5 rounded-full bg-brand-accent animate-ping" />
									<span className="w-1.5 h-1.5 rounded-full bg-brand-accent" />
								</div>
							</div>
						</button>
					</HowNodeShell>
				</div>

				<div className="relative transition-all duration-300">
					<div className="how-detail-panel rounded-2xl">
						<div className="how-detail-panel__body p-6 md:p-8">
							<div
								className={`how-detail-panel__glow pointer-events-none absolute -top-24 -right-24 h-80 w-80 rounded-full blur-3xl ${
									activeNode === "proxy" ? "bg-brand-indigo" : "bg-brand-accent"
								}`}
							/>

							<div className="relative grid grid-cols-1 items-start md:grid-cols-2 md:gap-0">
								<div
									aria-hidden
									className="how-detail-panel__divider pointer-events-none absolute left-1/2 top-[8%] hidden h-[84%] w-px -translate-x-1/2 md:block"
								/>

								<div className="flex w-full flex-col items-start gap-3 text-left md:pr-10">
									<p
										className={`m-0 w-full text-left text-xs font-mono font-semibold uppercase tracking-wider ${
											activeNode === "proxy" ? "text-brand-indigo" : "text-brand-accent"
										}`}
									>
										{activeNode === "clients" && currentLabels.subClients}
										{activeNode === "proxy" && currentLabels.subProxy}
										{activeNode === "upstream" && currentLabels.subUpstream}
									</p>
									<h4 className="m-0 w-full text-left text-xl font-bold leading-tight text-brand-foreground">
										{activeNode === "clients" && currentLabels.clientsTitle}
										{activeNode === "proxy" && currentLabels.proxyTitle}
										{activeNode === "upstream" && currentLabels.upstreamTitle}
									</h4>
									<p className="m-0 w-full text-left text-sm leading-relaxed section-muted">
										{activeNode === "clients" && currentLabels.clientsDesc}
										{activeNode === "proxy" && currentLabels.proxyDesc}
										{activeNode === "upstream" && currentLabels.upstreamDesc}
									</p>
								</div>

								<div className="space-y-3.5 border-t border-brand-border-subtle/50 pt-6 md:border-t-0 md:pt-0 md:pl-10">
									{currentLabels.features[activeNode].map((feat, idx) => {
										const [title, desc] = feat.split(/[:：]/, 2);
										return (
											<div key={idx} className="flex items-start gap-3">
												<div className={`mt-1 flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-xs font-bold ${
													activeNode === "proxy"
														? "bg-brand-indigo/10 text-brand-indigo"
														: "bg-brand-accent/10 text-brand-accent"
												}`}>
													{idx + 1}
												</div>
												<div>
													<p className="text-sm font-semibold text-brand-foreground">
														{title}
													</p>
													{desc && (
														<p className="text-xs section-muted mt-0.5 leading-relaxed">
															{desc}
														</p>
													)}
												</div>
											</div>
										);
									})}
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</Section>
	);
};

export default HowItWorks;
