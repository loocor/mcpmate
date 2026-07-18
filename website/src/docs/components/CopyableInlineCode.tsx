import { AlertCircle, Check, Clipboard, type LucideIcon } from "lucide-react";
import { useEffect, useState } from "react";

type CopyStatus = "idle" | "copied" | "error";

type CopyableInlineCodeProps = {
	children: string;
	copyLabel: string;
	copiedLabel: string;
	errorLabel: string;
};

function getStatusLabel(
	status: CopyStatus,
	labels: Pick<CopyableInlineCodeProps, "copyLabel" | "copiedLabel" | "errorLabel">,
): string {
	if (status === "copied") {
		return labels.copiedLabel;
	}

	if (status === "error") {
		return labels.errorLabel;
	}

	return labels.copyLabel;
}

function getStatusIcon(status: CopyStatus): LucideIcon {
	if (status === "copied") {
		return Check;
	}

	if (status === "error") {
		return AlertCircle;
	}

	return Clipboard;
}

export default function CopyableInlineCode({
	children,
	copyLabel,
	copiedLabel,
	errorLabel,
}: CopyableInlineCodeProps): JSX.Element {
	const [status, setStatus] = useState<CopyStatus>("idle");

	useEffect(() => {
		if (status === "idle") {
			return;
		}

		const timeout = window.setTimeout(() => setStatus("idle"), 1800);
		return () => window.clearTimeout(timeout);
	}, [status]);

	async function handleCopy(): Promise<void> {
		try {
			await navigator.clipboard.writeText(children);
			setStatus("copied");
		} catch {
			setStatus("error");
		}
	}

	const label = getStatusLabel(status, { copyLabel, copiedLabel, errorLabel });
	const Icon = getStatusIcon(status);

	return (
		<span className="group/code relative inline-flex max-w-full align-middle">
			<code className="break-all rounded bg-brand-overlay px-1.5 py-0.5 pr-7 font-mono text-[0.9em] text-brand-foreground">
				{children}
			</code>
			<button
				type="button"
				onClick={() => void handleCopy()}
				aria-label={label}
				title={label}
				className="absolute right-1 top-1/2 inline-flex -translate-y-1/2 items-center justify-center rounded p-0.5 text-brand-muted-soft opacity-0 transition-opacity hover:text-brand-accent focus:opacity-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-accent group-hover/code:opacity-100 group-focus-within/code:opacity-100"
			>
				<Icon size={13} aria-hidden />
			</button>
			<span className="sr-only" aria-live="polite">
				{status === "idle" ? "" : label}
			</span>
		</span>
	);
}
