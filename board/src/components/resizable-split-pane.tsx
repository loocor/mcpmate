import { GripVertical } from "lucide-react";
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";

import { cn } from "../lib/utils";

interface ResizableSplitPaneProps {
	children: [ReactNode, ReactNode];
	dividerAriaLabel: string;
	className?: string;
	initialLeftWidth?: number;
	minLeftWidth?: number;
	maxLeftWidth?: number;
	preferRightPanelSpace?: boolean;
}

export function ResizableSplitPane({
	children: [left, right],
	dividerAriaLabel,
	className,
	initialLeftWidth = 300,
	minLeftWidth = 240,
	maxLeftWidth = 460,
	preferRightPanelSpace = false,
}: ResizableSplitPaneProps) {
	const [leftWidth, setLeftWidth] = useState(initialLeftWidth);
	const containerRef = useRef<HTMLDivElement>(null);
	const containerWidthRef = useRef<number | null>(null);

	useEffect(() => {
		if (!preferRightPanelSpace || !containerRef.current) return;

		const observer = new ResizeObserver(([entry]) => {
			const width = entry.contentRect.width;
			const previousWidth = containerWidthRef.current;
			containerWidthRef.current = width;
			if (previousWidth !== null && width < previousWidth) {
				setLeftWidth((current) =>
					Math.max(minLeftWidth, current - (previousWidth - width)),
				);
			}
		});
		observer.observe(containerRef.current);
		return () => observer.disconnect();
	}, [minLeftWidth, preferRightPanelSpace]);

	const handleDividerPointerDown = useCallback(
		(event: React.PointerEvent<HTMLButtonElement>) => {
			event.preventDefault();
			const startX = event.clientX;
			const startWidth = leftWidth;
			const handlePointerMove = (moveEvent: PointerEvent) => {
				const nextWidth = startWidth + moveEvent.clientX - startX;
				const clampedWidth = Math.min(maxLeftWidth, Math.max(minLeftWidth, nextWidth));
				setLeftWidth(clampedWidth);
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
			ref={containerRef}
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
