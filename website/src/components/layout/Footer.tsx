import { Github, Mail, Twitter } from "lucide-react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import logoImage from "../../assets/images/logo.svg";
import { BROWSER_EXTENSION_LINKS } from "../../lib/browser-extensions";
import { getMarketingScrollPadding, syncMarketingScrollPadding } from "../../lib/section-scroll";
import { trackMCPMateEvents } from "../../utils/analytics";
import { useLanguage } from "../LanguageProvider";
import { useTheme } from "../ThemeProvider";
import LanguageSwitcher from "./LanguageSwitcher";
import ThemeSwitcher from "./ThemeSwitcher";

const IconDiscord = ({ size = 20, className = "" }: { size?: number; className?: string }) => (
	<svg
		xmlns="http://www.w3.org/2000/svg"
		role="img"
		viewBox="0 0 24 24"
		width={size}
		height={size}
		fill="none"
		stroke="currentColor"
		strokeWidth={2}
		strokeLinecap="round"
		strokeLinejoin="round"
		aria-hidden="true"
		className={className}
	>
		<path d="M20.317 4.369a19.791 19.791 0 00-4.885-1.515.074.074 0 00-.079.037c-.211.375-.444.864-.608 1.249-1.843-.276-3.68-.276-5.486 0-.164-.393-.405-.874-.617-1.249a.077.077 0 00-.079-.037A19.736 19.736 0 003.683 4.37a.07.07 0 00-.032.027C.533 9.046-.319 13.583.099 18.057a.082.082 0 00.031.056 19.9 19.9 0 005.995 3.03.077.077 0 00.084-.027c.461-.63.873-1.295 1.226-1.994a.076.076 0 00-.041-.105c-.652-.247-1.274-.549-1.852-.892a.077.077 0 01-.008-.127c.125-.094.25-.192.37-.291a.074.074 0 01.077-.01c3.927 1.793 8.18 1.793 12.061 0a.075.075 0 01.078.009c.12.099.244.198.37.292a.077.077 0 01-.006.127 12.298 12.298 0 01-1.853.891.076.076 0 00-.04.106c.36.698.772 1.362 1.225 1.993a.077.077 0 00.084.028 19.876 19.876 0 005.996-3.03.077.077 0 00.031-.055c.5-5.177-.838-9.673-3.549-13.661a.061.061 0 00-.031-.03z" />
		<circle cx="8.2" cy="12.6" r="2" fill="currentColor" stroke="none" />
		<circle cx="15.8" cy="12.6" r="2" fill="currentColor" stroke="none" />
	</svg>
);
const IconFeishu = ({ size = 20, className = "" }: { size?: number; className?: string }) => (
	<svg
		xmlns="http://www.w3.org/2000/svg"
		role="img"
		viewBox="0 0 48 48"
		width={size}
		height={size}
		fill="none"
		stroke="currentColor"
		strokeWidth={3.5}
		strokeLinecap="round"
		strokeLinejoin="round"
		aria-hidden="true"
		className={className}
	>
		<g strokeMiterlimit="10">
			<path d="m45.3 18c-5.4-2.7-12.3-1.6-16.5 2.7-3 2.9-5.9 6.2-9.7 8.1 5.9 2.6 13.8 6.4 19.1.8 1.6-1.6 3.1-5.4 4.2-7.3.7-1.5 1.7-3 2.9-4.2z" />
			<path d="m28.7 20.7c1.6-1.6 3.5-2.7 5.6-3.4-.9-3.4-2.5-6.5-4.7-9.3-.2-.3-.5-.5-.8-.6s-.6-.2-1-.2h-18s-.1 0-.2 0-.1.1-.1.2 0 .1 0 .2 0 .1.1.2c6.2 4.5 11.3 10.3 15 17 1.4-1.3 2.8-2.7 4.2-4.1z" />
			<path d="m38.2 29.5c-2.1 2.3-4.7 3-7.3 2.9-3.1 0-6.4-1.3-9.4-2.6-.8-.4-1.6-.7-2.4-1-.3-.1-.6-.3-.9-.4-5.6-2.8-10.6-6.5-14.9-11.1 0 0-.1 0-.2 0s-.1 0-.2 0c0 0-.1 0-.2.1v.2 16s0 1.3 0 1.3c0 .4 0 .7.3 1.1s.4.6.7.8c4.1 2.7 8.9 4.2 13.8 4.1 4.4 0 8.8-1.2 12.6-3.4 3.6-2.1 6.6-5.1 8.7-8.7-.2.3-.5.6-.7.8z" />
		</g>
	</svg>
);
function getFooterShellClass(isMarketingHome: boolean, isDocPage: boolean): string {
	if (isMarketingHome) {
		return "marketing-footer py-12";
	}

	if (isDocPage) {
		return "border-t border-brand-border-subtle bg-brand-surface py-12";
	}

	return "border-t border-slate-200 bg-slate-100 py-12 dark:border-brand-border-subtle dark:bg-brand-surface";
}

const Footer = () => {
	const { theme } = useTheme();
	const { language, t } = useLanguage();
	const navigate = useNavigate();
	const location = useLocation();
	const isMarketingHome = location.pathname === "/";

	function getLocalePath(page: string): string {
		if (language === "zh") return `/docs/zh/${page}`;
		if (language === "ja") return `/docs/ja/${page}`;
		return `/docs/en/${page}`;
	}

	const changelogPath = getLocalePath("changelog");
	const roadmapPath = getLocalePath("roadmap");

	const isDocPage = location.pathname.startsWith("/docs/");

	const scrollToSection = (id: string) => {
		if (id === "hero") {
			if (location.pathname !== "/") {
				navigate("/");
				return;
			}
			window.scrollTo({ top: 0, behavior: "smooth" });
			return;
		}

		const element = document.getElementById(id);
		if (location.pathname !== "/" || !element) {
			navigate(`/?section=${encodeURIComponent(id)}`);
			return;
		}
		syncMarketingScrollPadding();
		const offset = Math.max(getMarketingScrollPadding(), 80);
		const elementPosition = element.getBoundingClientRect().top;
		const offsetPosition = elementPosition + window.scrollY - offset;
		window.scrollTo({ top: offsetPosition, behavior: "smooth" });
	};

	const footerShellClass = getFooterShellClass(isMarketingHome, isDocPage);

	const headingClass = isMarketingHome || isDocPage
		? "mb-4 text-sm font-semibold uppercase tracking-wider text-brand-foreground/80"
		: "mb-4 text-sm font-medium uppercase tracking-wider text-slate-700 dark:text-brand-foreground/80";

	const linkClass = isMarketingHome || isDocPage
		? "text-sm section-muted transition-colors hover:text-brand-accent"
		: "text-sm text-slate-600 transition-colors hover:text-brand-accent dark:section-muted";

	const brandTextClass = isMarketingHome || isDocPage ? "text-brand-foreground" : "text-slate-900 dark:text-brand-foreground";

	const logoClass = isMarketingHome || isDocPage
		? theme === "dark"
			? "h-6 w-6 brightness-0 invert"
			: "h-6 w-6"
		: "h-6 w-6 dark:invert dark:brightness-0";

	return (
		<footer className={footerShellClass}>
			<div className="container mx-auto px-4 md:px-6">
				<div className="grid grid-cols-1 md:grid-cols-5 gap-8">
					<div className="md:col-span-2">
						<button onClick={() => scrollToSection("hero")} className="flex items-center gap-2">
							<img src={logoImage} alt="MCPMate Logo" className={logoClass} />
							<span className={`text-lg font-bold tracking-tight ${brandTextClass}`}>MCPMate</span>
						</button>
						<p className={`mt-4 text-sm leading-relaxed ${isMarketingHome || isDocPage ? "section-muted" : "text-slate-600 dark:section-muted"}`}>
							{t("footer.description")}
						</p>
						<div className="flex items-center gap-4 mt-6">
							<a
								href="https://www.feishu.cn/"
								target="_blank"
								rel="noopener noreferrer"
								className={`p-2 rounded-lg hover:bg-brand-overlay-hover transition-colors ${brandTextClass}`}
								aria-label="Feishu"
							>
								<IconFeishu />
							</a>
							<a
								href="https://discord.com/channels/1369086293933559838"
								target="_blank"
								rel="noopener noreferrer"
								className={`p-2 rounded-lg hover:bg-brand-overlay-hover transition-colors ${brandTextClass}`}
								aria-label="Discord"
							>
								<IconDiscord />
							</a>
							<a
								href="https://x.com/umatedotai"
								target="_blank"
								rel="noopener noreferrer"
								className={`p-2 rounded-lg hover:bg-brand-overlay-hover transition-colors ${brandTextClass}`}
								aria-label="Twitter"
							>
								<Twitter size={20} />
							</a>
							<a
								href="https://github.com/loocor/mcpmate"
								target="_blank"
								rel="noopener noreferrer"
								className={`p-2 rounded-lg transition-colors ${isMarketingHome ? "hover:bg-brand-overlay-hover text-brand-foreground" : "hover:bg-slate-200 dark:hover:bg-brand-overlay-hover"}`}
								aria-label="GitHub"
							>
								<Github size={20} />
							</a>
							<a
								href="mailto:mcp@umate.ai"
								className={`p-2 rounded-lg hover:bg-brand-overlay-hover transition-colors ${brandTextClass}`}
								aria-label="Email"
							>
								<Mail size={20} />
							</a>
						</div>
					</div>

					<div className="md:col-span-3 grid grid-cols-1 sm:grid-cols-3 gap-8">
						<div>
							<h3 className={headingClass}>{t("footer.product")}</h3>
							<ul className="space-y-3">
								<li>
									<button onClick={() => scrollToSection("features")} className={linkClass}>
										{t("features.title")}
									</button>
								</li>
								<li>
									<button onClick={() => scrollToSection("clients")} className={linkClass}>
										{t("nav.works_with")}
									</button>
								</li>
								<li>
									<button onClick={() => scrollToSection("how-it-works")} className={linkClass}>
										{t("nav.how")}
									</button>
								</li>
								<li>
									<button onClick={() => scrollToSection("modes")} className={linkClass}>
										{t("nav.modes")}
									</button>
								</li>
								<li>
									<button onClick={() => scrollToSection("hero")} className={linkClass}>
										{t("nav.download")}
									</button>
								</li>
							</ul>
						</div>

						<div>
							<h3 className={headingClass}>{t("footer.resources")}</h3>
							<ul className="space-y-3">
								<li>
									<button onClick={() => navigate(getLocalePath("quickstart"))} className={linkClass}>
										{t("footer.documentation")}
									</button>
								</li>
								<li>
									<button onClick={() => navigate(changelogPath)} className={linkClass}>
										{t("footer.changelog")}
									</button>
								</li>
								<li>
									<button onClick={() => navigate(roadmapPath)} className={linkClass}>
										{t("footer.roadmap")}
									</button>
								</li>
								{BROWSER_EXTENSION_LINKS.map((link) => (
									<li key={link.id}>
										<a
											href={link.url}
											target="_blank"
											rel="noopener noreferrer"
											onClick={() => trackMCPMateEvents.externalLinkClick(link.url)}
											className={linkClass}
										>
											{t(link.footerLabelKey)}
										</a>
									</li>
								))}
							</ul>
						</div>

						<div>
							<h3 className={headingClass}>{t("footer.legal")}</h3>
							<ul className="space-y-3">
								<li>
									<Link to="/privacy" className={linkClass}>
										{t("footer.privacy")}
									</Link>
								</li>
								<li>
									<Link to="/terms" className={linkClass}>
										{t("footer.terms")}
									</Link>
								</li>
							</ul>
						</div>
					</div>
				</div>

				<div className={`mt-12 pt-8 flex items-center justify-between border-t ${isMarketingHome ? "border-brand-border" : "border-slate-200 dark:border-brand-border"}`}>
					<p className={`text-sm ${isMarketingHome ? "section-muted-soft" : "text-slate-500 dark:section-muted"}`}>
						{t("footer.copyright")}
					</p>
					<div className="hidden items-center gap-3 sm:flex">
						<ThemeSwitcher />
						<LanguageSwitcher variant="footer" menuPlacement="above" />
					</div>
				</div>
			</div>
		</footer>
	);
};

export default Footer;
