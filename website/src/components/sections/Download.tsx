import {
	ArrowRight,
	Check,
	Copy,
	Terminal,
} from "lucide-react";
import { useState } from "react";
import { useLanguage } from "../LanguageProvider";
import { useNavigate } from "react-router-dom";
import Button from "../ui/Button";
import Section from "../ui/Section";

const QuickStartSection = () => {
	const [copied, setCopied] = useState(false);
	const { t, language } = useLanguage();
	const navigate = useNavigate();

	const buildCommand = "git clone https://github.com/loocor/mcpmate.git && cd mcpmate/backend && cargo build --release";

	const handleCopy = async () => {
		try {
			await navigator.clipboard.writeText(buildCommand);
			setCopied(true);
			setTimeout(() => setCopied(false), 2000);
		} catch {
			// noop
		}
	};

	return (
		<Section id="download" className="bg-gradient-to-b from-blue-50 to-white dark:from-slate-800/50 dark:to-slate-900 border-t border-slate-200/70 dark:border-slate-700/50">
			<div className="max-w-4xl mx-auto">
				<div className="text-center mb-12">
					<h2 className="text-3xl md:text-4xl font-bold mb-2">
						{t("download.quick_start")}
					</h2>
					<p className="text-lg text-slate-600 dark:text-slate-400 mt-3">
						{t("download.subtitle")}
					</p>
				</div>

				<div className="grid grid-cols-1 md:grid-cols-2 gap-8">
					<div>
						<h3 className="text-lg font-semibold mb-4">
							{t("download.install_cli")}
						</h3>
						<div className="bg-slate-900 rounded-lg p-4 font-mono text-sm text-white relative">
							<div className="flex items-center gap-2 mb-2">
								<Terminal size={16} className="text-slate-400" />
								<span className="text-slate-400 text-xs">bash</span>
							</div>
							<code className="break-all text-green-400">{buildCommand}</code>
							<button
								type="button"
								className="absolute top-3 right-3 p-2 rounded hover:bg-slate-800 transition-colors"
								onClick={handleCopy}
								aria-label="Copy command"
							>
								{copied ? (
									<Check className="h-4 w-4 text-green-400" />
								) : (
									<Copy className="h-4 w-4 text-slate-400" />
								)}
							</button>
						</div>
						<p className="text-sm text-slate-500 dark:text-slate-400 mt-3">
							{t("download.getting_started.desc")}
						</p>
					</div>

					<div className="space-y-6">
						<div>
							<h3 className="text-lg font-semibold mb-2">
								{t("download.getting_started")}
							</h3>
							<p className="text-slate-600 dark:text-slate-400 mb-4">
								{t("download.getting_started.desc")}
							</p>
							<Button
								variant="outline"
								className="w-full flex items-center justify-center gap-2"
								onClick={() => navigate(language === 'zh' ? '/docs/zh/quickstart' : '/docs/en/quickstart')}
							>
								<span>{t("download.read_guide")}</span>
								<ArrowRight size={16} />
							</Button>
						</div>

						<div>
							<h3 className="text-lg font-semibold mb-2">
								{t("contact.github")}
							</h3>
							<p className="text-slate-600 dark:text-slate-400 mb-4">
								{t("contact.github.desc")}
							</p>
							<Button
								variant="outline"
								className="w-full flex items-center justify-center gap-2"
								onClick={() => window.open('https://github.com/loocor/mcpmate', '_blank')}
							>
								<span>github.com/loocor/mcpmate</span>
								<ArrowRight size={16} />
							</Button>
						</div>
					</div>
				</div>
			</div>
		</Section>
	);
};

export default QuickStartSection;
