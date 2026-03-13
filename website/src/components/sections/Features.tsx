import {
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
import { useLanguage } from "../LanguageProvider";
import { useNavigate } from "react-router-dom";
import Card from "../ui/Card";
import Section from "../ui/Section";

interface FeatureCardProps {
	title: string;
	description: string;
	icon: ReactNode;
}

const FeatureCard = ({ title, description, icon }: FeatureCardProps) => {
    return (
        <Card hoverEffect className="h-full">
            <div className="p-6">
                <div className="w-12 h-12 flex items-center justify-center rounded-lg bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 mb-4">
                    {icon}
                </div>
                <h3 className="text-xl font-semibold mb-2">{title}</h3>
                <p className="text-slate-600 dark:text-slate-400">{description}</p>
            </div>
        </Card>
    );
};

const Features = () => {
    const { t, language } = useLanguage();
    const navigate = useNavigate();
    const features = [
        // 基础 → 进阶 → 技术向（递进式）
        {
            title: t("features.centralized"),
            description: t("features.centralized.desc"),
            icon: <Server size={24} />,
        },
        {
            title: t("features.marketplace"),
            description: t("features.marketplace.desc"),
            icon: <ShoppingCart size={24} />,
        },
        {
            title: t("features.context"),
            description: t("features.context.desc"),
            icon: <RefreshCcw size={24} />,
        },
        {
            title: t("features.autodiscovery"),
            description: t("features.autodiscovery.desc"),
            icon: <Sparkles size={24} />,
        },
        {
            title: t("features.uniimport"),
            description: t("features.uniimport.desc"),
            icon: <ClipboardPaste size={24} />,
        },
        {
            title: t("features.templates"),
            description: t("features.templates.desc"),
            icon: <Puzzle size={24} />,
        },
        {
            title: t("features.inspector"),
            description: t("features.inspector.desc"),
            icon: <Search size={24} />,
        },
        {
            title: t("features.resource"),
            description: t("features.resource.desc"),
            icon: <Zap size={24} />,
        },
        {
            title: t("features.bridge"),
            description: t("features.bridge.desc"),
            icon: <Terminal size={24} />,
        },
    ];

	return (
		<Section
			title={t("features.title")}
			titleClassName="text-4xl"
			subtitle={t("features.subtitle")}
			centered
			id="features"
			className="bg-slate-50 dark:bg-slate-800/40 border-t border-slate-200/70 dark:border-slate-700/50"
		>
			<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
				{features.map((feature, index) => (
					<FeatureCard
						key={index}
						title={feature.title}
						description={feature.description}
						icon={feature.icon}
					/>
				))}
			</div>
            <div className="mt-12 text-center">
                <button
                    onClick={() => navigate(language === 'zh' ? '/docs/zh/dashboard' : '/docs/en/dashboard')}
                    className="inline-flex items-center gap-1 text-sm font-medium text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 underline transition-colors"
                >
                    {t('features.read_more')}
                    <span aria-hidden>→</span>
                </button>
            </div>
		</Section>
	);
};

export default Features;
