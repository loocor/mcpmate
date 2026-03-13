import { type ReactNode } from "react";

/**
 * Reusable component for list item action section with conditional badge alignment.
 *
 * When hover actions are NOT provided:
 * - Badges align to the right edge (ml-auto)
 *
 * When hover actions ARE provided:
 * - Badges stay in their default position
 * - Actions appear on hover/focus to the right of badges without reserving space
 *
 * This ensures badges always appear right-aligned when action buttons aren't rendered,
 * preventing empty space on the right side.
 */
export interface ListItemActionsProps {
	/** Static badges that are always visible */
	badges?: ReactNode;
	/** Actions that appear on hover (Switch, debug button, etc.) */
	hoverActions?: ReactNode;
	/** Controls whether hover actions should reserve space or only appear on hover */
	appearance?: "inline" | "hover";
}

export function ListItemActions({
	badges,
	hoverActions,
	appearance = "inline",
}: ListItemActionsProps) {
	// When no hover actions, badges should take full right alignment
	if (!hoverActions && badges) {
		return <div className="ml-auto flex items-center gap-2">{badges}</div>;
	}

	// When hover actions exist, defer rendering until hover/focus keeps badges flush right by default
	if (badges || hoverActions) {
		const hoverClass =
			appearance === "hover"
				? "hidden items-center gap-2 transition-all duration-150 group-hover:flex group-focus-within:flex group-focus-visible:flex"
				: "flex items-center gap-2";

		return (
			<div className="ml-auto flex items-start gap-2">
				{badges}
				{hoverActions ? <div className={hoverClass}>{hoverActions}</div> : null}
			</div>
		);
	}

	return null;
}
