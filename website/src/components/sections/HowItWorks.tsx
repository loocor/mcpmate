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
	const { t } = useLanguage();
	const [activeNode, setActiveNode] = useState<ActiveNode>("control");

	const currentLabels = {
		accessTitle: t("how.nodes.access.title"),
		accessDesc: t("how.nodes.access.desc"),
		controlTitle: t("how.nodes.control.title"),
		controlDesc: t("how.nodes.control.desc"),
		runtimeTitle: t("how.nodes.runtime.title"),
		runtimeDesc: t("how.nodes.runtime.desc"),
		subAccess: t("how.nodes.access.label"),
		subControl: t("how.nodes.control.label"),
		subRuntime: t("how.nodes.runtime.label"),
		activeHint: t("how.nodes.activeHint"),
		expandHint: t("how.nodes.expandHint"),
		features: {
			access: [
				t("how.nodes.access.feature1"),
				t("how.nodes.access.feature2"),
				t("how.nodes.access.feature3"),
			],
			control: [
				t("how.nodes.control.feature1"),
				t("how.nodes.control.feature2"),
				t("how.nodes.control.feature3"),
			],
			runtime: [
				t("how.nodes.runtime.feature1"),
				t("how.nodes.runtime.feature2"),
				t("how.nodes.runtime.feature3"),
			],
		},
	};

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

							<div className="how-node-shell__meta mt-6 flex w-full items-center justify-between pt-4 text-[11px] font-medium text-brand-accent">
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

							<div className="how-node-shell__meta mt-6 flex w-full items-center justify-between pt-4 text-[11px] font-medium text-brand-indigo">
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

							<div className="how-node-shell__meta mt-6 flex w-full items-center justify-between pt-4 text-[11px] font-medium text-brand-accent">
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
