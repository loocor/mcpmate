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
};

export function TruncatedText({
	children,
	className,
	tooltipClassName,
}: TruncatedTextProps) {
	const textRef = useRef<HTMLDivElement>(null);
	const [isTruncated, setIsTruncated] = useState(false);

	const updateTruncation = useCallback(() => {
		const element = textRef.current;
		if (!element) return;
		setIsTruncated(element.scrollWidth > element.clientWidth);
	}, []);

	useLayoutEffect(() => {
		updateTruncation();
	}, [children, updateTruncation]);

	useEffect(() => {
		const element = textRef.current;
		if (!element) return;
		const observer = new ResizeObserver(updateTruncation);
		observer.observe(element);
		return () => observer.disconnect();
	}, [updateTruncation]);

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
