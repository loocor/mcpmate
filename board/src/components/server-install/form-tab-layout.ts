import { cn } from "../../lib/utils";

export const FORM_TAB_SHELL_CLASS =
	"mt-2 flex min-h-0 flex-1 flex-col gap-4 focus-visible:outline-none";

export const FORM_TAB_TOOLBAR_ROW_CLASS = "flex shrink-0 justify-end pt-2";

/** Below tab bar; matches Core toolbar `pt-2` without toggle height or `gap-4`. */
export const FORM_TAB_PANEL_TOP_INSET_CLASS = "pt-2";

export function formContentScrollClass(isCoreJsonPanel: boolean) {
	return isCoreJsonPanel
		? "overflow-hidden"
		: "overflow-y-auto overscroll-contain";
}

export function formTabScrollClass(viewMode: "form" | "json") {
	return cn(
		"min-h-0 flex-1",
		viewMode === "json"
			? "flex flex-col overflow-hidden"
			: "space-y-4 py-0.5",
	);
}
