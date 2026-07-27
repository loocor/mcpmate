import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { cn } from "../lib/utils";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "./ui/tooltip";

type TruncatedTextProps = {
	children: string;
	className?: string;
	tooltipClassName?: string;
	/** When true, render the full text without single-line truncation. */
	expanded?: boolean;
};

export function TruncatedText({
	children,
	className,
	tooltipClassName,
	expanded = false,
}: TruncatedTextProps) {
	const textRef = useRef<HTMLDivElement>(null);
	const [isTruncated, setIsTruncated] = useState(false);

	const updateTruncation = useCallback(() => {
		const element = textRef.current;
		if (!element) return;
		setIsTruncated(element.scrollWidth > element.clientWidth);
	}, []);

	useLayoutEffect(() => {
		if (expanded) {
			setIsTruncated(false);
			return;
		}
		updateTruncation();
	}, [children, expanded, updateTruncation]);

	useEffect(() => {
		if (expanded) return;
		const element = textRef.current;
		if (!element) return;
		const observer = new ResizeObserver(updateTruncation);
		observer.observe(element);
		return () => observer.disconnect();
	}, [expanded, updateTruncation]);

	if (expanded) {
		return (
			<div
				className={cn(
					"block w-full min-w-0 whitespace-pre-wrap break-words",
					className,
				)}
			>
				{children}
			</div>
		);
	}

	return (
		<Tooltip delayDuration={200} open={isTruncated ? undefined : false}>
			<TooltipTrigger asChild>
				<div
					ref={textRef}
					className={cn(
						"block w-full min-w-0 truncate",
						isTruncated && "cursor-default",
						className,
					)}
					aria-label={isTruncated ? children : undefined}
				>
					{children}
				</div>
			</TooltipTrigger>
			<TooltipContent
				side="top"
				align="start"
				collisionPadding={12}
				className={cn(
					"pointer-events-auto max-w-sm max-h-[min(14rem,40vh)] overflow-y-auto overscroll-contain whitespace-normal text-left font-normal leading-relaxed [scrollbar-width:thin]",
					tooltipClassName,
				)}
				onWheel={(event) => event.stopPropagation()}
			>
				{children}
			</TooltipContent>
		</Tooltip>
	);
}
