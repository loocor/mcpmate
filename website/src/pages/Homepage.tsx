import { useEffect, useMemo } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useLanguage } from "../components/LanguageProvider";
import SchemaOrg from "../components/SchemaOrg";
import Architecture from "../components/sections/Architecture";
import ContactSection from "../components/sections/Contact";
import DownloadSection from "../components/sections/Download";
import FAQSection from "../components/sections/FAQ";
import Features from "../components/sections/Features";
import Hero from "../components/sections/Hero";
import ValueProposition from "../components/sections/ValueProposition";
import { setDocumentMeta } from "../utils/seo";
import { buildOrganization, buildSoftwareApplication } from "../utils/schema";

const Homepage = () => {
	const { t, language } = useLanguage();
	const location = useLocation();
	const navigate = useNavigate();

	const schemas = useMemo(
		() => [
			buildSoftwareApplication({
				name: "MCPMate",
				description: t("site.description"),
			}),
			buildOrganization(),
		],
		[t],
	);

	useEffect(() => {
		setDocumentMeta({
			title: t("site.title"),
			description: t("site.description"),
			pathname: "/",
		});
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
			<SchemaOrg schema={schemas} />
			<div id="hero">
				<Hero />
			</div>
			<div className="bg-white dark:bg-slate-900">
				<div id="download">
					<DownloadSection />
				</div>
				<ValueProposition />
				<Features />
				<Architecture />
				<FAQSection />
				<div id="contact">
					<ContactSection />
				</div>
			</div>
		</div>
	);
};

export default Homepage;
