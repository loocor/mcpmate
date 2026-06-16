import { cn } from "../../lib/utils";
import type { FormViewMode } from "./view-mode-toggle";

export const FORM_TAB_SHELL_CLASS =
	"mt-2 flex min-h-0 flex-1 flex-col gap-4 focus-visible:outline-none";

export const FORM_TAB_TOOLBAR_ROW_CLASS = "flex shrink-0 justify-end pt-2";

/** Below tab bar; matches Core toolbar `pt-2` without toggle height or `gap-4`. */
export const FORM_TAB_PANEL_TOP_INSET_CLASS = "pt-2";

export const SECONDARY_TAB_CONTENT_CLASS = cn(
	"mt-2 min-h-0 flex-1 focus-visible:outline-none",
	FORM_TAB_PANEL_TOP_INSET_CLASS,
);

export const INSTALL_DRAWER_CONTENT_CLASS = "flex h-full flex-col overflow-hidden";

export const INSTALL_FORM_CLASS = "flex h-full min-h-0 flex-col";

export const FORM_FILL_SHELL_CLASS = "flex min-h-0 flex-1 flex-col overflow-hidden";

export function isCoreJsonView(activeTab: string, viewMode: FormViewMode): boolean {
	return activeTab === "core" && viewMode === "json";
}

export function formContentScrollClass(isCoreJsonPanel: boolean) {
	return isCoreJsonPanel
		? "overflow-hidden"
		: "overflow-y-auto overscroll-contain";
}

export function installFormBodyClass(
	ingestEnabled: boolean,
	isCoreJsonPanel: boolean,
) {
	return cn(
		"relative z-0 flex min-h-0 flex-1 flex-col px-4 pb-4",
		ingestEnabled ? "pt-0" : "pt-4",
		formContentScrollClass(isCoreJsonPanel),
	);
}

export function formTabScrollClass(viewMode: FormViewMode) {
	return cn(
		"min-h-0 flex-1",
		viewMode === "json"
			? "flex flex-col overflow-hidden py-0.5"
			: "space-y-4 py-0.5",
	);
}
