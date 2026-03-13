import React from "react";
import { ChevronDown } from "lucide-react";

export function AccordionGroup({ children }: { children: React.ReactNode }) {
	return (
		<div className="not-prose divide-y divide-slate-200 dark:divide-slate-700 rounded-md border border-slate-200 dark:border-slate-700 overflow-hidden">
			{children}
		</div>
	);
}

export function Accordion({
	title,
	children,
	defaultOpen = false,
}: {
	title: string;
	children: React.ReactNode;
	defaultOpen?: boolean;
}) {
	const [open, setOpen] = React.useState(defaultOpen);
	return (
		<div>
			<button
				className="w-full flex items-center justify-between px-4 py-3 text-left bg-slate-50 dark:bg-slate-800/40 hover:bg-slate-100 dark:hover:bg-slate-800"
				onClick={() => setOpen((v) => !v)}
				aria-expanded={open}
			>
				<span className="font-medium">{title}</span>
				<ChevronDown
					className={`transition-transform ${open ? "rotate-180" : ""}`}
					size={18}
				/>
			</button>
			{open && (
				<div className="px-4 py-3 text-sm text-slate-700 dark:text-slate-300">
					{children}
				</div>
			)}
		</div>
	);
}
