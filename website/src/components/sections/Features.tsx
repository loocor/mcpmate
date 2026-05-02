import {
	ArrowRight,
	ClipboardPaste,
	Puzzle,
	RefreshCcw,
	Search,
	Server,
	ShoppingCart,
	Sparkles,
	Terminal,
	Zap,
} from "lucide-react";
import type { ReactNode } from "react";
import { useNavigate } from "react-router-dom";
import { useLanguage } from "../LanguageProvider";
import Card from "../ui/Card";
import Section from "../ui/Section";

interface FeatureCardProps {
	title: string;
	description: string;
	icon: ReactNode;
	docPath: string;
	ctaLabel: string;
	onOpen: (path: string) => void;
}

const FeatureCard = ({
	title,
	description,
	icon,
	docPath,
	ctaLabel,
	onOpen,
}: FeatureCardProps) => {
	return (
		<Card hoverEffect className="h-full">
			<div className="p-6 h-full flex flex-col">
				<div className="w-12 h-12 flex items-center justify-center rounded-lg bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 mb-4">
					{icon}
				</div>
				<h3 className="text-xl font-semibold mb-2">{title}</h3>
				<p className="text-slate-600 dark:text-slate-400 flex-1">{description}</p>
				<button
					type="button"
					onClick={() => onOpen(docPath)}
					className="mt-5 inline-flex items-center gap-1 text-sm font-medium text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 underline transition-colors"
				>
					{ctaLabel}
					<ArrowRight size={14} />
				</button>
			</div>
		</Card>
	);
};

const Features = () => {
	const { t, language } = useLanguage();
	const navigate = useNavigate();
	const locale = language === "zh" ? "zh" : language === "ja" ? "ja" : "en";
	const featureDocsBase = `/docs/${locale}`;
	const openDoc = (path: string) => navigate(path);
	const features = [
		{
			title: t("features.centralized"),
			description: t("features.centralized.desc"),
			icon: <Server size={24} />,
			docPath: `${featureDocsBase}/centralized-config`,
		},
		{
			title: t("features.marketplace"),
			description: t("features.marketplace.desc"),
			icon: <ShoppingCart size={24} />,
			docPath: `${featureDocsBase}/marketplace`,
		},
		{
			title: t("features.context"),
			description: t("features.context.desc"),
			icon: <RefreshCcw size={24} />,
			docPath: `${featureDocsBase}/context-switching`,
		},
		{
			title: t("features.autodiscovery"),
			description: t("features.autodiscovery.desc"),
			icon: <Sparkles size={24} />,
			docPath: `${featureDocsBase}/auto-discovery`,
		},
		{
			title: t("features.uniimport"),
			description: t("features.uniimport.desc"),
			icon: <ClipboardPaste size={24} />,
			docPath: `${featureDocsBase}/uni-import`,
		},
		{
			title: t("features.templates"),
			description: t("features.templates.desc"),
			icon: <Puzzle size={24} />,
			docPath: `${featureDocsBase}/granular-controls`,
		},
		{
			title: t("features.inspector"),
			description: t("features.inspector.desc"),
			icon: <Search size={24} />,
			docPath: `${featureDocsBase}/inspector`,
		},
		{
			title: t("features.resource"),
			description: t("features.resource.desc"),
			icon: <Zap size={24} />,
			docPath: `${featureDocsBase}/resource-optimization`,
		},
		{
			title: t("features.bridge"),
			description: t("features.bridge.desc"),
			icon: <Terminal size={24} />,
			docPath: `${featureDocsBase}/protocol-bridging`,
		},
	];

	return (
		<Section
			title={t("features.title")}
			titleClassName="text-4xl"
			subtitle={t("features.subtitle")}
			centered
			id="features"
			className="border-t border-slate-200/70 dark:border-slate-800/60"
		>
			<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
				{features.map((feature) => (
					<FeatureCard
						key={feature.docPath}
						title={feature.title}
						description={feature.description}
						icon={feature.icon}
						docPath={feature.docPath}
						ctaLabel={t("features.read_more")}
						onOpen={openDoc}
					/>
				))}
			</div>
			<div className="mt-12 text-center">
				<button
					type="button"
					onClick={() => navigate(`${featureDocsBase}/features-overview`)}
					className="inline-flex items-center gap-1 text-sm font-medium text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 underline transition-colors"
				>
					{t("features.explore_all")}
					<ArrowRight size={14} />
				</button>
			</div>
		</Section>
	);
};

export default Features;
