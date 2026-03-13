import { cn } from "@/lib/utils";

type SpinnerSize = "sm" | "md" | "lg";

type SpinnerProps = {
	size?: SpinnerSize;
	className?: string;
};

const sizeMap: Record<SpinnerSize, string> = {
	sm: "h-4 w-4 border",
	md: "h-6 w-6 border-2",
	lg: "h-8 w-8 border-2",
};

export function Spinner({ size = "md", className }: SpinnerProps) {
	return (
		<div
			className={cn(
				"inline-flex items-center justify-center rounded-full border-slate-200 border-t-primary/70 animate-spin",
				sizeMap[size],
				className,
			)}
			role="status"
			aria-live="polite"
			aria-label="Loading"
		/>
	);
}
