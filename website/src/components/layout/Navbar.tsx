import { Menu, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import logoImage from "../../assets/images/logo.svg";
import { trackMCPMateEvents } from "../../utils/analytics";
import { useLanguage } from "../LanguageProvider";
import { useTheme } from "../ThemeProvider";

const Navbar = () => {
	useTheme();
    const { t, language } = useLanguage();
	const navigate = useNavigate();
	const location = useLocation();
	const [isOpen, setIsOpen] = useState(false);
	const [scrolled, setScrolled] = useState(false);

	const toggleMenu = () => {
		setIsOpen(!isOpen);
	};

	useEffect(() => {
		const handleScroll = () => {
			setScrolled(window.scrollY > 10);
		};

		window.addEventListener("scroll", handleScroll);
		return () => {
			window.removeEventListener("scroll", handleScroll);
		};
	}, []);

	const scrollToSection = (id: string) => {
		// track navigation click event
		trackMCPMateEvents.navClick(id);

		// If not on homepage or the element is not present yet, navigate with query param
		const element = document.getElementById(id);
		if (location.pathname !== "/" || !element) {
			navigate(`/?section=${encodeURIComponent(id)}`);
			setIsOpen(false);
			return;
		}

		const offset = 80; // Account for fixed header
		const elementPosition = element.getBoundingClientRect().top;
		const offsetPosition = elementPosition + window.pageYOffset - offset;

		window.scrollTo({
			top: offsetPosition,
			behavior: "smooth",
		});
		setIsOpen(false);
	};

	// reserved for external links if needed in future

	return (
		<header
			className={`fixed left-0 right-0 z-50 transition-all duration-300 ${
				scrolled
					? "py-3 bg-white/90 dark:bg-slate-900/95 shadow-md backdrop-blur-md"
					: "py-5 bg-transparent"
			}`}
			style={{ top: "var(--banner-height, 0px)" }}
		>
			<div className="container mx-auto px-4 md:px-0">
				<div className="flex items-center justify-between">
					<button
						onClick={() => scrollToSection("hero")}
						className="flex items-center gap-2"
					>
						<img
							src={logoImage}
							alt="MCPMate Logo"
							className="h-8 w-8 dark:invert dark:brightness-0"
						/>
						<span className="text-xl font-bold tracking-tight">MCPMate</span>
					</button>

					<nav className="hidden md:flex items-center space-x-8">
						<button
							onClick={() => scrollToSection("why")}
							className="text-sm font-medium transition-colors hover:text-blue-600 dark:hover:text-blue-400"
						>
							{t("nav.why")}
						</button>
                    <button
                        onClick={() => scrollToSection("features")}
                        className="text-sm font-medium transition-colors hover:text-blue-600 dark:hover:text-blue-400"
                    >
                        {t("nav.features")}
                    </button>
                    <button
                        onClick={() => {
                            navigate(`/docs/${language}/quickstart`);
                        }}
                        className="text-sm font-medium transition-colors hover:text-blue-600 dark:hover:text-blue-400"
                    >
                        {t("nav.documentation")}
                    </button>
						<button
							onClick={() => scrollToSection("faq")}
							className="text-sm font-medium transition-colors hover:text-blue-600 dark:hover:text-blue-400"
						>
							{t("nav.faq")}
						</button>
						<button
							onClick={() => scrollToSection("contact")}
							className="text-sm font-medium transition-colors hover:text-blue-600 dark:hover:text-blue-400"
						>
							{t("nav.contact")}
						</button>
						<button
							onClick={() => scrollToSection("download")}
							className="px-4 py-2 rounded-lg bg-blue-600 text-white hover:bg-blue-700 transition-colors"
						>
							{t("nav.download")}
						</button>
					</nav>

					<div className="flex items-center md:hidden gap-4">
						<button
							onClick={toggleMenu}
							className="p-2 rounded-lg text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
							aria-label="Toggle menu"
						>
							{isOpen ? <X size={24} /> : <Menu size={24} />}
						</button>
					</div>
				</div>
			</div>

			<div
				className={`fixed inset-0 top-[60px] bg-white dark:bg-slate-900 md:hidden transition-transform duration-300 transform ${
					isOpen ? "translate-x-0" : "translate-x-full"
				}`}
			>
				<nav className="flex flex-col p-4 space-y-4">
					<button
						onClick={() => scrollToSection("why")}
						className="p-3 rounded-lg text-center font-medium transition-colors hover:bg-slate-100 dark:hover:bg-slate-800"
					>
						{t("nav.why")}
					</button>
					<button
						onClick={() => scrollToSection("features")}
						className="p-3 rounded-lg text-center font-medium transition-colors hover:bg-slate-100 dark:hover:bg-slate-800"
					>
						{t("nav.features")}
					</button>
					<button
						onClick={() => scrollToSection("faq")}
						className="p-3 rounded-lg text-center font-medium transition-colors hover:bg-slate-100 dark:hover:bg-slate-800"
					>
						{t("nav.faq")}
					</button>
					<button
						onClick={() => scrollToSection("contact")}
						className="p-3 rounded-lg text-center font-medium transition-colors hover:bg-slate-100 dark:hover:bg-slate-800"
					>
						{t("nav.contact")}
					</button>
					<button
						onClick={() => scrollToSection("download")}
						className="p-3 rounded-lg text-center bg-blue-600 text-white hover:bg-blue-700 transition-colors"
					>
						{t("nav.download")}
					</button>
				</nav>
			</div>
		</header>
	);
};

export default Navbar;
