/** Shared shell for client logo tiles and matching marketing stat cards. */
export const CLIENT_TILE_SHELL_CLASS =
	"flex flex-col items-center justify-center gap-2.5 rounded-xl border border-[color-mix(in_srgb,var(--brand-border-subtle)_72%,var(--brand-elevated))] bg-brand-elevated px-3 py-4";

export const CLIENT_TILE_SHELL_TALL_CLASS = `${CLIENT_TILE_SHELL_CLASS} min-h-[108px]`;

export const CLIENT_TILE_HOVER_CLASS =
	"transition-all duration-200 hover:-translate-y-0.5 hover:border-brand-accent/60 hover:shadow-glow-sm";

export const CLIENT_TILE_GROUP_HOVER_CLASS =
	"transition-all duration-200 group-hover:-translate-y-0.5 group-hover:border-brand-accent/60 group-hover:shadow-glow-sm";
