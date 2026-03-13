import { ListTree } from "lucide-react";
import React from "react";
import { useDocContext } from "../context/DocContext";

export default function ToC() {
	const { headings } = useDocContext();
	const [active, setActive] = React.useState<string>("");

	React.useEffect(() => {
		const obs = new IntersectionObserver(
			(entries) => {
				entries.forEach((e) => {
					if (e.isIntersecting) {
						const id = (e.target as HTMLElement).id;
						if (id) setActive(id);
					}
				});
			},
			{ rootMargin: "-40% 0px -55% 0px", threshold: [0, 1] },
		);
		headings.forEach((h) => h.el && obs.observe(h.el));
		return () => obs.disconnect();
	}, [headings]);

	if (!headings.length) return null;

	return (
		<div className="sticky top-40 flex justify-end">
			<div className="group/toc relative inline-flex">
				<button
					type="button"
					className="flex h-11 w-11 items-center justify-center rounded-full border border-slate-200/70 bg-white/20 text-slate-600 shadow-sm backdrop-blur transition-colors duration-200 hover:text-blue-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:border-slate-700/60 dark:bg-slate-950/30 dark:text-slate-300 dark:focus-visible:ring-offset-slate-950"
					aria-label="Open table of contents"
				>
					<ListTree className="h-5 w-5" strokeWidth={2} />
				</button>
				<div className="pointer-events-none absolute right-0 top-0 z-20 origin-top-right translate-y-2 scale-95 opacity-0 transition-all duration-200 ease-out group-hover/toc:pointer-events-auto group-hover/toc:translate-y-0 group-hover/toc:scale-100 group-hover/toc:opacity-100 group-focus-within/toc:pointer-events-auto group-focus-within/toc:translate-y-0 group-focus-within/toc:scale-100 group-focus-within/toc:opacity-100">
					<div className="not-prose mt-0 max-h-[70vh] min-w-[15rem] overflow-auto rounded-2xl border border-slate-200/60 bg-white/25 p-4 shadow-xl backdrop-blur-md dark:border-slate-700/60 dark:bg-slate-900/45">
						<div className="mb-3 text-xs font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">
							On this page
						</div>
						<nav className="space-y-1 text-sm border-l border-slate-200/40 pl-3 dark:border-slate-700/40">
							{headings.map((h) => (
								<a
									key={h.id}
									href={`#${h.id}`}
									className={`block truncate transition-colors hover:text-blue-600 dark:hover:text-blue-400 ${
										active === h.id
											? "text-blue-600 dark:text-blue-400"
											: "text-slate-600 dark:text-slate-300"
									} ${h.level === 3 ? "ml-3" : ""}`}
								>
									{h.text}
								</a>
							))}
						</nav>
					</div>
				</div>
			</div>
		</div>
	);
}
