import { ChevronRight } from "lucide-react";
import { useState, type ReactNode } from "react";
import { cn } from "../lib/utils";

export function InspectorDrawerCollapsibleSection({
	title,
	children,
	fill = false,
	collapsible = true,
	defaultExpanded = true,
	headerActions,
	className,
}: {
	title: string;
	children: ReactNode;
	fill?: boolean;
	collapsible?: boolean;
	defaultExpanded?: boolean;
	headerActions?: ReactNode;
	className?: string;
}) {
	const [expanded, setExpanded] = useState(defaultExpanded);
	const isOpen = collapsible ? expanded : true;

	return (
		<section
			className={cn(
				fill ? "flex min-h-0 min-w-0 flex-1 flex-col" : "shrink-0",
				className,
			)}
		>
			<div className="mb-2 flex shrink-0 items-center justify-between gap-2">
				{collapsible ? (
					<button
						type="button"
						className="inline-flex min-w-0 max-w-full items-center gap-1 text-left"
						aria-expanded={expanded}
						onClick={() => setExpanded((current) => !current)}
					>
						<span className="truncate text-xs font-medium uppercase tracking-wide text-muted-foreground">
							{title}
						</span>
						<ChevronRight
							className={cn(
								"h-3 w-3 shrink-0 text-muted-foreground transition-transform",
								expanded && "rotate-90",
							)}
							aria-hidden
						/>
					</button>
				) : (
					<h3 className="truncate text-xs font-medium uppercase tracking-wide text-muted-foreground">
						{title}
					</h3>
				)}
				{isOpen && headerActions ? <div className="shrink-0">{headerActions}</div> : null}
			</div>
			{isOpen ? (
				<div className={cn(fill && "flex min-h-0 flex-1 flex-col")}>{children}</div>
			) : null}
		</section>
	);
}
