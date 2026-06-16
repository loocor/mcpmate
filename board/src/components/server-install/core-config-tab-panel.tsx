import type { ReactNode } from "react";
import { FORM_TAB_TOOLBAR_ROW_CLASS, formTabScrollClass } from "./form-tab-layout";
import { FormViewModeToggle, type FormViewMode } from "./view-mode-toggle";

interface CoreConfigTabPanelProps {
	viewMode: FormViewMode;
	onViewModeChange: (mode: FormViewMode) => void;
	onContentClick?: () => void;
	toggleVariant?: "default" | "compact";
	formContent: ReactNode;
	jsonContent: ReactNode;
}

export function CoreConfigTabPanel({
	viewMode,
	onViewModeChange,
	onContentClick,
	toggleVariant = "compact",
	formContent,
	jsonContent,
}: CoreConfigTabPanelProps) {
	return (
		<>
			<div className={FORM_TAB_TOOLBAR_ROW_CLASS}>
				<FormViewModeToggle
					mode={viewMode}
					onChange={onViewModeChange}
					variant={toggleVariant}
				/>
			</div>
			<div className={formTabScrollClass(viewMode)} onClick={onContentClick}>
				{viewMode === "form" ? formContent : jsonContent}
			</div>
		</>
	);
}
