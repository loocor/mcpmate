import { useEffect } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useLanguage } from "../components/LanguageProvider";
import Architecture from "../components/sections/Architecture";
import ContactSection from "../components/sections/Contact";
import DownloadSection from "../components/sections/Download";
import Features from "../components/sections/Features";
import Hero from "../components/sections/Hero";
import PricingSection from "../components/sections/Pricing";
import ValueProposition from "../components/sections/ValueProposition";

const Homepage = () => {
	const { t, language } = useLanguage();
	const location = useLocation();
	const navigate = useNavigate();

	useEffect(() => {
		document.title = t("site.title");
	}, [language, t]);

	// Handle cross-route section scrolls from navbar/footer
	useEffect(() => {
		const params = new URLSearchParams(location.search);
		const section = params.get("section");
		if (section) {
			const scroll = () => {
				const el = document.getElementById(section);
				if (el) {
					const offset = 80;
					const top =
						el.getBoundingClientRect().top + window.pageYOffset - offset;
					window.scrollTo({ top, behavior: "smooth" });
				}
			};
			// Wait one tick to ensure sections are rendered
			setTimeout(scroll, 0);
			// Clean the query to avoid repeated scrolling on re-renders
			navigate("/", { replace: true });
		}
	}, [location.search, navigate]);

	return (
		<div>
			<div id="hero">
				<Hero />
			</div>
			<ValueProposition />
			<Features />
			<div id="download">
				<DownloadSection />
			</div>
			<PricingSection />
			<Architecture />
			<div id="contact">
				<ContactSection />
			</div>
		</div>
	);
};

export default Homepage;
