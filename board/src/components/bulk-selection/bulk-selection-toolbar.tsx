import { ListChecks } from "lucide-react";
import { Button } from "../ui/button";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../ui/tooltip";
import { cn } from "../../lib/utils";
import type { BulkAction } from "./types";

const segmentButtonClassName =
	"h-8 w-8 shrink-0 rounded-none border-0 shadow-none bg-muted/30 hover:bg-accent hover:text-accent-foreground";

type BulkSelectionToolbarProps = {
	isBulkMode: boolean;
	onToggleMode: () => void;
	modeToggleLabel: string;
	modeExitLabel: string;
	actions: BulkAction[];
	className?: string;
};

export function BulkSelectionToolbar({
	isBulkMode,
	onToggleMode,
	modeToggleLabel,
	modeExitLabel,
	actions,
	className,
}: BulkSelectionToolbarProps) {
	return (
		<TooltipProvider delayDuration={200}>
			<div
				className={cn(
					"inline-flex shrink-0 items-center",
					isBulkMode &&
					"gap-px overflow-hidden rounded-lg border border-input bg-border/80 shadow-sm",
					className,
				)}
			>
				{isBulkMode
					? actions.map((action) => (
						<Tooltip key={action.id}>
							<TooltipTrigger asChild>
								<Button
									type="button"
									variant="ghost"
									size="icon"
									className={cn(
										segmentButtonClassName,
										action.variant === "secondary" &&
										"text-muted-foreground hover:text-foreground",
										action.variant === "default" &&
										"bg-primary text-primary-foreground hover:bg-primary/90 hover:text-primary-foreground",
									)}
									disabled={action.disabled}
									aria-label={action.label}
									onClick={action.onClick}
								>
									<action.icon className="h-4 w-4" />
								</Button>
							</TooltipTrigger>
							<TooltipContent side="bottom">{action.label}</TooltipContent>
						</Tooltip>
					))
					: null}
				<Tooltip>
					<TooltipTrigger asChild>
						<Button
							type="button"
							variant={isBulkMode ? "default" : "outline"}
							size="icon"
							className={cn(
								isBulkMode
									? cn(
										segmentButtonClassName,
										"bg-primary text-primary-foreground hover:bg-primary/90 hover:text-primary-foreground",
									)
									: "h-8 w-8 rounded-md",
							)}
							aria-label={isBulkMode ? modeExitLabel : modeToggleLabel}
							aria-pressed={isBulkMode}
							onClick={onToggleMode}
						>
							<ListChecks className="h-4 w-4" />
						</Button>
					</TooltipTrigger>
					<TooltipContent side="bottom">
						{isBulkMode ? modeExitLabel : modeToggleLabel}
					</TooltipContent>
				</Tooltip>
			</div>
		</TooltipProvider>
	);
}
