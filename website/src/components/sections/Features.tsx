import {
	ArrowRight,
	Eye,
	LayoutGrid,
	RefreshCcw,
	Server,
	SlidersHorizontal,
} from "lucide-react";
import type { KeyboardEvent, ReactNode } from "react";
import { useNavigate } from "react-router-dom";
import { scrollToMarketingSection } from "../../lib/section-scroll";
import { useLanguage } from "../LanguageProvider";
import Section from "../ui/Section";

interface PillarCardProps {
	title: string;
	description: string;
	icon: ReactNode;
	videoSrc: string;
	ctaLabel: string;
	onAction: () => void;
}

function getDocsLocale(language: string): "en" | "ja" | "zh" {
	if (language === "zh" || language === "ja") {
		return language;
	}

	return "en";
}

const handleCardKeyDown = (event: KeyboardEvent<HTMLElement>, onAction: () => void) => {
	if (event.key !== "Enter" && event.key !== " ") {
		return;
	}

	event.preventDefault();
	onAction();
};

const PillarCard = ({ title, description, icon, videoSrc, ctaLabel, onAction }: PillarCardProps) => {
	return (
		<article
			role="button"
			tabIndex={0}
			aria-label={`${title}: ${ctaLabel}`}
			onClick={onAction}
			onKeyDown={(event) => handleCardKeyDown(event, onAction)}
			className="feature-card glass-card-hover group/feature relative flex min-h-[17rem] cursor-pointer flex-col overflow-hidden rounded-2xl p-5 outline-none transition-[border-color,box-shadow,transform] duration-300 ease-out focus-visible:ring-2 focus-visible:ring-brand-accent focus-visible:ring-offset-2 focus-visible:ring-offset-brand-bg"
		>
			<div
				className="feature-card__media pointer-events-none absolute inset-x-0 top-0 h-[60%] overflow-hidden rounded-t-2xl border-b border-brand-border-subtle bg-brand-overlay opacity-0 shadow-glow-sm [clip-path:inset(0_0_100%_0)] transition-[clip-path,opacity] duration-500 ease-[cubic-bezier(0.16,1,0.3,1)] group-hover/feature:opacity-100 group-hover/feature:[clip-path:inset(0_0_0_0)] group-focus-visible/feature:opacity-100 group-focus-visible/feature:[clip-path:inset(0_0_0_0)]"
				aria-hidden
			>
				<video
					src={videoSrc}
					className="feature-card__video h-full w-full object-cover opacity-90"
					autoPlay
					loop
					muted
					playsInline
					preload="metadata"
					tabIndex={-1}
				/>
				<div className="absolute inset-0 bg-gradient-to-b from-white/10 via-transparent to-brand-bg/20" />
			</div>

			<div className="relative z-10 flex h-full min-h-[14.5rem] flex-col">
				<div className="feature-card__icon mb-4 flex h-11 w-11 items-center justify-center rounded-xl bg-brand-overlay-strong text-brand-indigo ring-1 ring-brand-border-subtle transition-[opacity,transform] duration-300 ease-out group-hover/feature:-translate-y-3 group-hover/feature:scale-75 group-hover/feature:opacity-0 group-focus-visible/feature:-translate-y-3 group-focus-visible/feature:scale-75 group-focus-visible/feature:opacity-0">
					{icon}
				</div>
				<h3 className="feature-card__title mb-2 text-lg font-semibold text-brand-foreground transition-transform duration-500 ease-[cubic-bezier(0.16,1,0.3,1)] group-hover/feature:translate-y-[7.25rem] group-focus-visible/feature:translate-y-[7.25rem]">
					{title}
				</h3>
				<div className="feature-card__body flex flex-1 flex-col">
					<p className="feature-card__description flex-1 text-sm leading-relaxed section-muted transition-[opacity,transform] duration-500 ease-[cubic-bezier(0.16,1,0.3,1)] group-hover/feature:translate-y-14 group-hover/feature:opacity-0 group-focus-visible/feature:translate-y-14 group-focus-visible/feature:opacity-0">
						{description}
					</p>
					<span className="feature-card__cta mt-5 inline-flex items-center gap-1 text-sm font-medium text-brand-accent">
						{ctaLabel}
						<ArrowRight size={14} aria-hidden />
					</span>
				</div>
			</div>

			<div
				className="pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-500 group-hover/feature:opacity-100 group-focus-visible/feature:opacity-100"
				aria-hidden
			>
				<div className="absolute inset-x-0 bottom-0 h-24 bg-gradient-to-t from-brand-elevated/95 to-transparent" />
			</div>
		</article>
	);
};

const Features = () => {
	const { t, language } = useLanguage();
	const navigate = useNavigate();
	const locale = getDocsLocale(language);
	const featureDocsBase = `/docs/${locale}`;
	const openDoc = (path: string) => navigate(path);

	const pillars: Array<{
		title: string;
		description: string;
		icon: ReactNode;
		videoSrc: string;
		docPath?: string;
		scrollToId?: string;
	}> = [
		{
			title: t("features.pillar1.title"),
			description: t("features.pillar1.desc"),
			icon: <Server size={22} aria-hidden />,
			videoSrc: "/video/features/configure.webm",
			docPath: `${featureDocsBase}/centralized-config`,
		},
		{
			title: t("features.pillar2.title"),
			description: t("features.pillar2.desc"),
			icon: <RefreshCcw size={22} aria-hidden />,
			videoSrc: "/video/features/scenarios.webm",
			docPath: `${featureDocsBase}/context-switching`,
		},
		{
			title: t("features.pillar3.title"),
			description: t("features.pillar3.desc"),
			icon: <SlidersHorizontal size={22} aria-hidden />,
			videoSrc: "/video/features/client-tools.webm",
			docPath: `${featureDocsBase}/granular-controls`,
		},
		{
			title: t("features.pillar4.title"),
			description: t("features.pillar4.desc"),
			icon: <LayoutGrid size={22} aria-hidden />,
			videoSrc: "/video/features/setup-modes.webm",
			scrollToId: "modes",
		},
		{
			title: t("features.pillar5.title"),
			description: t("features.pillar5.desc"),
			icon: <Eye size={22} aria-hidden />,
			videoSrc: "/video/features/verify.webm",
			docPath: `${featureDocsBase}/inspector`,
		},
	];

	const handlePillarAction = (pillar: (typeof pillars)[number]) => {
		if (pillar.scrollToId) {
			scrollToMarketingSection(pillar.scrollToId);
			return;
		}
		if (pillar.docPath) {
			openDoc(pillar.docPath);
		}
	};

	const renderPillar = (pillar: (typeof pillars)[number]) => (
		<PillarCard
			key={pillar.title}
			title={pillar.title}
			description={pillar.description}
			icon={pillar.icon}
			videoSrc={pillar.videoSrc}
			ctaLabel={t("features.read_more")}
			onAction={() => handlePillarAction(pillar)}
		/>
	);

	return (
		<Section
			title={t("features.title")}
			titleClassName="text-3xl md:text-4xl text-brand-foreground"
			subtitle={t("features.subtitle")}
			subtitleClassName="section-muted"
			centered
			id="features"
			snap
			className="features-section !py-14 md:!py-16 [@media(max-height:52rem)]:!py-10 [@media(max-height:52rem)]:md:!py-12"
		>
			<div className="features-pillar-grid hidden xl:grid xl:grid-cols-5 xl:gap-4">
				{pillars.map(renderPillar)}
			</div>

			<div className="space-y-4 xl:hidden">
				<div className="grid grid-cols-1 gap-4 lg:grid-cols-3">{pillars.slice(0, 3).map(renderPillar)}</div>
				<div className="mx-auto grid max-w-3xl grid-cols-1 gap-4 lg:grid-cols-2">{pillars.slice(3).map(renderPillar)}</div>
			</div>

			<div className="mt-6 text-center md:mt-8">
				<button
					type="button"
					onClick={() => navigate(`${featureDocsBase}/features-overview`)}
					className="inline-flex items-center gap-1 text-sm font-medium text-brand-accent transition-colors hover:text-brand-accent-hover"
				>
					{t("features.explore_all")}
					<ArrowRight size={14} aria-hidden />
				</button>
			</div>
		</Section>
	);
};

export default Features;
