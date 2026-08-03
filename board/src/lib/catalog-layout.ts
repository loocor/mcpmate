/**
 * Catalog list hover uses `-translate-y-0.5` (2px). Top inset prevents border clipping
 * inside `overflow-y-auto` shells; matching negative margin keeps stats-to-list rhythm.
 */
export const catalogPageSectionClassName = "flex min-h-0 flex-1 flex-col -mt-0.5";

export const catalogScrollShellClassName =
	"min-h-0 flex-1 overflow-y-auto pt-0.5";

/** Catalog surface without a dedicated inner scroll shell (legacy partial inset). */
export const catalogSurfaceClassName = "-mt-0.5 pt-0.5";
