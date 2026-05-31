import React from "react";
import { useTranslation } from "react-i18next";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { OperatorPanelHeader } from "./operator-panel-header";
import { OperatorPanelFrame, OperatorPanelShell } from "./operator-panel-shell";

const OPERATOR_SURFACE_ATTRIBUTE = "data-mcpmate-surface";
const OPERATOR_SURFACE_VALUE = "operator";

type BackendReadinessMessageKey = "starting" | "waitingForBackend" | "confirmingReadiness";

export function OperatorBackendWaitingPage({
	messageKey,
}: {
	messageKey: BackendReadinessMessageKey;
}) {
	usePageTranslations("operator");
	const { t } = useTranslation();
	const message = t(`operator:startup.${messageKey}`, {
		defaultValue: "Starting MCPMate Core...",
	});

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
				</section>
			</OperatorPanelShell>
		</OperatorPanelFrame>
	);
}
