import { Check } from "lucide-react";
import { cn } from "../../lib/utils";

type BulkSelectionCheckboxProps = {
	visible: boolean;
	checked: boolean;
	onToggle: () => void;
	ariaLabel: string;
	className?: string;
};

export function BulkSelectionCheckbox({
	visible,
	checked,
	onToggle,
	ariaLabel,
	className,
}: BulkSelectionCheckboxProps) {
	return (
		<div
			className={cn(
				"shrink-0 overflow-hidden transition-[width,opacity] duration-200",
				visible ? "w-6 opacity-100" : "w-0 opacity-0",
				className,
			)}
			aria-hidden={!visible}
		>
			<button
				type="button"
				tabIndex={visible ? 0 : -1}
				className={cn(
					"flex h-6 w-6 items-center justify-center rounded-full border text-[0px] transition-all duration-200",
					checked
						? "border-primary bg-primary text-white shadow-sm"
						: "border-slate-300 text-transparent hover:border-primary/50 hover:text-primary/60 dark:border-slate-700 dark:hover:border-primary/50",
				)}
				onClick={(event) => {
					event.stopPropagation();
					onToggle();
				}}
				aria-label={ariaLabel}
				aria-pressed={checked}
				disabled={!visible}
			>
				<Check className="h-3 w-3" />
			</button>
		</div>
	);
}
