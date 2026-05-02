import { useMemo } from "react";
import Section from "../ui/Section";
import SchemaOrg from "../SchemaOrg";
import { useLanguage } from "../LanguageProvider";
import { buildFAQPage } from "../../utils/schema";

type FAQItem = readonly [string, string];

type FAQSectionGroup = {
	headingKey: string;
	items: readonly FAQItem[];
};

const faqSections: readonly FAQSectionGroup[] = [
	{
		headingKey: "faq.group.basics",
		items: [
			["faq.functions.title", "faq.functions.answer"],
			["faq.usage.title", "faq.usage.answer"],
			["faq.different.title", "faq.different.answer"],
			["faq.opensource.title", "faq.opensource.answer"],
		],
	},
	{
		headingKey: "faq.group.setup",
		items: [
			["faq.platforms.title", "faq.platforms.answer"],
			["faq.compatible.title", "faq.compatible.answer"],
			["faq.migration.title", "faq.migration.answer"],
			["faq.updates.title", "faq.updates.answer"],
			["faq.languages.title", "faq.languages.answer"],
		],
	},
	{
		headingKey: "faq.group.control",
		items: [
			["faq.clients.title", "faq.clients.answer"],
			["faq.runtime.title", "faq.runtime.answer"],
			["faq.hotreload.title", "faq.hotreload.answer"],
			["faq.security.title", "faq.security.answer"],
			["faq.privacy.title", "faq.privacy.answer"],
		],
	},
	{
		headingKey: "faq.group.compare",
		items: [
			["faq.vs_claude_desktop.title", "faq.vs_claude_desktop.answer"],
			["faq.vs_manual.title", "faq.vs_manual.answer"],
			["faq.contributing.title", "faq.contributing.answer"],
		],
	},
];

const FAQSection = () => {
	const { t } = useLanguage();

	const faqItems = useMemo(
		() => faqSections.flatMap((section) => section.items),
		[],
	);

	const schema = useMemo(
		() =>
			buildFAQPage(
				faqItems.map(([questionKey, answerKey]) => ({
					question: t(questionKey),
					answer: t(answerKey),
				})),
			),
		[faqItems, t],
	);

	return (
		<Section id="faq" className="border-t border-slate-200/70 dark:border-slate-800/60">
			<SchemaOrg schema={schema} />
			<div className="max-w-4xl mx-auto">
				<h2 className="text-3xl md:text-4xl font-bold text-center mb-10">
					{t("faq.title")}
				</h2>

				<div className="space-y-6">
					{faqSections.map((section) => (
						<div
							key={section.headingKey}
							className="rounded-2xl overflow-hidden border-2 border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 shadow-lg"
						>
							<div className="px-6 py-4 border-b border-slate-200 dark:border-slate-700 bg-slate-100/80 dark:bg-slate-800/60">
								<p className="text-base font-semibold text-slate-800 dark:text-slate-200">
									{t(section.headingKey)}
								</p>
							</div>
							<div className="divide-y divide-slate-200 dark:divide-slate-700">
								{section.items.map(([titleKey, answerKey]) => (
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
					))}
				</div>
			</div>
		</Section>
	);
};

export default FAQSection;
