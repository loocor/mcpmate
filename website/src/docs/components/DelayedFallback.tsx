import React from "react";

export function DelayedFallback({ delay = 400 }: { delay?: number }) {
	const [show, setShow] = React.useState(false);

	React.useEffect(() => {
		const timer = setTimeout(() => setShow(true), delay);
		return () => clearTimeout(timer);
	}, [delay]);

	if (!show) return null;

	return (
		<div
			className="relative mx-auto w-full max-w-7xl px-4 md:px-6 py-8 md:py-10 animate-in fade-in duration-200"
		>
			<div className="flex gap-6 mt-2 md:mt-4">
				{/* Sidebar skeleton */}
				<aside className="hidden md:block w-72 shrink-0">
					<div className="space-y-2">
						<div className="h-10 bg-slate-200 dark:bg-slate-700 rounded animate-pulse"></div>
						<div className="h-10 bg-slate-200 dark:bg-slate-700 rounded animate-pulse"></div>
						<div className="h-10 bg-slate-200 dark:bg-slate-700 rounded animate-pulse"></div>
					</div>
				</aside>
				{/* Content skeleton */}
				<article className="group flex-1 min-w-0">
					<div className="h-8 bg-slate-200 dark:bg-slate-700 rounded mb-4 w-1/2 animate-pulse"></div>
					<div className="space-y-3">
						<div className="h-4 bg-slate-200 dark:bg-slate-700 rounded w-3/4 animate-pulse"></div>
						<div className="h-4 bg-slate-200 dark:bg-slate-700 rounded w-1/2 animate-pulse"></div>
						<div className="h-4 bg-slate-200 dark:bg-slate-700 rounded w-5/6 animate-pulse"></div>
						<div className="h-4 bg-slate-200 dark:bg-slate-700 rounded w-2/3 animate-pulse"></div>
					</div>
				</article>
			</div>
		</div>
	);
}
