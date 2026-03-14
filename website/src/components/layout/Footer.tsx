import { Github, Monitor, Moon, Sun, Twitter } from "lucide-react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import logoImage from "../../assets/images/logo.svg";

// Lightweight Discord brand icon (inline SVG), sized like lucide icons and using currentColor
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
    <path d="M20.317 4.369a19.791 19.791 0 00-4.885-1.515.074.074 0 00-.079.037c-.211.375-.444.864-.608 1.249-1.843-.276-3.68-.276-5.486 0-.164-.393-.405-.874-.617-1.249a.077.077 0 00-.079-.037A19.736 19.736 0 003.683 4.37a.07.07 0 00-.032.027C.533 9.046-.319 13.583.099 18.057a.082.082 0 00.031.056 19.9 19.9 0 005.995 3.03.077.077 0 00.084-.027c.461-.63.873-1.295 1.226-1.994a.076.076 0 00-.041-.105c-.652-.247-1.274-.549-1.852-.892a.077.077 0 01-.008-.127c.125-.094.25-.192.37-.291a.074.074 0 01.077-.01c3.927 1.793 8.18 1.793 12.061 0a.075.075 0 01.078.009c.12.099.244.198.37.292a.077.077 0 01-.006.127 12.298 12.298 0 01-1.853.891.076.076 0 00-.04.106c.36.698.772 1.362 1.225 1.993a.077.077 0 00.084.028 19.876 19.876 0 005.996-3.03.077.077 0 00.031-.055c.5-5.177-.838-9.673-3.549-13.661a.061.061 0 00-.031-.03z"/>
    <circle cx="8.2" cy="12.6" r="2" fill="currentColor" stroke="none" />
    <circle cx="15.8" cy="12.6" r="2" fill="currentColor" stroke="none" />
  </svg>
);
import { useLanguage, type Language } from "../LanguageProvider";
import { useTheme } from "../ThemeProvider";

const Footer = () => {
	const { mode, setMode } = useTheme();
	const { language, setLanguage, t } = useLanguage();
	const navigate = useNavigate();
	const location = useLocation();
	const changelogPath =
		language === "zh" ? "/docs/zh/changelog" : "/docs/en/changelog";
	const roadmapPath =
		language === "zh" ? "/docs/zh/roadmap" : "/docs/en/roadmap";

	// Helper function to check if current page is a documentation page
	const isDocPage = (pathname: string): boolean => {
		return pathname.startsWith("/docs/");
	};

	// Helper function to extract page ID from documentation path
	const getDocPageId = (pathname: string): string => {
		const parts = pathname.split("/");
		return parts[parts.length - 1]; // Get the last part (page ID)
	};

	// Enhanced language change handler that handles documentation page navigation
	const handleLanguageChange = (newLang: Language) => {
		setLanguage(newLang);

		// If currently on a documentation page, navigate to the same page in the new language
		if (isDocPage(location.pathname)) {
			const pageId = getDocPageId(location.pathname);
			const newPath = `/docs/${newLang}/${pageId}`;
			navigate(newPath);
		}
	};

	const scrollToSection = (id: string) => {
		const element = document.getElementById(id);
		if (location.pathname !== "/" || !element) {
			navigate(`/?section=${encodeURIComponent(id)}`);
			return;
		}
		const offset = 80;
		const elementPosition = element.getBoundingClientRect().top;
		const offsetPosition = elementPosition + window.pageYOffset - offset;
		window.scrollTo({ top: offsetPosition, behavior: "smooth" });
	};

	return (
		<footer className="bg-slate-100 dark:bg-slate-800/50 py-12">
			<div className="container mx-auto px-4 md:px-6">
				<div className="grid grid-cols-1 md:grid-cols-5 gap-8">
					<div className="md:col-span-2">
						<button
							onClick={() => scrollToSection("hero")}
							className="flex items-center gap-2"
						>
							<img
								src={logoImage}
								alt="MCPMate Logo"
								className="h-6 w-6 dark:invert dark:brightness-0"
							/>
							<span className="text-lg font-bold tracking-tight">MCPMate</span>
						</button>
						<p className="mt-4 text-sm text-slate-600 dark:text-slate-400">
							{t("footer.description")}
						</p>

						<div className="flex items-center gap-4 mt-6">
							<a
									href="https://discord.com/channels/1369086293933559838"
									target="_blank"
									rel="noopener noreferrer"
									className="p-2 rounded-lg hover:bg-slate-200 dark:hover:bg-slate-700 transition-colors"
									aria-label="Discord"
								>
								<IconDiscord />
							</a>
<a
									href="https://x.com/umatedotai"
									target="_blank"
									rel="noopener noreferrer"
									className="p-2 rounded-lg hover:bg-slate-200 dark:hover:bg-slate-700 transition-colors"
									aria-label="Twitter"
								>
								<Twitter size={20} />
							</a>
							<a
								href="https://github.com/loocor/mcpmate"
								target="_blank"
								rel="noopener noreferrer"
								className="p-2 rounded-lg hover:bg-slate-200 dark:hover:bg-slate-700 transition-colors"
								aria-label="GitHub"
							>
								<Github size={20} />
							</a>
						</div>
					</div>

					<div className="md:col-span-3 grid grid-cols-1 sm:grid-cols-4 gap-8">
						<div>
							<h3 className="font-medium text-sm uppercase tracking-wider text-slate-700 dark:text-slate-300 mb-4">
								{t("footer.product")}
							</h3>
							<ul className="space-y-3">
								<li>
									<button
										onClick={() => scrollToSection("why")}
										className="text-sm text-slate-600 hover:text-blue-600 dark:text-slate-400 dark:hover:text-blue-400 transition-colors"
									>
										{t("value.title")}
									</button>
								</li>
								<li>
									<button
										onClick={() => scrollToSection("features")}
										className="text-sm text-slate-600 hover:text-blue-600 dark:text-slate-400 dark:hover:text-blue-400 transition-colors"
									>
										{t("nav.features")}
									</button>
								</li>
								<li>
									<button
										onClick={() => scrollToSection("download")}
										className="text-sm text-slate-600 hover:text-blue-600 dark:text-slate-400 dark:hover:text-blue-400 transition-colors"
									>
										{t("nav.download")}
									</button>
								</li>
							</ul>
						</div>

						<div>
							<h3 className="font-medium text-sm uppercase tracking-wider text-slate-700 dark:text-slate-300 mb-4">
								{t("footer.resources")}
							</h3>
							<ul className="space-y-3">
								<li>
									<button
										onClick={() =>
											navigate(
												language === "zh"
													? "/docs/zh/quickstart"
													: "/docs/en/quickstart",
											)
										}
										className="text-sm text-slate-600 hover:text-blue-600 dark:text-slate-400 dark:hover:text-blue-400 transition-colors"
									>
										{t("footer.documentation")}
									</button>
								</li>
								<li>
									<button
										onClick={() => navigate(changelogPath)}
										className="text-sm text-slate-600 hover:text-blue-600 dark:text-slate-400 dark:hover:text-blue-400 transition-colors"
									>
										{t("footer.changelog")}
									</button>
								</li>
								<li>
									<button
										onClick={() => navigate(roadmapPath)}
										className="text-sm text-slate-600 hover:text-blue-600 dark:text-slate-400 dark:hover:text-blue-400 transition-colors"
									>
										{t("footer.roadmap")}
									</button>
								</li>
							</ul>
						</div>

						<div>
							<h3 className="font-medium text-sm uppercase tracking-wider text-slate-700 dark:text-slate-300 mb-4">
								{t("footer.legal")}
							</h3>
							<ul className="space-y-3">
								<li>
									<Link
										to="/privacy"
										className="text-sm text-slate-600 hover:text-blue-600 dark:text-slate-400 dark:hover:text-blue-400 transition-colors"
									>
										{t("footer.privacy")}
									</Link>
								</li>
								<li>
									<Link
										to="/terms"
										className="text-sm text-slate-600 hover:text-blue-600 dark:text-slate-400 dark:hover:text-blue-400 transition-colors"
									>
										{t("footer.terms")}
									</Link>
								</li>
							</ul>
						</div>

						<div>
							<h3 className="font-medium text-sm uppercase tracking-wider text-slate-700 dark:text-slate-300 mb-4">
								{t("footer.language")}
							</h3>
							<ul className="space-y-3">
								<li>
									<button
										onClick={() => handleLanguageChange("en")}
										className={`group flex items-center gap-2 text-sm ${language === "en" ? "text-blue-600 dark:text-blue-400" : "text-slate-600 hover:text-blue-600 dark:text-slate-400 dark:hover:text-blue-400"} transition-colors`}
									>
										<span
											className={`inline-flex items-center justify-center w-5 h-5 rounded-sm ${language === "en" ? "bg-blue-600" : "bg-slate-600 group-hover:bg-blue-600"} dark:bg-slate-600 dark:group-hover:bg-blue-600 text-white text-xs font-medium transition-colors`}
										>
											EN
										</span>
										<span>English</span>
									</button>
								</li>
								<li>
									<button
										onClick={() => handleLanguageChange("zh")}
										className={`group flex items-center gap-2 text-sm ${language === "zh" ? "text-blue-600 dark:text-blue-400" : "text-slate-600 hover:text-blue-600 dark:text-slate-400 dark:hover:text-blue-400"} transition-colors`}
									>
										<span
											className={`inline-flex items-center justify-center w-5 h-5 rounded-sm ${language === "zh" ? "bg-blue-600" : "bg-slate-600 group-hover:bg-blue-600"} dark:bg-slate-600 dark:group-hover:bg-blue-600 text-white text-xs font-medium transition-colors`}
										>
											中
										</span>
										<span>中文</span>
									</button>
								</li>
							</ul>
						</div>
					</div>
				</div>

				<div className="border-t border-slate-200 dark:border-slate-700 mt-12 pt-8 flex items-center justify-between">
					<p className="text-sm text-slate-500 dark:text-slate-400">
						{t("footer.copyright")}
					</p>
					<div className="flex items-center gap-3">
						{/* Theme Toggle: light / dark / system */}
						<div className="flex items-center gap-1">
							<button
								aria-label="Light theme"
								aria-pressed={mode === "light"}
								className={`inline-flex items-center justify-center w-5 h-5 rounded-lg transition-transform transition-opacity duration-150 hover:scale-110 ${mode === "light" ? "opacity-100 text-blue-600 dark:text-blue-400" : "opacity-60 text-slate-600 dark:text-slate-300 hover:opacity-80"}`}
								onClick={() => setMode("light")}
							>
								<Sun size={14} />
							</button>
							<button
								aria-label="Dark theme"
								aria-pressed={mode === "dark"}
								className={`inline-flex items-center justify-center w-5 h-5 rounded-lg transition-transform transition-opacity duration-150 hover:scale-110 ${mode === "dark" ? "opacity-100 text-blue-600 dark:text-blue-400" : "opacity-60 text-slate-600 dark:text-slate-300 hover:opacity-80"}`}
								onClick={() => setMode("dark")}
							>
								<Moon size={14} />
							</button>
							<button
								aria-label="System theme"
								aria-pressed={mode === "system"}
								className={`inline-flex items-center justify-center w-5 h-5 rounded-lg transition-transform transition-opacity duration-150 hover:scale-110 ${mode === "system" ? "opacity-100 text-blue-600 dark:text-blue-400" : "opacity-60 text-slate-600 dark:text-slate-300 hover:opacity-80"}`}
								onClick={() => setMode("system")}
							>
								<Monitor size={14} />
							</button>
						</div>
					</div>
				</div>
			</div>
		</footer>
	);
};

export default Footer;
