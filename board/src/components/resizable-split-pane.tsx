import { GripVertical } from "lucide-react";
import { useCallback, useState, type ReactNode } from "react";

import { cn } from "../lib/utils";

interface ResizableSplitPaneProps {
	children: [ReactNode, ReactNode];
	dividerAriaLabel: string;
	className?: string;
	initialLeftWidth?: number;
	minLeftWidth?: number;
	maxLeftWidth?: number;
}

export function ResizableSplitPane({
	children: [left, right],
	dividerAriaLabel,
	className,
	initialLeftWidth = 300,
	minLeftWidth = 240,
	maxLeftWidth = 460,
}: ResizableSplitPaneProps) {
	const [leftWidth, setLeftWidth] = useState(initialLeftWidth);

	const handleDividerPointerDown = useCallback(
		(event: React.PointerEvent<HTMLButtonElement>) => {
			event.preventDefault();
			const startX = event.clientX;
			const startWidth = leftWidth;
			const handlePointerMove = (moveEvent: PointerEvent) => {
				const nextWidth = startWidth + moveEvent.clientX - startX;
				setLeftWidth(Math.min(maxLeftWidth, Math.max(minLeftWidth, nextWidth)));
			};
			const handlePointerUp = () => {
				window.removeEventListener("pointermove", handlePointerMove);
				window.removeEventListener("pointerup", handlePointerUp);
			};

			window.addEventListener("pointermove", handlePointerMove);
			window.addEventListener("pointerup", handlePointerUp);
		},
		[leftWidth, maxLeftWidth, minLeftWidth],
	);

	return (
		<div
			className={cn("grid min-h-0 flex-1 overflow-hidden", className)}
			style={{ gridTemplateColumns: `${leftWidth}px 8px minmax(0, 1fr)` }}
		>
			{left}
			<button
				type="button"
				aria-label={dividerAriaLabel}
				className="group flex cursor-col-resize items-center justify-center border-x border-border bg-muted/20 text-muted-foreground transition-colors hover:bg-muted/50 focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
				onPointerDown={handleDividerPointerDown}
			>
				<GripVertical className="h-4 w-4 opacity-50 group-hover:opacity-80" />
			</button>
			{right}
		</div>
	);
}
