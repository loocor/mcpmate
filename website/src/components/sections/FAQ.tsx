import Section from "../ui/Section";
import { useLanguage } from "../LanguageProvider";

const FAQSection = () => {
	const { t } = useLanguage();

	const faqItems = [
		["faq.different.title", "faq.different.answer"],
		["faq.compatible.title", "faq.compatible.answer"],
		["faq.expiry.title", "faq.expiry.answer"],
		["faq.platforms.title", "faq.platforms.answer"],
		["faq.security.title", "faq.security.answer"],
		["faq.privacy.title", "faq.privacy.answer"],
		["faq.updates.title", "faq.updates.answer"],
		["faq.opensource.title", "faq.opensource.answer"],
	] as const;

	return (
		<Section id="faq" className="bg-slate-50 dark:bg-slate-800/40 border-t border-slate-200/70 dark:border-slate-700/50">
			<div className="max-w-3xl mx-auto">
				<h2 className="text-3xl md:text-4xl font-bold text-center mb-10">
					{t("faq.title")}
				</h2>

				<div className="divide-y divide-slate-200 dark:divide-slate-700 rounded-2xl overflow-hidden border-2 border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 shadow-lg">
					{faqItems.map(([titleKey, answerKey]) => (
						<details
							key={titleKey}
							className="group hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors"
						>
							<summary className="cursor-pointer select-none px-6 py-4 text-left text-slate-900 dark:text-slate-100 font-semibold flex items-center justify-between">
								<span>{t(titleKey)}</span>
								<span className="ml-2 text-blue-600 dark:text-blue-400 group-open:rotate-180 transition-transform text-xl">
									▾
								</span>
							</summary>
							<div className="px-6 pb-5 text-slate-600 dark:text-slate-400 leading-relaxed">
								{t(answerKey)}
							</div>
						</details>
					))}
				</div>
			</div>
		</Section>
	);
};

export default FAQSection;
