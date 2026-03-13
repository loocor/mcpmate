import React from "react";

export function CardGroup({
	cols = 2,
	children,
}: {
	cols?: 1 | 2 | 3;
	children: React.ReactNode;
}) {
	const grid =
		cols === 1
			? "grid-cols-1"
			: cols === 2
				? "grid-cols-1 sm:grid-cols-2"
				: "grid-cols-1 sm:grid-cols-2 lg:grid-cols-3";
	return <div className={`not-prose grid ${grid} gap-4`}>{children}</div>;
}

export function Card({
	title,
	icon,
	children,
}: {
	title: string;
	icon?: React.ReactNode;
	children: React.ReactNode;
}) {
	return (
		<div className="rounded-lg border border-slate-200 dark:border-slate-700 p-4 bg-white/70 dark:bg-slate-800/60">
			<div className="flex items-center gap-2 mb-2">
				{icon}
				<div className="font-medium">{title}</div>
			</div>
			<div className="text-sm text-slate-700 dark:text-slate-300 leading-6">
				{children}
			</div>
		</div>
	);
}
