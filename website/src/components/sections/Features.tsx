import {
	ArrowRight,
	Eye,
	LayoutGrid,
	RefreshCcw,
	Server,
	SlidersHorizontal,
} from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { useNavigate } from "react-router-dom";
import { scrollToMarketingSection } from "../../lib/section-scroll";
import { useLanguage } from "../LanguageProvider";
import Section from "../ui/Section";

interface PillarCardProps {
	id: string;
	title: string;
	description: string;
	icon: ReactNode;
	videoSrc: string;
	ctaLabel: string;
	onAction: () => void;
	isPreviewActive: boolean;
	isPreviewToggleEnabled: boolean;
	onPreviewToggle: (id: string) => void;
}

function getDocsLocale(language: string): "en" | "ja" | "zh" {
	if (language === "zh" || language === "ja") {
		return language;
	}

	return "en";
}

function usePreviewToggleEnabled(): boolean {
	const [enabled, setEnabled] = useState(false);

	useEffect(() => {
		if (typeof window === "undefined") {
			return;
		}

		const query = window.matchMedia("(hover: none), (pointer: coarse)");
		const update = () => setEnabled(query.matches);
		update();
		query.addEventListener("change", update);

		return () => {
			query.removeEventListener("change", update);
		};
	}, []);

	return enabled;
}

const PillarCard = ({
	id,
	title,
	description,
	icon,
	videoSrc,
	ctaLabel,
	onAction,
	isPreviewActive,
	isPreviewToggleEnabled,
	onPreviewToggle,
}: PillarCardProps) => {
	const activeClass = isPreviewActive ? "is-preview-active" : "";
	const interactiveClass = isPreviewToggleEnabled ? "cursor-pointer" : "";

	const handlePreviewClick = () => {
		if (isPreviewToggleEnabled) {
			onPreviewToggle(id);
		}
	};

	return (
		<article
			onClick={handlePreviewClick}
			className={`feature-card glass-card-hover group/feature relative flex min-h-[17rem] flex-col overflow-hidden rounded-2xl p-5 transition-[border-color,box-shadow,transform] duration-300 ease-out ${activeClass} ${interactiveClass}`}
		>
			<div
				className="feature-card__media pointer-events-none absolute inset-x-0 top-0 h-[60%] overflow-hidden rounded-t-2xl border-b border-brand-border-subtle bg-brand-overlay opacity-0 shadow-glow-sm [clip-path:inset(0_0_100%_0)] transition-[clip-path,opacity] duration-500 ease-[cubic-bezier(0.16,1,0.3,1)] group-hover/feature:opacity-100 group-hover/feature:[clip-path:inset(0_0_0_0)] group-focus-within/feature:opacity-100 group-focus-within/feature:[clip-path:inset(0_0_0_0)]"
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
				<div className="feature-card__icon mb-4 flex h-11 w-11 items-center justify-center rounded-xl bg-brand-overlay-strong text-brand-indigo ring-1 ring-brand-border-subtle transition-[opacity,transform] duration-300 ease-out group-hover/feature:-translate-y-3 group-hover/feature:scale-75 group-hover/feature:opacity-0 group-focus-within/feature:-translate-y-3 group-focus-within/feature:scale-75 group-focus-within/feature:opacity-0">
					{icon}
				</div>
				<h3 className="feature-card__title mb-2 text-lg font-semibold text-brand-foreground transition-transform duration-500 ease-[cubic-bezier(0.16,1,0.3,1)] group-hover/feature:translate-y-[7.25rem] group-focus-within/feature:translate-y-[7.25rem]">
					{title}
				</h3>
				<div className="feature-card__body flex flex-1 flex-col">
					<p className="feature-card__description flex-1 text-sm leading-relaxed section-muted transition-[opacity,transform] duration-500 ease-[cubic-bezier(0.16,1,0.3,1)] group-hover/feature:translate-y-14 group-hover/feature:opacity-0 group-focus-within/feature:translate-y-14 group-focus-within/feature:opacity-0">
						{description}
					</p>
					<button
						type="button"
						onClick={(event) => {
							event.stopPropagation();
							onAction();
						}}
						className="feature-card__cta mt-5 inline-flex w-fit items-center gap-1 text-sm font-medium text-brand-accent transition-colors hover:text-brand-accent-hover focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-accent focus-visible:ring-offset-2 focus-visible:ring-offset-brand-bg"
					>
						{ctaLabel}
						<ArrowRight size={14} aria-hidden />
					</button>
				</div>
			</div>

			<div
				className="feature-card__shade pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-500 group-hover/feature:opacity-100 group-focus-within/feature:opacity-100"
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
	const previewToggleEnabled = usePreviewToggleEnabled();
	const [activePillarId, setActivePillarId] = useState<string | null>(null);
	const locale = getDocsLocale(language);
	const featureDocsBase = `/docs/${locale}`;
	const openDoc = (path: string) => navigate(path);

	const pillars: Array<{
		id: string;
		title: string;
		description: string;
		ctaLabel: string;
		icon: ReactNode;
		videoSrc: string;
		docPath?: string;
		scrollToId?: string;
	}> = [
		{
			id: "configure",
			title: t("features.pillar1.title"),
			description: t("features.pillar1.desc"),
			ctaLabel: t("features.pillar1.cta"),
			icon: <Server size={22} aria-hidden />,
			videoSrc: "/video/features/configure.webm",
			docPath: `${featureDocsBase}/centralized-config`,
		},
		{
			id: "scenarios",
			title: t("features.pillar2.title"),
			description: t("features.pillar2.desc"),
			ctaLabel: t("features.pillar2.cta"),
			icon: <RefreshCcw size={22} aria-hidden />,
			videoSrc: "/video/features/scenarios.webm",
			docPath: `${featureDocsBase}/context-switching`,
		},
		{
			id: "client-tools",
			title: t("features.pillar3.title"),
			description: t("features.pillar3.desc"),
			ctaLabel: t("features.pillar3.cta"),
			icon: <SlidersHorizontal size={22} aria-hidden />,
			videoSrc: "/video/features/client-tools.webm",
			docPath: `${featureDocsBase}/granular-controls`,
		},
		{
			id: "setup-modes",
			title: t("features.pillar4.title"),
			description: t("features.pillar4.desc"),
			ctaLabel: t("features.pillar4.cta"),
			icon: <LayoutGrid size={22} aria-hidden />,
			videoSrc: "/video/features/setup-modes.webm",
			scrollToId: "modes",
		},
		{
			id: "verify",
			title: t("features.pillar5.title"),
			description: t("features.pillar5.desc"),
			ctaLabel: t("features.pillar5.cta"),
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

	const togglePillarPreview = (id: string) => {
		setActivePillarId((currentId) => (currentId === id ? null : id));
	};

	const renderPillar = (pillar: (typeof pillars)[number]) => (
		<PillarCard
			key={pillar.id}
			id={pillar.id}
			title={pillar.title}
			description={pillar.description}
			icon={pillar.icon}
			videoSrc={pillar.videoSrc}
			ctaLabel={pillar.ctaLabel}
			onAction={() => handlePillarAction(pillar)}
			isPreviewActive={previewToggleEnabled && activePillarId === pillar.id}
			isPreviewToggleEnabled={previewToggleEnabled}
			onPreviewToggle={togglePillarPreview}
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
