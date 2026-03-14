import { useEffect } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useLanguage } from "../components/LanguageProvider";
import Architecture from "../components/sections/Architecture";
import ContactSection from "../components/sections/Contact";
import DownloadSection from "../components/sections/Download";
import FAQSection from "../components/sections/FAQ";
import Features from "../components/sections/Features";
import Hero from "../components/sections/Hero";
import ValueProposition from "../components/sections/ValueProposition";

const Homepage = () => {
	const { t, language } = useLanguage();
	const location = useLocation();
	const navigate = useNavigate();

	useEffect(() => {
		document.title = t("site.title");
	}, [language, t]);

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
			setTimeout(scroll, 0);
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
			<FAQSection />
			<Architecture />
			<div id="contact">
				<ContactSection />
			</div>
		</div>
	);
};

export default Homepage;
