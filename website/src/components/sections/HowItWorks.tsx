import { useState, type ReactNode } from "react";
import { ArrowRightLeft, MonitorSmartphone } from "lucide-react";
import mcpIcon from "../../assets/images/mcp-icon.svg";
import logoImage from "../../assets/images/logo.svg";
import { useLanguage } from "../LanguageProvider";
import Section from "../ui/Section";

type ActiveNode = "access" | "control" | "runtime";
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
	const [activeNode, setActiveNode] = useState<ActiveNode>("control");

	const labels = {
		zh: {
			accessTitle: "桌面端、API 与浏览器扩展入口",
			accessDesc: "日常使用通过桌面 Web UI，自动化通过 REST API，浏览器扩展负责把发现到的服务带回本地工作空间。",
			controlTitle: "配置集、客户端预设与 UILib 配置模式",
			controlDesc: "MCPMate 将服务整理成可复用的配置集和按客户端分发的预设，让复杂 MCP 配置可以被组合、展示和检查，而不是反复手写。",
			runtimeTitle: "基于 Rust RMCP 的本地运行核心",
			runtimeDesc: "MCPMate 核心使用 Rust RMCP 运行时管理 MCP 连接、能力发现、工具暴露和运行状态。",

			subAccess: "外层入口",
			subControl: "配置控制层",
			subRuntime: "运行核心层",
			activeHint: "详情见下方",
			expandHint: "▼ 点击查看特性",

			features: {
				access: [
					"桌面 Web：让管理界面贴近本地运行时，同时保留 Web UI 的迭代速度",
					"REST API：让脚本、工具和未来集成可以操作同一份 MCPMate 状态",
					"浏览器扩展：缩短从发现服务到导入本地的路径"
				],
				control: [
					"配置集：按开发、写作、调研等任务组织工具集",
					"客户端预设：不复制服务器配置，也能决定每个 AI 应用收到什么",
					"UILib 配置模式：把原始 MCP 配置转成可由 UI 管理的结构化选择"
				],
				runtime: [
					"连接处理：通过支持的传输方式连接本地和远程 MCP 服务",
					"能力发现：先理解可用工具，再把它们暴露给客户端",
					"运行检查：展示就绪状态、绑定关系、调用记录和日志，方便排查"
				]
			},
		},
		en: {
			accessTitle: "Desktop, API, and extension entry points",
			accessDesc: "Use the desktop web UI for daily work, the REST API for automation, and the browser extension to bring discovered servers into your local workspace.",
			controlTitle: "Profiles, client presets, and UILib config mode",
			controlDesc: "MCPMate normalizes servers into reusable profiles and client-specific presets, so complex MCP configuration can be composed and inspected instead of hand-edited.",
			runtimeTitle: "Rust RMCP runtime for local MCP control",
			runtimeDesc: "At the core, MCPMate uses a Rust RMCP-based runtime to manage MCP connections, capability discovery, tool exposure, and runtime state.",

			subAccess: "Access Layer",
			subControl: "Config Layer",
			subRuntime: "Runtime Layer",

			features: {
				access: [
					"Desktop Web: keeps the management UI close to the local runtime while staying fast to iterate",
					"REST API: lets scripts, tools, and future integrations operate the same MCPMate state",
					"Browser Extension: shortens the path from finding a server to importing it locally"
				],
				control: [
					"Profiles: group tool sets by task, such as coding, writing, or research",
					"Client presets: decide what each AI app receives without duplicating server setup",
					"UILib config mode: turns raw MCP configuration into structured UI-managed choices"
				],
				runtime: [
					"Connection handling: keeps local and remote MCP servers reachable through supported transports",
					"Capability discovery: understands available tools before exposing them to clients",
					"Runtime inspection: surfaces readiness, bindings, calls, and logs for troubleshooting"
				]
			},
			activeHint: "Details below",
			expandHint: "▼ View details",
		},
		ja: {
			accessTitle: "デスクトップ、API、ブラウザー拡張の入口",
			accessDesc: "日常作業はデスクトップ Web UI、自動化は REST API、見つけたサーバーの取り込みはブラウザー拡張から行えます。",
			controlTitle: "プロファイル、クライアントプリセット、UILib 設定モード",
			controlDesc: "MCPMate はサーバーを再利用できるプロファイルとクライアント別プリセットに整理し、複雑な MCP 設定を手編集ではなく構成・確認できる形にします。",
			runtimeTitle: "ローカル MCP 制御のための Rust RMCP ランタイム",
			runtimeDesc: "中心では Rust RMCP ベースのランタイムが MCP 接続、能力発見、ツール公開、ランタイム状態を管理します。",

			subAccess: "入口レイヤー",
			subControl: "制御レイヤー",
			subRuntime: "ランタイムレイヤー",
			activeHint: "下に詳細を表示",
			expandHint: "▼ 詳細を見る",

			features: {
				access: [
					"デスクトップ Web: 管理 UI をローカルランタイムの近くに置きながら素早く改善できます",
					"REST API: スクリプト、ツール、将来の連携が同じ MCPMate 状態を操作できます",
					"ブラウザー拡張: サーバー発見からローカルインポートまでの距離を短くします"
				],
				control: [
					"プロファイル: 開発、執筆、調査などのタスクごとにツール群をまとめます",
					"クライアントプリセット: サーバー設定を複製せず、各 AI アプリへ渡す内容を決めます",
					"UILib 設定モード: 生の MCP 設定を UI で管理できる構造化された選択肢にします"
				],
				runtime: [
					"接続処理: 対応するトランスポートでローカルとリモートの MCP サーバーに到達します",
					"能力発見: 利用可能なツールを理解してからクライアントへ公開します",
					"ランタイム確認: 準備状態、接続、呼び出し、ログを表示して調査を助けます"
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
					<HowNodeShell active={activeNode === "access"} tone="accent">
						<button
							type="button"
							onClick={() => setActiveNode("access")}
							className={`${HOW_NODE_CARD_BTN} focus-visible:ring-brand-accent`}
						>
							<div className="absolute top-0 right-0 w-24 h-24 bg-brand-accent/5 rounded-full blur-2xl pointer-events-none transition-opacity duration-300 group-hover:opacity-100 opacity-70" />
							<div>
								<div className="flex items-center gap-3 mb-3">
									<div
										className={`flex h-11 w-11 shrink-0 items-center justify-center rounded-xl transition-colors ${HOW_NODE_ICON_WRAP} ${
											activeNode === "access"
												? "bg-brand-accent/20 text-brand-accent"
												: "bg-brand-accent/10 text-brand-accent/80"
										}`}
									>
										<MonitorSmartphone size={22} aria-hidden />
									</div>
									<span className="text-[10px] font-mono uppercase tracking-wider text-brand-accent/80 font-semibold">
										{currentLabels.subAccess}
									</span>
								</div>
								<h3 className="text-lg font-bold text-brand-foreground mb-2">
									{currentLabels.accessTitle}
								</h3>
								<p className="text-xs section-muted leading-relaxed">
									{currentLabels.accessDesc}
								</p>
							</div>

							<div className="mt-6 pt-4 border-t border-brand-border-subtle/40 flex items-center justify-between w-full text-[11px] font-medium text-brand-accent">
								<span>{activeNode === "access" ? currentLabels.activeHint : currentLabels.expandHint}</span>
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
									className={`transition-colors duration-300 ${activeNode === "access" || activeNode === "control" ? "text-brand-accent" : ""}`}
								/>
								{(activeNode === "access" || activeNode === "control") && (
									<span className="absolute inset-0 bg-brand-accent/20 blur-sm rounded-full animate-ping" />
								)}
							</div>
							<span className="text-[9px] font-mono text-brand-muted-soft">
								Bridge / HTTP
							</span>
						</div>
					</div>

					<HowNodeShell
						active={activeNode === "control"}
						tone="indigo"
						className="md:min-h-[17.5rem]"
					>
						<button
							type="button"
							onClick={() => setActiveNode("control")}
							className={`${HOW_NODE_CARD_BTN} md:py-8 focus-visible:ring-brand-indigo`}
						>
							<div className="absolute top-0 right-0 w-32 h-32 bg-brand-indigo/5 rounded-full blur-3xl pointer-events-none transition-opacity duration-300 group-hover:opacity-100 opacity-70" />
							<div>
								<div className="flex items-center gap-3 mb-3">
									<div
										className={`flex h-12 w-12 shrink-0 items-center justify-center rounded-xl transition-colors ${HOW_NODE_ICON_WRAP} ${
											activeNode === "control"
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
										{currentLabels.subControl}
									</span>
								</div>
								<h3 className="mb-2 text-lg font-bold leading-tight text-brand-foreground">
									{currentLabels.controlTitle}
								</h3>
								<p className="text-xs section-muted leading-relaxed">
									{currentLabels.controlDesc}
								</p>
							</div>

							<div className="mt-6 pt-4 border-t border-brand-border-subtle/40 flex items-center justify-between w-full text-[11px] font-medium text-brand-indigo">
								<span>{activeNode === "control" ? currentLabels.activeHint : currentLabels.expandHint}</span>
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
									className={`transition-colors duration-300 ${activeNode === "runtime" || activeNode === "control" ? "text-brand-indigo" : ""}`}
								/>
								{(activeNode === "runtime" || activeNode === "control") && (
									<span className="absolute inset-0 bg-brand-indigo/20 blur-sm rounded-full animate-ping" />
								)}
							</div>
							<span className="text-[9px] font-mono text-brand-muted-soft">
								stdio / HTTP
							</span>
						</div>
					</div>

					<HowNodeShell active={activeNode === "runtime"} tone="accent">
						<button
							type="button"
							onClick={() => setActiveNode("runtime")}
							className={`${HOW_NODE_CARD_BTN} focus-visible:ring-brand-accent`}
						>
							<div className="absolute top-0 right-0 w-24 h-24 bg-brand-accent/5 rounded-full blur-2xl pointer-events-none transition-opacity duration-300 group-hover:opacity-100 opacity-70" />
							<div>
								<div className="flex items-center gap-3 mb-3">
									<div
										className={`flex h-11 w-11 shrink-0 items-center justify-center rounded-xl transition-colors ${HOW_NODE_ICON_WRAP} ${
											activeNode === "runtime"
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
										{currentLabels.subRuntime}
									</span>
								</div>
								<h3 className="text-lg font-bold text-brand-foreground mb-2">
									{currentLabels.runtimeTitle}
								</h3>
								<p className="text-xs section-muted leading-relaxed">
									{currentLabels.runtimeDesc}
								</p>
							</div>

							<div className="mt-6 pt-4 border-t border-brand-border-subtle/40 flex items-center justify-between w-full text-[11px] font-medium text-brand-accent">
								<span>{activeNode === "runtime" ? currentLabels.activeHint : currentLabels.expandHint}</span>
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
									activeNode === "control" ? "bg-brand-indigo" : "bg-brand-accent"
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
											activeNode === "control" ? "text-brand-indigo" : "text-brand-accent"
										}`}
									>
										{activeNode === "access" && currentLabels.subAccess}
										{activeNode === "control" && currentLabels.subControl}
										{activeNode === "runtime" && currentLabels.subRuntime}
									</p>
									<h4 className="m-0 w-full text-left text-xl font-bold leading-tight text-brand-foreground">
										{activeNode === "access" && currentLabels.accessTitle}
										{activeNode === "control" && currentLabels.controlTitle}
										{activeNode === "runtime" && currentLabels.runtimeTitle}
									</h4>
									<p className="m-0 w-full text-left text-sm leading-relaxed section-muted">
										{activeNode === "access" && currentLabels.accessDesc}
										{activeNode === "control" && currentLabels.controlDesc}
										{activeNode === "runtime" && currentLabels.runtimeDesc}
									</p>
								</div>

								<div className="space-y-3.5 border-t border-brand-border-subtle/50 pt-6 md:border-t-0 md:pt-0 md:pl-10">
									{currentLabels.features[activeNode].map((feat, idx) => {
										const [title, desc] = feat.split(/[:：]/, 2);
										return (
											<div key={idx} className="flex items-start gap-3">
												<div className={`mt-1 flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-xs font-bold ${
													activeNode === "control"
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
