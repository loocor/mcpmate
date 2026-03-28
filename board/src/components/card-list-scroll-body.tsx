import type { ReactNode } from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { cn } from "../lib/utils";

type CardListScrollBodyProps = {
	children: ReactNode;
	className?: string;
};

const SCROLL_SHADE_HIDE_MS = 280;

/** Border + clip; shadow lives on an overlay so list rows do not paint over it. */
const frameClass =
	"relative flex min-h-0 flex-1 flex-col overflow-hidden rounded-[10px] border border-slate-200/80 bg-clip-padding dark:border-slate-700/80";

const scrollClass =
	"relative z-0 min-h-0 flex-1 overflow-y-auto overflow-x-hidden overscroll-contain";

const insetShadeBaseClass =
	"pointer-events-none absolute inset-0 z-[1] rounded-[10px] transition-[opacity,box-shadow] duration-200 ease-out";

const insetShadeActiveClass =
	"opacity-100 shadow-[inset_0_2px_10px_rgba(15,23,42,0.09)] dark:shadow-[inset_0_2px_12px_rgba(0,0,0,0.5)]";

const insetShadeIdleClass = "opacity-0 shadow-none";

/**
 * Use inside a flex `Card` body (`CardContent` with `flex min-h-0 flex-1 flex-col overflow-hidden`).
 * Scrolls children in a pane with full corner radius and border aligned to list content (`CapsuleStripeList`).
 * Inset shade appears only while the user is scrolling (and briefly after), so the panel stays flat at rest.
 */
export function CardListScrollBody({ children, className }: CardListScrollBodyProps) {
	const [shadeFromScroll, setShadeFromScroll] = useState(false);
	const hideTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

	const clearHideTimer = useCallback(() => {
		if (hideTimerRef.current != null) {
			clearTimeout(hideTimerRef.current);
			hideTimerRef.current = null;
		}
	}, []);

	const handleScroll = useCallback(() => {
		setShadeFromScroll(true);
		clearHideTimer();
		hideTimerRef.current = setTimeout(() => {
			setShadeFromScroll(false);
			hideTimerRef.current = null;
		}, SCROLL_SHADE_HIDE_MS);
	}, [clearHideTimer]);

	useEffect(() => () => clearHideTimer(), [clearHideTimer]);

	return (
		<div className={cn("flex min-h-0 flex-1 flex-col overflow-hidden", className)}>
			<div className={frameClass}>
				<div className={scrollClass} onScroll={handleScroll}>
					{children}
				</div>
				<div
					className={cn(
						insetShadeBaseClass,
						shadeFromScroll ? insetShadeActiveClass : insetShadeIdleClass,
					)}
					aria-hidden
				/>
			</div>
		</div>
	);
}
