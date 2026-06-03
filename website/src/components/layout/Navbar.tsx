import { Menu, X } from "lucide-react";
import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useLocation, useNavigate } from "react-router-dom";
import logoImage from "../../assets/images/logo.svg";
import { useScrollSpy } from "../../hooks/useScrollSpy";
import {
	MARKETING_NAV_ITEMS,
	MARKETING_NAV_SECTIONS,
	scrollToMarketingSection,
	type MarketingNavSectionId,
} from "../../lib/section-scroll";
import { trackMCPMateEvents } from "../../utils/analytics";
import { useLanguage } from "../LanguageProvider";
import { useTheme } from "../ThemeProvider";

const NAV_TEXT_LINK_BASE =
	"relative inline-flex items-center border-0 bg-transparent p-0 text-[15px] font-medium leading-none transition-colors pb-1 text-brand-foreground/85 hover:text-brand-accent cursor-pointer xl:text-base";
const NAV_CTA_LINK_CLASS =
	"-mt-1 inline-flex items-center rounded-md bg-brand-accent px-3 py-1.5 text-[15px] font-semibold leading-none text-brand-accent-fg transition-all duration-200 hover:bg-brand-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-accent focus-visible:ring-offset-2 focus-visible:ring-offset-brand-bg dark:hover:text-white dark:hover:ring-2 dark:hover:ring-white dark:hover:ring-offset-2 dark:hover:ring-offset-brand-bg dark:focus-visible:ring-2 dark:focus-visible:ring-white dark:focus-visible:ring-offset-2 dark:focus-visible:ring-offset-brand-bg xl:text-base";

function getNavLinkClass(sectionId: MarketingNavSectionId, activeSection: string | null): string {
	if (activeSection !== sectionId) {
		return NAV_TEXT_LINK_BASE;
	}

	return `${NAV_TEXT_LINK_BASE} text-brand-accent font-semibold after:absolute after:inset-x-0 after:-bottom-0.5 after:h-0.5 after:rounded-full after:bg-brand-accent`;
}

function getMobileNavLinkClass(sectionId: MarketingNavSectionId, activeSection: string | null): string {
	return activeSection === sectionId ? "mobile-nav-link is-active" : "mobile-nav-link";
}

function getLogoClass(isHome: boolean, theme: string): string {
	if (!isHome) {
		return "h-8 w-8 dark:invert dark:brightness-0";
	}

	return theme === "dark" ? "h-8 w-8 brightness-0 invert" : "h-8 w-8";
}

function getHeaderClass(isOpen: boolean, scrolled: boolean, isHome: boolean): string {
	const zIndexClass = isOpen ? "z-[90]" : "z-50";

	if (isOpen) {
		return `fixed left-0 right-0 transition-all duration-300 ${zIndexClass} border-b border-brand-border-subtle bg-brand-bg py-3 shadow-card`;
	}

	if (scrolled || !isHome) {
		return `fixed left-0 right-0 transition-all duration-300 ${zIndexClass} border-b border-brand-border-subtle bg-brand-bg/95 py-3 shadow-card backdrop-blur-md`;
	}

	return `fixed left-0 right-0 transition-all duration-300 ${zIndexClass} bg-transparent py-5`;
}

const Navbar = () => {
	const { t } = useLanguage();
	const { theme } = useTheme();
	const navigate = useNavigate();
	const location = useLocation();
	const [isOpen, setIsOpen] = useState(false);
	const [scrolled, setScrolled] = useState(false);
	const isHome = location.pathname === "/";
	const activeSection = useScrollSpy(MARKETING_NAV_SECTIONS, isHome);

	useEffect(() => {
		const handleScroll = () => {
			setScrolled(window.scrollY > 10);
		};

		window.addEventListener("scroll", handleScroll);
		return () => {
			window.removeEventListener("scroll", handleScroll);
		};
	}, []);

	useEffect(() => {
		setIsOpen(false);
	}, [location.pathname]);

	useEffect(() => {
		if (!isOpen) {
			return;
		}

		const previousOverflow = document.body.style.overflow;
		document.body.style.overflow = "hidden";

		return () => {
			document.body.style.overflow = previousOverflow;
		};
	}, [isOpen]);

	const toggleMenu = () => {
		setIsOpen((open) => !open);
	};

	const scrollToSection = (id: string) => {
		trackMCPMateEvents.navClick(id);

		if (location.pathname !== "/" || !document.getElementById(id)) {
			navigate(`/?section=${encodeURIComponent(id)}`);
			setIsOpen(false);
			return;
		}

		scrollToMarketingSection(id);
		setIsOpen(false);
	};

	const mobileMenu = isOpen
		? createPortal(
				<div
					className={`mobile-nav-overlay ${theme} fixed inset-0 z-[80] lg:hidden`}
					role="dialog"
					aria-modal="true"
					aria-label="Navigation menu"
				>
					<nav className="mobile-nav-overlay__nav">
						{MARKETING_NAV_ITEMS.map(({ id, labelKey }) => (
							<button
								key={id}
								type="button"
								onClick={() => scrollToSection(id)}
								className={getMobileNavLinkClass(id, activeSection)}
							>
								{t(labelKey)}
							</button>
						))}
					</nav>
				</div>,
				document.body,
		)
		: null;

	const logoClass = getLogoClass(isHome, theme);
	const headerClass = getHeaderClass(isOpen, scrolled, isHome);

	return (
		<header
			className={headerClass}
			style={{ top: "var(--banner-height, 0px)" }}
		>
			<div className="container mx-auto px-4 md:px-6">
				<div className="flex items-center justify-between">
					<button
						onClick={() => scrollToSection("hero")}
						className="flex items-center gap-2"
					>
						<img src={logoImage} alt="MCPMate Logo" className={logoClass} />
						<span className="text-xl font-bold tracking-tight text-brand-foreground">MCPMate</span>
					</button>

					<div className="hidden lg:flex items-center gap-5 xl:gap-6">
						<nav className="flex items-center gap-5 xl:gap-6">
							{MARKETING_NAV_ITEMS.map(({ id, labelKey }) => (
								<button
									key={id}
									type="button"
									onClick={() => scrollToSection(id)}
									className={getNavLinkClass(id, activeSection)}
								>
									{t(labelKey)}
								</button>
							))}
						</nav>
						<button
							type="button"
							onClick={() => scrollToSection("hero")}
							className={NAV_CTA_LINK_CLASS}
						>
							{t("nav.cta.start")}
						</button>
					</div>

					<div className="flex items-center gap-2 lg:hidden">
						<button
							onClick={toggleMenu}
							className="p-2 rounded-lg text-brand-foreground hover:bg-brand-overlay-hover transition-colors"
							aria-label="Toggle menu"
						>
							{isOpen ? <X size={24} /> : <Menu size={24} />}
						</button>
					</div>
				</div>
			</div>

			{mobileMenu}
		</header>
	);
};

export default Navbar;
