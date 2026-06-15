import { Plus, Target } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

import { canIngestFromDataTransfer } from "../lib/server-uni-import-transfer";
import { cn } from "../lib/utils";
import { Button } from "./ui/button";

type ServerImportDropButtonProps = {
	className?: string;
	label?: string;
	onClick: () => void;
	onDrop: (dataTransfer: DataTransfer) => void | Promise<void>;
	title: string;
	variant?: "icon" | "labeled";
};

function isDragLeaveInsideCurrentTarget(
	event: React.DragEvent<HTMLButtonElement>,
): boolean {
	const nextTarget = event.relatedTarget as Node | null;
	return Boolean(nextTarget && event.currentTarget.contains(nextTarget));
}

export function ServerImportDropButton({
	className,
	label,
	onClick,
	onDrop,
	title,
	variant = "icon",
}: ServerImportDropButtonProps) {
	const [dragActive, setDragActive] = useState(false);
	const dragActiveRef = useRef(false);

	const resetDragState = useCallback(() => {
		dragActiveRef.current = false;
		setDragActive(false);
	}, []);

	useEffect(() => {
		window.addEventListener("blur", resetDragState);
		window.addEventListener("dragend", resetDragState);
		window.addEventListener("drop", resetDragState);
		return () => {
			window.removeEventListener("blur", resetDragState);
			window.removeEventListener("dragend", resetDragState);
			window.removeEventListener("drop", resetDragState);
		};
	}, [resetDragState]);

	const handleDragEnter = useCallback(
		(event: React.DragEvent<HTMLButtonElement>) => {
			if (!canIngestFromDataTransfer(event.dataTransfer)) {
				return;
			}
			event.preventDefault();
			event.stopPropagation();
			if (!dragActiveRef.current) {
				dragActiveRef.current = true;
				setDragActive(true);
			}
		},
		[],
	);

	const handleDragOver = useCallback(
		(event: React.DragEvent<HTMLButtonElement>) => {
			if (!canIngestFromDataTransfer(event.dataTransfer)) {
				return;
			}
			event.preventDefault();
			event.stopPropagation();
			event.dataTransfer.dropEffect = "copy";
		},
		[],
	);

	const handleDragLeave = useCallback(
		(event: React.DragEvent<HTMLButtonElement>) => {
			if (isDragLeaveInsideCurrentTarget(event)) {
				return;
			}
			resetDragState();
		},
		[resetDragState],
	);

	const handleDrop = useCallback(
		async (event: React.DragEvent<HTMLButtonElement>) => {
			if (!canIngestFromDataTransfer(event.dataTransfer)) {
				resetDragState();
				return;
			}
			event.preventDefault();
			event.stopPropagation();
			resetDragState();
			await onDrop(event.dataTransfer);
		},
		[onDrop, resetDragState],
	);

	const iconClass = variant === "labeled" ? "mr-2 h-4 w-4" : "h-4 w-4";

	return (
		<Button
			data-desktop-drop-target="server-import"
			size={variant === "icon" ? "icon" : "sm"}
			className={cn(
				variant === "icon" ? "h-9 w-9 transition-colors" : "transition-colors",
				dragActive && "ring-2 ring-slate-300 dark:ring-slate-600",
				className,
			)}
			title={title}
			onClick={onClick}
			onDragEnd={resetDragState}
			onDragEnter={handleDragEnter}
			onDragLeave={handleDragLeave}
			onDragOver={handleDragOver}
			onDrop={handleDrop}
		>
			<Plus className={cn(iconClass, dragActive && "hidden")} />
			<Target className={cn(iconClass, !dragActive && "hidden")} />
			{label}
		</Button>
	);
}
