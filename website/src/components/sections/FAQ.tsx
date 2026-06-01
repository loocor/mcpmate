import { BookOpen, ChevronRight, Mail } from "lucide-react";
import { useMemo } from "react";
import SchemaOrg from "../SchemaOrg";
import { useLanguage, type Language } from "../LanguageProvider";
import { buildFAQPage } from "../../utils/schema";
import { trackMCPMateEvents } from "../../utils/analytics";

const CONTACT_MAILTO = "mailto:loocor@gmail.com";

function getQuickstartPath(language: Language): string {
	if (language === "zh") return "/docs/zh/quickstart";
	if (language === "ja") return "/docs/ja/quickstart";
	return "/docs/en/quickstart";
}

type FAQItem = readonly [string, string];

const faqItems: readonly FAQItem[] = [
	["faq.functions.title", "faq.functions.answer"],
	["faq.usage.title", "faq.usage.answer"],
	["faq.vs_clients.title", "faq.vs_clients.answer"],
	["faq.compatible.title", "faq.compatible.answer"],
	["faq.clients.title", "faq.clients.answer"],
	["faq.security.title", "faq.security.answer"],
];

const FAQSection = () => {
	const { t, language } = useLanguage();
	const docsPath = getQuickstartPath(language);

	const schema = useMemo(
		() =>
			buildFAQPage(
				faqItems.map(([questionKey, answerKey]) => ({
					question: t(questionKey),
					answer: t(answerKey),
				})),
			),
		[t],
	);

	return (
		<section id="faq" className="snap-section relative py-16 md:py-20">
			<div className="container relative mx-auto px-4 md:px-6">
				<SchemaOrg schema={schema} />
				<div className="mx-auto max-w-3xl">
					<h2 className="mb-8 text-center text-3xl font-bold text-brand-foreground md:mb-10 md:text-4xl">
						{t("faq.title")}
					</h2>
					<div className="glass-panel divide-y divide-brand-border-subtle overflow-hidden rounded-2xl">
						{faqItems.map(([titleKey, answerKey], index) => (
							<details
								key={titleKey}
								open={index === 0}
								className="group transition-colors hover:bg-brand-overlay"
							>
								<summary className="flex cursor-pointer select-none items-center justify-between px-6 py-4 text-left font-semibold text-brand-foreground focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-brand-accent">
									<span>{t(titleKey)}</span>
									<span
										className="ml-2 text-xl text-brand-accent transition-transform group-open:rotate-180"
										aria-hidden
									>
										▾
									</span>
								</summary>
								<div className="section-muted px-6 pb-5 text-sm leading-relaxed md:text-base">
									{t(answerKey)}
								</div>
							</details>
						))}
					</div>
					<div className="mt-8 text-center md:mt-10">
						<p className="text-sm leading-relaxed section-muted md:text-base">
							{t("faq.lead")}
						</p>
						<div className="mt-5 flex flex-col items-stretch gap-3 sm:flex-row sm:items-center sm:justify-center sm:gap-4">
							<a
								href={CONTACT_MAILTO}
								className="inline-flex items-center justify-center gap-2 rounded-lg bg-brand-accent px-5 py-2.5 text-sm font-semibold text-brand-accent-fg transition-all hover:bg-brand-accent-hover focus:outline-none focus:ring-2 focus:ring-brand-accent focus:ring-offset-2 focus:ring-offset-brand-bg dark:hover:ring-2 dark:hover:ring-white dark:hover:ring-offset-2 dark:hover:ring-offset-brand-bg dark:focus-visible:ring-2 dark:focus-visible:ring-white dark:focus-visible:ring-offset-2 dark:focus-visible:ring-offset-brand-bg"
								onClick={() => trackMCPMateEvents.externalLinkClick(CONTACT_MAILTO)}
							>
								<Mail size={18} aria-hidden />
								{t("faq.cta.contact")}
							</a>
							<a
								href={docsPath}
								target="_blank"
								rel="noopener noreferrer"
								className="inline-flex items-center justify-center gap-1 rounded-lg border border-brand-border-subtle bg-brand-overlay/40 px-5 py-2.5 text-sm font-semibold text-brand-foreground transition-colors hover:bg-brand-overlay"
								onClick={() => trackMCPMateEvents.externalLinkClick(docsPath)}
							>
								<BookOpen size={18} className="text-brand-accent" aria-hidden />
								{t("faq.cta.docs")}
								<ChevronRight size={16} className="text-brand-muted" aria-hidden />
							</a>
						</div>
					</div>
				</div>
			</div>
		</section>
	);
};

export default FAQSection;
