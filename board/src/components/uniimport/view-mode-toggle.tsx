import { memo, useCallback } from "react";
import type { KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

export type FormViewMode = "form" | "json";

interface FormViewModeToggleProps {
	mode: FormViewMode;
	onChange: (mode: FormViewMode) => void;
	variant?: "default" | "compact";
	className?: string;
}

export const FormViewModeToggle = memo(function FormViewModeToggle({
	mode,
	onChange,
	variant = "default",
	className,
}: FormViewModeToggleProps) {
	const { t } = useTranslation("servers");
	const handleForm = useCallback(() => {
		onChange("form");
	}, [onChange]);

	const handleJson = useCallback(() => {
		onChange("json");
	}, [onChange]);

	const handleKeyDown = useCallback(
		(event: KeyboardEvent<HTMLButtonElement>, nextMode: FormViewMode) => {
			if (event.key === "Enter" || event.key === " ") {
				event.preventDefault();
				onChange(nextMode);
			}
		},
		[onChange],
	);

	const sizeClasses =
		variant === "compact" ? "px-2 py-1 text-xs" : "px-3 py-1 text-sm";
	const containerClasses =
		variant === "compact"
			? "flex rounded-md border border-input text-xs"
			: "flex rounded-md border border-input";

	return (
		<div className={`${containerClasses} ${className ?? ""}`}>
			<button
				type="button"
				onClick={handleForm}
				onKeyDown={(event) => handleKeyDown(event, "form")}
				className={`rounded-l-md rounded-r-none font-medium transition-colors ${sizeClasses} ${
					mode === "form"
						? "bg-primary text-primary-foreground"
						: "text-slate-600 hover:text-slate-900 dark:text-slate-300 dark:hover:text-slate-100"
				}`}
			>
				{t("manual.viewMode.form", { defaultValue: "Form" })}
			</button>
			<button
				type="button"
				onClick={handleJson}
				onKeyDown={(event) => handleKeyDown(event, "json")}
				className={`rounded-r-md rounded-l-none font-medium transition-colors ${sizeClasses} ${
					mode === "json"
						? "bg-primary text-primary-foreground"
						: "text-slate-600 hover:text-slate-900 dark:text-slate-300 dark:hover:text-slate-100"
				}`}
			>
				{t("manual.viewMode.json", { defaultValue: "JSON" })}
			</button>
		</div>
	);
});
