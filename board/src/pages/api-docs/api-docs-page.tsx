import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertCircle } from "lucide-react";
import { API_BASE_CHANGED_EVENT, resolveApiUrl } from "../../lib/api";

function resolveApiDocsUrl(): string {
	return resolveApiUrl("/docs");
}

export function ApiDocsPage() {
	const { t } = useTranslation();
	const [docsUrl, setDocsUrl] = useState(resolveApiDocsUrl);
	const [iframeFailed, setIframeFailed] = useState(false);

	useEffect(() => {
		const handleApiBaseChanged = () => {
			setIframeFailed(false);
			setDocsUrl(resolveApiDocsUrl());
		};

		window.addEventListener(API_BASE_CHANGED_EVENT, handleApiBaseChanged);
		return () => {
			window.removeEventListener(API_BASE_CHANGED_EVENT, handleApiBaseChanged);
		};
	}, []);

	const pageTitle = t("nav.apiDocs", { defaultValue: "API Docs" });

	return (
		<div className="flex h-[calc(100vh-8rem)] flex-col gap-3">
			<div className="relative min-h-0 flex-1 overflow-hidden rounded-lg border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-900">
				<iframe
					key={docsUrl}
					src={docsUrl}
					title={pageTitle}
					className="h-full w-full"
					onError={() => setIframeFailed(true)}
				/>
				{iframeFailed ? (
					<div className="absolute inset-0 flex items-center justify-center bg-background/95">
						<div className="max-w-md space-y-2 p-4 text-center">
							<AlertCircle className="mx-auto h-8 w-8 text-amber-500" />
							<p className="text-sm font-medium text-foreground">
								{t("errors.serverUnavailable", {
									defaultValue: "Service unavailable",
								})}
							</p>
							<p className="text-xs text-muted-foreground">
								{t("settings:system.portsReloadFailedDescription", {
									defaultValue:
										"Check the desktop app is healthy and try Reload again.",
								})}
							</p>
						</div>
					</div>
				) : null}
			</div>
		</div>
	);
}
