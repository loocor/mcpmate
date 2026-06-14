import React from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangle, Download } from "lucide-react";
import {
	translateBackendReadinessIssue,
	type BackendReadinessIssue,
} from "../../lib/backend-readiness-diagnostics";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { OperatorPanelHeader } from "./operator-panel-header";
import { OperatorPanelFrame, OperatorPanelShell } from "./operator-panel-shell";

const OPERATOR_SURFACE_ATTRIBUTE = "data-mcpmate-surface";
const OPERATOR_SURFACE_VALUE = "operator";

type BackendReadinessMessageKey = "starting" | "waitingForBackend" | "confirmingReadiness";

export function OperatorBackendWaitingPage({
	diagnosticsAvailable,
	diagnosticsExportError,
	diagnosticsExporting,
	diagnosticsExportPath,
	issue,
	messageKey,
	onExportDiagnostics,
}: {
	diagnosticsAvailable: boolean;
	diagnosticsExportError: string | null;
	diagnosticsExporting: boolean;
	diagnosticsExportPath: string | null;
	issue: BackendReadinessIssue | null;
	messageKey: BackendReadinessMessageKey;
	onExportDiagnostics: () => Promise<void>;
}) {
	usePageTranslations("operator");
	const { t } = useTranslation();
	const message = t(`operator:startup.${messageKey}`, {
		defaultValue: "Starting MCPMate Core...",
	});
	const issueDetail = issue ? translateBackendReadinessIssue(t, issue) : null;

	React.useEffect(() => {
		if (typeof document === "undefined") {
			return;
		}

		const html = document.documentElement;
		const body = document.body;
		const root = document.getElementById("root");

		html.setAttribute(OPERATOR_SURFACE_ATTRIBUTE, OPERATOR_SURFACE_VALUE);
		body.setAttribute(OPERATOR_SURFACE_ATTRIBUTE, OPERATOR_SURFACE_VALUE);
		root?.setAttribute(OPERATOR_SURFACE_ATTRIBUTE, OPERATOR_SURFACE_VALUE);

		return () => {
			for (const element of [html, body, root]) {
				if (element?.getAttribute(OPERATOR_SURFACE_ATTRIBUTE) === OPERATOR_SURFACE_VALUE) {
					element.removeAttribute(OPERATOR_SURFACE_ATTRIBUTE);
				}
			}
		};
	}, []);

	return (
		<OperatorPanelFrame>
			<OperatorPanelShell data-testid="operator-backend-waiting">
				<OperatorPanelHeader />
				<section className="flex min-h-0 flex-1 flex-col items-center justify-center px-5 py-8 text-center">
					<div className="h-6 w-6 animate-spin rounded-full border-2 border-slate-300 border-t-emerald-500 dark:border-slate-700 dark:border-t-emerald-400" />
					<p className="mt-4 text-sm font-medium text-slate-800 dark:text-slate-100">
						{t("operator:startup.title", { defaultValue: "Starting Core" })}
					</p>
					<p className="mt-1 text-xs text-slate-500 dark:text-slate-400">{message}</p>
					{issue ? (
						<div className="mt-4 w-full rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-left text-amber-950 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-100">
							<div className="flex items-start gap-2">
								<AlertTriangle className="mt-0.5 h-3.5 w-3.5 flex-none" aria-hidden="true" />
								<div className="min-w-0">
									<p className="text-xs font-medium">
										{t("backendReadiness.issueTitle", {
											defaultValue: "Startup needs attention",
										})}
									</p>
									<p className="mt-1 break-words text-[11px] leading-4 opacity-90">
										{issueDetail}
									</p>
								</div>
							</div>
						</div>
					) : null}
					{diagnosticsAvailable ? (
						<div className="mt-3 flex w-full flex-col items-center gap-2">
							<button
								type="button"
								onClick={() => void onExportDiagnostics()}
								disabled={diagnosticsExporting}
								className="inline-flex items-center gap-2 rounded-md border border-slate-200 bg-white px-3 py-1.5 text-xs font-medium text-slate-700 shadow-sm transition hover:border-slate-300 hover:bg-slate-50 disabled:cursor-not-allowed disabled:opacity-60 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200 dark:hover:bg-slate-800"
							>
								<Download className="h-3.5 w-3.5" aria-hidden="true" />
								{diagnosticsExporting
									? t("backendReadiness.exportingDiagnostics", {
											defaultValue: "Exporting diagnostics...",
										})
									: t("backendReadiness.exportDiagnostics", {
											defaultValue: "Export diagnostics",
										})}
							</button>
							{diagnosticsExportPath ? (
								<p className="max-w-full break-words text-[11px] leading-4 text-slate-500 dark:text-slate-400">
									{t("backendReadiness.exportSuccess", {
										defaultValue: "Diagnostics exported to {{path}}",
										path: diagnosticsExportPath,
									})}
								</p>
							) : null}
							{diagnosticsExportError ? (
								<p className="max-w-full break-words text-[11px] leading-4 text-rose-600 dark:text-rose-300">
									{t("backendReadiness.exportFailed", {
										defaultValue: "Unable to export diagnostics: {{error}}",
										error: diagnosticsExportError,
									})}
								</p>
							) : null}
						</div>
					) : null}
				</section>
			</OperatorPanelShell>
		</OperatorPanelFrame>
	);
}
