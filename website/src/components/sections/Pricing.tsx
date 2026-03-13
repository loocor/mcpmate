import { CheckCircle2 } from "lucide-react";
import React from "react";
import { useLanguage } from "../LanguageProvider";
import Button from "../ui/Button";
import Card from "../ui/Card";
import Section from "../ui/Section";

const PricingSection = () => {
	const scrollToSection = (id: string) => {
		const element = document.getElementById(id);
		if (element) {
			const offset = 80;
			const elementPosition = element.getBoundingClientRect().top;
			const offsetPosition = elementPosition + window.pageYOffset - offset;

			window.scrollTo({
				top: offsetPosition,
				behavior: "smooth",
			});
		}
	};

	const { t } = useLanguage();
	const [billing, setBilling] = React.useState<"monthly" | "annual">("monthly");
	return (
		<Section
			id="pricing"
			className="bg-gradient-to-b from-slate-50 to-white dark:from-slate-900 dark:to-slate-800/40 border-t border-slate-200/70 dark:border-slate-700/50"
		>
			<div className="max-w-4xl mx-auto text-center mb-16">
				<h2 className="text-5xl font-extrabold mb-6 bg-gradient-to-r from-slate-900 via-blue-900 to-slate-900 dark:from-slate-100 dark:via-blue-300 dark:to-slate-100 bg-clip-text text-transparent">
					{t("pricing.title")}
				</h2>
				<p className="text-xl text-slate-600 dark:text-slate-400 max-w-2xl mx-auto leading-relaxed">
					{t("pricing.subtitle")}
				</p>
				<p className="mt-3 text-sm text-slate-500 dark:text-slate-400 font-medium">
					{t("pricing.notice.pending")}
				</p>
				<div className="mt-8 inline-flex rounded-lg overflow-hidden border-2 border-slate-300 dark:border-slate-600 shadow-lg bg-white dark:bg-slate-800 p-1">
					<button
						className={`px-6 py-2.5 text-sm font-semibold rounded-md transition-all ${billing === "monthly" ? "bg-gradient-to-r from-blue-600 to-indigo-600 text-white shadow-md" : "text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700"}`}
						onClick={() => setBilling("monthly")}
					>
						{t("pricing.billing.monthly")}
					</button>
					<button
						className={`px-6 py-2.5 text-sm font-semibold rounded-md transition-all ${billing === "annual" ? "bg-gradient-to-r from-blue-600 to-indigo-600 text-white shadow-md" : "text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700"}`}
						onClick={() => setBilling("annual")}
					>
						{t("pricing.billing.annual")}
					</button>
				</div>
			</div>

			<div className="grid grid-cols-1 md:grid-cols-3 gap-8 max-w-6xl mx-auto">
				{/* Starter Plan */}
				<Card className="relative overflow-hidden border-2 border-slate-200 dark:border-slate-700 hover:border-slate-300 dark:hover:border-slate-600 transition-all hover:shadow-xl group">
					<div className="p-8 h-full flex flex-col">
						<div className="mb-auto">
							<h3 className="text-2xl font-bold mb-2 bg-gradient-to-r from-slate-900 to-slate-700 dark:from-slate-100 dark:to-slate-300 bg-clip-text text-transparent">
								{t("pricing.starter")}
							</h3>
							<p className="text-slate-600 dark:text-slate-400 mb-4 text-sm min-h-[2.5rem]">
								{t("pricing.starter.desc")}
							</p>

							<div className="mb-6">
								<span className="text-5xl font-extrabold bg-gradient-to-r from-slate-900 to-slate-700 dark:from-slate-100 dark:to-slate-300 bg-clip-text text-transparent">
									{t("pricing.starter.price")}
								</span>
							</div>

							<Button
								fullWidth
								size="lg"
								className="mb-8 font-semibold shadow-md hover:shadow-lg transition-all"
								onClick={() => scrollToSection("download")}
							>
								{t("nav.preview")}
							</Button>

							<div className="space-y-3.5">
								<h4 className="font-semibold text-slate-900 dark:text-slate-100 text-sm uppercase tracking-wide">
									{t("pricing.whats_included")}:
								</h4>

								<div className="flex items-start group/item">
									<CheckCircle2 className="h-5 w-5 text-emerald-500 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
									<span className="text-slate-700 dark:text-slate-300 text-sm">
										{t("pricing.starter.feature1")}
									</span>
								</div>

								<div className="flex items-start group/item">
									<CheckCircle2 className="h-5 w-5 text-emerald-500 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
									<span className="text-slate-700 dark:text-slate-300 text-sm">
										{t("pricing.starter.feature2")}
									</span>
								</div>

								<div className="flex items-start group/item">
									<CheckCircle2 className="h-5 w-5 text-emerald-500 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
									<span className="text-slate-700 dark:text-slate-300 text-sm">
										{t("pricing.starter.feature3")}
									</span>
								</div>

								<div className="flex items-start group/item">
									<CheckCircle2 className="h-5 w-5 text-emerald-500 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
									<span className="text-slate-700 dark:text-slate-300 text-sm">
										{t("pricing.starter.feature4")}
									</span>
								</div>

								<div className="flex items-start group/item">
									<CheckCircle2 className="h-5 w-5 text-emerald-500 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
									<span className="text-slate-700 dark:text-slate-300 text-sm">
										{t("pricing.starter.feature5")}
									</span>
								</div>

								<div className="flex items-start group/item">
									<CheckCircle2 className="h-5 w-5 text-emerald-500 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
									<span className="text-slate-700 dark:text-slate-300 text-sm">
										{t("pricing.starter.feature6")}
									</span>
								</div>
							</div>
						</div>
					</div>
				</Card>

				{/* Professional Plan (highlighted) */}
				<Card className="relative overflow-hidden ring-2 ring-blue-500 shadow-2xl md:scale-105 lg:scale-110 md:-mt-2 transition-all hover:shadow-[0_18px_40px_-12px_rgba(15,23,42,0.55)] hover:-translate-y-0.5">
					<div className="absolute top-0 right-0 bg-gradient-to-br from-yellow-400 to-orange-500 text-white px-4 py-1.5 rounded-bl-lg text-xs font-bold uppercase tracking-wide shadow-lg z-20">
						{t("pricing.popular")}
					</div>
					{/* Background gradient overlay */}
					<div className="absolute inset-0 bg-gradient-to-br from-blue-600 via-blue-700 to-indigo-800 pointer-events-none" />
					{/* Content wrapper should not be absolutely positioned, otherwise height collapses on small screens */}
					<div className="p-8 h-full flex flex-col relative z-10">
						<h3 className="text-2xl font-bold mb-2 text-white">
							{t("pricing.professional")}
						</h3>
						<p className="mb-4 text-white/90">
							{t("pricing.professional.desc")}
						</p>
                    {(() => {
                        const priceText =
                            billing === "monthly"
                                ? t("pricing.price.professional.monthly")
                                : t("pricing.price.professional.annual_per_month");
                        const isTbd = priceText === t("pricing.professional.price") || /^(tbd|待定)$/i.test(priceText);
                        return (
                            <>
                                <div className={isTbd ? "mb-6" : "mb-2"}>
                                    <span className="text-5xl font-extrabold tracking-tight text-white">
                                        {priceText}
                                    </span>
                                    {!isTbd && (
                                        <span className="ml-2 text-white/80">{t("pricing.per_month")}</span>
                                    )}
                                </div>
                                {!isTbd && (
                                    <div className="mb-6 text-sm text-white/80">
                                        {billing === "annual"
                                            ? t("pricing.billed_annually")
                                            : t("pricing.billed_monthly")}
                                    </div>
                                )}
                            </>
                        );
                    })()}

						<Button
							fullWidth
							size="lg"
							className="mb-8 bg-white/10 text-white hover:bg-white/20 border border-white/20 font-semibold shadow-lg hover:shadow-xl transition-all"
							onClick={() => scrollToSection("download")}
						>
							{t("nav.preview")}
						</Button>

						<div className="space-y-4 flex-grow">
							<h4 className="font-semibold text-white/95 text-sm uppercase tracking-wide">
								{t("pricing.professional.includes")}
							</h4>

							<div className="flex items-start group/item">
								<CheckCircle2 className="h-5 w-5 text-white/90 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
								<span className="text-white/95 text-sm">
									{t("pricing.professional.feature1")}
								</span>
							</div>

							<div className="flex items-start group/item">
								<CheckCircle2 className="h-5 w-5 text-white/90 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
								<span className="text-white/95 text-sm">
									{t("pricing.professional.feature2")}
								</span>
							</div>

							<div className="flex items-start group/item">
								<CheckCircle2 className="h-5 w-5 text-white/90 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
								<span className="text-white/95 text-sm">
									{t("pricing.professional.feature3")}
								</span>
							</div>

							<div className="flex items-start group/item">
								<CheckCircle2 className="h-5 w-5 text-white/90 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
								<span className="text-white/95 text-sm">
									{t("pricing.professional.feature4")}
								</span>
							</div>

							<div className="flex items-start group/item">
								<CheckCircle2 className="h-5 w-5 text-white/90 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
								<span className="text-white/95 text-sm">
									{t("pricing.professional.feature5")}
								</span>
							</div>

							<div className="flex items-start group/item">
								<CheckCircle2 className="h-5 w-5 text-white/90 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
								<span className="text-white/95 text-sm">
									{t("pricing.professional.feature6")}
								</span>
							</div>
						</div>
					</div>
				</Card>

				{/* Advanced Plan */}
				<Card className="relative overflow-hidden border-2 border-slate-200 dark:border-slate-700 hover:border-purple-300 dark:hover:border-purple-600 transition-all hover:shadow-xl group">
					<div className="absolute inset-0 bg-gradient-to-br from-purple-50 to-transparent dark:from-purple-950/20 dark:to-transparent opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none"></div>
					<div className="p-8 h-full flex flex-col relative z-10">
						<div className="mb-auto">
							<h3 className="text-2xl font-bold mb-2 bg-gradient-to-r from-purple-600 to-indigo-600 dark:from-purple-400 dark:to-indigo-400 bg-clip-text text-transparent">
								{t("pricing.advanced")}
							</h3>
							<p className="text-slate-600 dark:text-slate-400 mb-4 text-sm min-h-[2.5rem]">
								{t("pricing.advanced.desc")}
							</p>

							<div className="mb-6">
								<span className="text-5xl font-extrabold bg-gradient-to-r from-purple-600 to-indigo-600 dark:from-purple-400 dark:to-indigo-400 bg-clip-text text-transparent">
									{t("pricing.advanced.price")}
								</span>
							</div>

							<Button
								fullWidth
								size="lg"
								className="mb-8 font-semibold shadow-md hover:shadow-lg transition-all bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-700 hover:to-indigo-700 text-white border-0"
								onClick={() => scrollToSection("contact")}
							>
								{t("pricing.contact_sales")}
							</Button>

							<div className="space-y-3.5">
								<h4 className="font-semibold text-slate-900 dark:text-slate-100 text-sm uppercase tracking-wide">
									{t("pricing.advanced.includes")}
								</h4>
								<div className="flex items-start group/item">
									<CheckCircle2 className="h-5 w-5 text-purple-500 dark:text-purple-400 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
									<span className="text-slate-700 dark:text-slate-300 text-sm">
										{t("pricing.advanced.feature1")}
									</span>
								</div>
								<div className="flex items-start group/item">
									<CheckCircle2 className="h-5 w-5 text-purple-500 dark:text-purple-400 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
									<span className="text-slate-700 dark:text-slate-300 text-sm">
										{t("pricing.advanced.feature2")}
									</span>
								</div>
								<div className="flex items-start group/item">
									<CheckCircle2 className="h-5 w-5 text-purple-500 dark:text-purple-400 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
									<span className="text-slate-700 dark:text-slate-300 text-sm">
										{t("pricing.advanced.feature3")}
									</span>
								</div>
								<div className="flex items-start group/item">
									<CheckCircle2 className="h-5 w-5 text-purple-500 dark:text-purple-400 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
									<span className="text-slate-700 dark:text-slate-300 text-sm">
										{t("pricing.advanced.feature4")}
									</span>
								</div>
								<div className="flex items-start group/item">
									<CheckCircle2 className="h-5 w-5 text-purple-500 dark:text-purple-400 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
									<span className="text-slate-700 dark:text-slate-300 text-sm">
										{t("pricing.advanced.feature5")}
									</span>
								</div>
								<div className="flex items-start group/item">
									<CheckCircle2 className="h-5 w-5 text-purple-500 dark:text-purple-400 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
									<span className="text-slate-700 dark:text-slate-300 text-sm">
										{t("pricing.advanced.feature6")}
									</span>
								</div>
								<div className="flex items-start group/item">
									<CheckCircle2 className="h-5 w-5 text-purple-500 dark:text-purple-400 mt-0.5 mr-3 flex-shrink-0 group-hover/item:scale-110 transition-transform" />
									<span className="text-slate-700 dark:text-slate-300 text-sm">
										{t("pricing.advanced.feature7")}
									</span>
								</div>
							</div>
						</div>
					</div>
				</Card>
			</div>

			{/* FAQ */}
			<div id="faq" className="max-w-3xl mx-auto mt-24">
				<h3 className="text-3xl font-extrabold text-center mb-10 bg-gradient-to-r from-slate-900 via-blue-900 to-slate-900 dark:from-slate-100 dark:via-blue-300 dark:to-slate-100 bg-clip-text text-transparent">
					{t("faq.title")}
				</h3>

				<div className="divide-y divide-slate-200 dark:divide-slate-700 rounded-2xl overflow-hidden bg-clip-padding isolate border-2 border-slate-200 dark:border-slate-700 bg-white/80 dark:bg-slate-900/60 backdrop-blur-sm shadow-lg">
					{(
						[
							["faq.different.title", "faq.different.answer"],
							["faq.compatible.title", "faq.compatible.answer"],
							["faq.upgrade.title", "faq.upgrade.answer"],
							["faq.expiry.title", "faq.expiry.answer"],
							["faq.platforms.title", "faq.platforms.answer"],
							["faq.security.title", "faq.security.answer"],
							["faq.privacy.title", "faq.privacy.answer"],
							["faq.updates.title", "faq.updates.answer"],
							["faq.opensource.title", "faq.opensource.answer"],
						] as const
					).map(([titleKey, answerKey]) => (
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

export default PricingSection;
