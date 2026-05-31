import React from "react";
import { useTranslation } from "react-i18next";
import { operatorNoDragRegionStyle } from "./operator-row-detail-shared";

const OPERATOR_HEADER_LOGO_CLASS =
	"h-7 w-7 shrink-0 object-contain dark:invert dark:brightness-0";
export const OPERATOR_HEADER_CLASS =
	"flex h-11 shrink-0 items-center gap-2 border-b border-slate-200 px-3 dark:border-slate-800";

function OperatorPanelLogo() {
	return (
		<img
			src="/logo.svg"
			alt="MCPMate"
			className={OPERATOR_HEADER_LOGO_CLASS}
			draggable={false}
		/>
	);
}

function OperatorPanelHeaderBrand() {
	const { t } = useTranslation();

	return (
		<>
			<OperatorPanelLogo />
			<div className="min-w-0">
				<h1 className="truncate text-sm font-semibold">MCPMate</h1>
				<p className="truncate text-[10px] font-medium text-slate-500 dark:text-slate-400">
					{t("operator:title", { defaultValue: "Operator Panel" })}
				</p>
			</div>
		</>
	);
}

function OperatorPanelHeaderDragSpacer() {
	return <div aria-hidden className="min-w-0 flex-1 self-stretch" />;
}

export function OperatorPanelHeader({ controls }: { controls?: React.ReactNode }) {
	return (
		<header
			className={OPERATOR_HEADER_CLASS}
			data-operator-drag-region="true"
		>
			<div className="flex min-w-0 items-center gap-2">
				<OperatorPanelHeaderBrand />
			</div>
			<OperatorPanelHeaderDragSpacer />
			{controls ? (
				<div
					className="flex shrink-0 items-center gap-1"
					data-operator-no-drag="true"
					style={operatorNoDragRegionStyle}
				>
					{controls}
				</div>
			) : null}
		</header>
	);
}

export { operatorNoDragRegionStyle };
