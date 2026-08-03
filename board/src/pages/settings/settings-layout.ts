/**
 * Shared layout tokens for Settings tab panels and setting field rows.
 * Import these instead of repeating Tailwind strings across settings pages.
 */

export const SETTINGS_TAB_TRIGGER_CLASS =
	"w-full justify-center gap-2 px-2 py-2 text-left text-sm font-medium text-slate-600 data-[state=active]:text-emerald-700 md:justify-start md:px-3 dark:text-slate-300";

export const SETTINGS_CARD_CONTENT_CLASS = "space-y-5";

export const SETTINGS_CARD_CONTENT_STACK_CLASS = "flex h-full flex-col gap-5";

export const SETTINGS_SECTION_CLASS = "space-y-5";

export const SETTINGS_ITEM_TITLE_CLASS = "text-base font-medium";

export const SETTINGS_ITEM_DESCRIPTION_CLASS = "text-sm text-muted-foreground";

/** Title + description column in a setting row. */
export const SETTINGS_LABEL_CLASS = "min-w-0 space-y-0.5";

export const SETTINGS_ROW_CLASS =
	"flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between sm:gap-4";

export const SETTINGS_CONTROL_CLASS = "w-full shrink-0 sm:w-72";

export const SETTINGS_CLIENTS_ROW_CLASS =
	"flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between sm:gap-6";

export const SETTINGS_CLIENTS_LABEL_CLASS =
	"min-w-0 max-w-full flex-1 space-y-0.5 sm:pr-2 lg:max-w-lg xl:max-w-xl";

export const SETTINGS_CLIENTS_CONTROL_CLASS =
	"w-full shrink-0 sm:w-auto sm:min-w-[16rem] md:min-w-[20rem] lg:min-w-[24rem]";

export const SETTINGS_SWITCH_ROW_CLASS =
	"flex items-center justify-between gap-4";

export const SETTINGS_GRID_ROW_CLASS =
	"grid grid-cols-1 gap-2 sm:grid-cols-2 sm:items-center";

export const SETTINGS_GRID_CONTROL_CLASS = "flex sm:justify-end";

export const SETTINGS_SELECT_TRIGGER_WIDE_CLASS = "w-full sm:w-72";

export const SETTINGS_SELECT_TRIGGER_CLASS = "w-full sm:w-64";

export const SETTINGS_INPUT_CLASS = "w-full sm:w-64";

export const SETTINGS_INPUT_WIDE_CLASS = "w-full sm:w-80";

export const SETTINGS_SECURITY_GROUP_CLASS = "space-y-3";

export const SETTINGS_SECURITY_DIVIDER_CLASS = "space-y-3 border-t pt-6";

export const SETTINGS_LABEL_FLEX_CLASS = "min-w-0 space-y-0.5 md:flex-1";

export const SETTINGS_MUTED_HINT_CLASS =
	"text-xs text-muted-foreground sm:text-right";
