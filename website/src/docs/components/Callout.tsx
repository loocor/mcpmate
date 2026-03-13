import React from "react";
import { AlertCircle, CheckCircle2, Info, AlertTriangle } from "lucide-react";

type CalloutType = "info" | "success" | "warning" | "danger";

const typeMap: Record<CalloutType, { icon: React.ElementType; cls: string }> = {
	info: {
		icon: Info,
		cls: "border-sky-300 bg-sky-50 text-sky-800 dark:bg-sky-950/40 dark:text-sky-300",
	},
	success: {
		icon: CheckCircle2,
		cls: "border-emerald-300 bg-emerald-50 text-emerald-800 dark:bg-emerald-950/40 dark:text-emerald-300",
	},
	warning: {
		icon: AlertTriangle,
		cls: "border-amber-300 bg-amber-50 text-amber-800 dark:bg-amber-950/40 dark:text-amber-300",
	},
	danger: {
		icon: AlertCircle,
		cls: "border-rose-300 bg-rose-50 text-rose-800 dark:bg-rose-950/40 dark:text-rose-300",
	},
};

export default function Callout({
	type = "info",
	title,
	children,
}: {
	type?: CalloutType;
	title?: string;
	children?: React.ReactNode;
}) {
	const { icon: Icon, cls } = typeMap[type];
	return (
		<div
			className={`not-prose border rounded-md p-3 flex gap-3 items-start ${cls}`}
		>
			<Icon className="mt-0.5" size={18} />
			<div>
				{title ? <div className="font-medium mb-1">{title}</div> : null}
				<div className="text-sm leading-6">{children}</div>
			</div>
		</div>
	);
}
