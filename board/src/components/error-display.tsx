import type { LucideIcon } from "lucide-react";
import { RefreshCw } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "./ui/alert";
import { Button } from "./ui/button";

interface ErrorDisplayProps {
	title?: string;
	error: Error | string | null;
	onRetry?: () => void;
	retryLabel?: string;
	icon?: LucideIcon;
}

export function ErrorDisplay({
	title = "Error",
	error,
	onRetry,
	retryLabel = "Retry",
	icon: Icon,
}: ErrorDisplayProps) {
	if (!error) {
		return null;
	}

	const errorMessage =
		typeof error === "string"
			? error
			: (error.message || "An unknown error occurred");

	return (
		<Alert variant="destructive" className="my-4">
			{Icon ? <Icon className="h-4 w-4" /> : null}
			<AlertTitle className="flex items-center justify-between">
				{title}
				{onRetry ? (
					<Button
						variant="outline"
						size="sm"
						onClick={onRetry}
						className="ml-2"
					>
						<RefreshCw className="mr-2 h-4 w-4" />
						{retryLabel}
					</Button>
				) : null}
			</AlertTitle>
			<AlertDescription className="mt-2">{errorMessage}</AlertDescription>
		</Alert>
	);
}
