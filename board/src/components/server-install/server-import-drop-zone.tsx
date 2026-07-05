import { Loader2, Target } from "lucide-react";
import {
	forwardRef,
	useEffect,
	type ClipboardEvent,
	type DragEvent,
	type MouseEvent,
} from "react";
import { cn } from "../../lib/utils";
import { breathingAnimation } from "./types";

type ServerImportDropZoneProps = {
	collapsed: boolean;
	message: string;
	error?: string | null;
	success?: boolean;
	dragOver?: boolean;
	ingesting?: boolean;
	pasteTipPrefix?: string;
	pasteShortcut?: string;
	pasteTipSuffix?: string;
	className?: string;
	onClick?: (event: MouseEvent<HTMLButtonElement>) => void;
	onDrop?: (event: DragEvent<HTMLButtonElement>) => void;
	onDragOver?: (event: DragEvent<HTMLButtonElement>) => void;
	onDragEnter?: (event: DragEvent<HTMLButtonElement>) => void;
	onDragLeave?: (event: DragEvent<HTMLButtonElement>) => void;
	onPaste?: (event: ClipboardEvent<HTMLButtonElement>) => void;
};

export const ServerImportDropZone = forwardRef<
	HTMLButtonElement,
	ServerImportDropZoneProps
>(
	(
		{
			collapsed,
			message,
			error,
			success = false,
			dragOver = false,
			ingesting = false,
			pasteTipPrefix = "Tip: press",
			pasteShortcut = "Ctrl/Cmd + V",
			pasteTipSuffix = "to paste instantly.",
			className,
			onClick,
			onDrop,
			onDragOver,
			onDragEnter,
			onDragLeave,
			onPaste,
		},
		ref,
	) => {
		useEffect(() => {
			if (document.getElementById("breathing-animation")) return;
			const style = document.createElement("style");
			style.id = "breathing-animation";
			style.textContent = breathingAnimation;
			document.head.appendChild(style);
		}, []);

		const showPasteTip = !collapsed && !error;

		return (
			<button
				data-desktop-drop-target="server-import"
				ref={ref}
				type="button"
				onDrop={onDrop}
				onDragOver={onDragOver}
				onDragEnter={onDragEnter}
				onDragLeave={onDragLeave}
				onPaste={onPaste}
				onClick={onClick}
				className={cn(
					"w-full cursor-pointer overflow-hidden focus:outline-none",
					"transition-[height] duration-300 ease-in-out",
					collapsed ? "h-10" : "h-[18vh]",
					className,
				)}
				style={{ border: "none" }}
			>
				<div
					className={cn(
						"flex h-full w-full items-center justify-center gap-4 rounded-lg border border-dashed",
						"transition-all duration-300 ease-in-out",
						collapsed
							? "flex-row px-4 py-2 border-slate-200 bg-slate-50 dark:border-slate-700 dark:bg-slate-900/40"
							: "flex-col py-8 border-slate-300 bg-slate-50 dark:border-slate-700 dark:bg-slate-900/40",
						error
							? "border-red-300 bg-red-50 dark:border-red-700 dark:bg-red-900/20"
							: success
								? "border-green-300 bg-green-50 dark:border-green-700 dark:bg-green-900/20"
								: dragOver
									? "border-blue-300 bg-blue-50 dark:border-blue-700 dark:bg-blue-900/20"
									: null,
					)}
				>
					{ingesting ? (
						<Loader2
							className={cn(
								"shrink-0 animate-spin transition-all duration-300 ease-in-out",
								collapsed ? "size-4" : "size-6",
							)}
						/>
					) : (
						<Target
							className={cn(
								"shrink-0 transition-all duration-300 ease-in-out",
								collapsed ? "size-4" : "size-12",
								dragOver || ingesting ? "animate-pulse" : "scale-100",
								dragOver ? "text-blue-500" : "text-slate-500",
							)}
							style={{
								animation:
									error || dragOver || ingesting
										? "breathing 1.5s ease-in-out infinite"
										: undefined,
							}}
						/>
					)}

					<div
						className={cn(
							"min-w-0 transition-all duration-300 ease-in-out",
							collapsed
								? "max-w-full text-left"
								: "max-w-full text-center",
						)}
					>
						<p
							className={cn(
								"leading-relaxed transition-all duration-300 ease-in-out",
								collapsed ? "truncate text-sm" : "max-w-none px-4 text-sm",
								error
									? "text-red-600 dark:text-red-400"
									: success
										? "text-green-600 dark:text-green-400"
										: dragOver
											? "text-blue-600 dark:text-blue-400"
											: "text-slate-600 dark:text-slate-300",
								(ingesting || dragOver) && "animate-pulse",
							)}
						>
							{error || message}
						</p>
						<p
							className={cn(
								"overflow-hidden text-xs text-slate-400 transition-all duration-300 ease-in-out",
								showPasteTip
									? "mt-2 max-h-8 opacity-100"
									: "mt-0 max-h-0 opacity-0",
							)}
							aria-hidden={!showPasteTip}
						>
							{pasteTipPrefix}{" "}
							<kbd className="rounded bg-slate-200 px-1 text-[10px]">
								{pasteShortcut}
							</kbd>{" "}
							{pasteTipSuffix}
						</p>
					</div>
				</div>
			</button>
		);
	},
);

ServerImportDropZone.displayName = "ServerImportDropZone";
