import { useEffect, useMemo } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useLanguage } from "../components/LanguageProvider";
import SchemaOrg from "../components/SchemaOrg";
import ClientLogoWall from "../components/sections/ClientLogoWall";
import ClientModes from "../components/sections/ClientModes";
import FAQSection from "../components/sections/FAQ";
import Features from "../components/sections/Features";
import MarketingAmbientBackground from "../components/marketing/MarketingAmbientBackground";
import Hero from "../components/sections/Hero";
import HowItWorks from "../components/sections/HowItWorks";
import { scrollToMarketingSection, syncMarketingScrollPadding } from "../lib/section-scroll";
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
		syncMarketingScrollPadding();

		const onResize = () => syncMarketingScrollPadding();
		window.addEventListener("resize", onResize);

		return () => {
			window.removeEventListener("resize", onResize);
		};
	}, []);

	useEffect(() => {
		const params = new URLSearchParams(location.search);
		const section = params.get("section");
		if (section) {
			const scroll = () => scrollToMarketingSection(section);
			setTimeout(scroll, 0);
			navigate("/", { replace: true });
		}
	}, [location.search, navigate]);

	return (
		<>
			<MarketingAmbientBackground />
			<div className="relative z-[1]">
				<SchemaOrg schema={schemas} />
				<div id="hero" className="snap-section-hero flex items-start md:items-center">
					<Hero />
				</div>
				<Features />
				<ClientLogoWall />
				<HowItWorks />
				<ClientModes />
				<FAQSection />
			</div>
		</>
	);
};

export default Homepage;
