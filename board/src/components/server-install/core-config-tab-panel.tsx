import type { ReactNode } from "react";
import { FORM_TAB_TOOLBAR_ROW_CLASS, formTabScrollClass } from "./form-tab-layout";
import { FormViewModeToggle, type FormViewMode } from "./view-mode-toggle";
import { cn } from "../../lib/utils";

interface CoreConfigTabPanelProps {
	viewMode: FormViewMode;
	onViewModeChange: (mode: FormViewMode) => void;
	onContentClick?: () => void;
	toggleVariant?: "default" | "compact";
	toolbarClassName?: string;
	toolbarInsideScroll?: boolean;
	scrollClassName?: string;
	formContent: ReactNode;
	jsonContent: ReactNode;
}

export function CoreConfigTabPanel({
	viewMode,
	onViewModeChange,
	onContentClick,
	toggleVariant = "compact",
	toolbarClassName,
	toolbarInsideScroll = false,
	scrollClassName,
	formContent,
	jsonContent,
}: CoreConfigTabPanelProps) {
	const toolbar = (
		<div className={cn(FORM_TAB_TOOLBAR_ROW_CLASS, toolbarClassName)}>
			<FormViewModeToggle
				mode={viewMode}
				onChange={onViewModeChange}
				variant={toggleVariant}
			/>
		</div>
	);
	const content = viewMode === "form" ? formContent : jsonContent;

	if (toolbarInsideScroll) {
		return (
			<div
				className={cn(formTabScrollClass(viewMode), scrollClassName)}
				onClick={onContentClick}
			>
				{toolbar}
				{content}
			</div>
		);
	}

	return (
		<>
			{toolbar}
			<div
				className={cn(formTabScrollClass(viewMode), scrollClassName)}
				onClick={onContentClick}
			>
				{content}
			</div>
		</>
	);
}
