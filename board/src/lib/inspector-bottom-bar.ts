export const INSPECTOR_BOTTOM_BAR_COLLAPSED_HEIGHT_PX = 32;

export const INSPECTOR_BOTTOM_BAR_MIN_HEIGHT_PX = 120;

export const INSPECTOR_BOTTOM_BAR_MAX_HEIGHT_PX = 640;

export function resolveInspectorBottomBarHeight(
	expanded: boolean,
	height: number,
): number {
	if (!expanded) {
		return INSPECTOR_BOTTOM_BAR_COLLAPSED_HEIGHT_PX;
	}
	return Math.min(
		INSPECTOR_BOTTOM_BAR_MAX_HEIGHT_PX,
		Math.max(INSPECTOR_BOTTOM_BAR_MIN_HEIGHT_PX, height),
	);
}

export const INSPECTOR_BOTTOM_BAR_HEADER_CLASSNAME =
	"flex h-8 shrink-0 items-center border-b border-border/60";

export const INSPECTOR_BOTTOM_BAR_HEADER_WITH_ACTIONS_CLASSNAME =
	"flex h-8 shrink-0 items-center border-b border-border/60";

export const INSPECTOR_BOTTOM_BAR_HEADER_ACTIONS_CLASSNAME =
	"flex shrink-0 items-center gap-1 pr-1";

export const INSPECTOR_BOTTOM_BAR_ICON_BUTTON_CLASSNAME =
	"inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-transparent text-muted-foreground transition-colors hover:bg-transparent hover:text-foreground focus-visible:outline-none focus-visible:ring-0";

export const INSPECTOR_BOTTOM_BAR_TOGGLE_CLASSNAME =
	"flex h-8 min-w-0 flex-1 items-center gap-1.5 px-3 text-left";

export const INSPECTOR_BOTTOM_BAR_TOGGLE_COLLAPSED_CLASSNAME =
	"flex h-8 w-full items-center justify-center";

export const INSPECTOR_BOTTOM_BAR_SHELL_CLASSNAME =
	"shrink-0 overflow-hidden border-t border-border bg-card";
