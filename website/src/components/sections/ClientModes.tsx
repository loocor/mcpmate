import { CheckCircle2 } from "lucide-react";
import { useLanguage } from "../LanguageProvider";
import Section from "../ui/Section";

const modeCards = [
	{
		titleKey: "modes.unify.title",
		taglineKey: "modes.unify.tagline",
		choiceKey: "modes.unify.choice",
		descKey: "modes.unify.desc",
		bulletKeys: ["modes.unify.b1", "modes.unify.b2", "modes.unify.b3"],
		accent: "border-brand-indigo/30",
		iconClass: "text-brand-indigo bg-brand-indigo/10",
	},
	{
		titleKey: "modes.hosted.title",
		taglineKey: "modes.hosted.tagline",
		choiceKey: "modes.hosted.choice",
		descKey: "modes.hosted.desc",
		bulletKeys: ["modes.hosted.b1", "modes.hosted.b2", "modes.hosted.b3"],
		accent: "border-brand-accent/30 shadow-glow-sm",
		iconClass: "text-brand-accent bg-brand-accent/10",
	},
	{
		titleKey: "modes.transparent.title",
		taglineKey: "modes.transparent.tagline",
		choiceKey: "modes.transparent.choice",
		descKey: "modes.transparent.desc",
		bulletKeys: ["modes.transparent.b1", "modes.transparent.b2", "modes.transparent.b3"],
		accent: "border-brand-border",
		iconClass: "text-brand-muted bg-brand-overlay",
	},
] as const;

const ClientModes = () => {
	const { t } = useLanguage();

	return (
		<Section
			id="modes"
			title={t("modes.title")}
			subtitle={t("modes.subtitle")}
			centered
			snap
			titleClassName="text-3xl md:text-4xl text-brand-foreground"
			subtitleClassName="section-muted"
		>
			<div className="grid grid-cols-1 gap-6 lg:grid-cols-3 lg:items-stretch">
				{modeCards.map((mode) => (
					<article
						key={mode.titleKey}
						className={`glass-card-hover flex h-full flex-col rounded-2xl p-6 ${mode.accent}`}
					>
						<div className="mb-4 flex items-start justify-between gap-3">
							<div>
								<h3 className="text-xl font-semibold text-brand-foreground">{t(mode.titleKey)}</h3>
								<p className="mt-1 text-sm font-medium text-brand-accent">{t(mode.taglineKey)}</p>
							</div>
						</div>
						<p className="flex-1 text-sm leading-relaxed section-muted">
							<strong className="font-semibold text-brand-foreground underline decoration-brand-accent/50 underline-offset-4">
								{t(mode.choiceKey)}
							</strong>{" "}
							{t(mode.descKey)}
						</p>
						<ul className="mt-5 shrink-0 space-y-2">
							{mode.bulletKeys.map((bulletKey) => (
								<li key={bulletKey} className="flex items-start gap-2 text-sm section-muted">
									<CheckCircle2 className={`mt-0.5 h-4 w-4 shrink-0 ${mode.iconClass.split(" ")[0]}`} aria-hidden />
									<span>{t(bulletKey)}</span>
								</li>
							))}
						</ul>
					</article>
				))}
			</div>
		</Section>
	);
};

export default ClientModes;
