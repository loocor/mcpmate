/**
 * Shared TabsContent layout used by detail pages.
 * Keeps tab panels stretchable while inner list/card handles scrolling.
 */
export const DETAIL_TAB_CONTENT_CLASS =
	"mt-0 flex min-h-0 flex-1 flex-col overflow-hidden data-[state=inactive]:hidden";

/** Overview tab column: metadata/stats pinned at top, logs panel fills remaining height. */
export const DETAIL_OVERVIEW_STACK_CLASS =
	"flex h-full min-h-0 flex-1 flex-col gap-4";

/** Sections above the fill-height logs card (metadata, stats, instance lists). */
export const DETAIL_OVERVIEW_PINNED_SECTION_CLASS = "shrink-0";

/** Allow unbounded overview lists to scroll without changing short-list height. */
export const DETAIL_OVERVIEW_SCROLLABLE_LIST_CLASS =
	"max-h-[min(16rem,35vh)] overflow-y-auto";
