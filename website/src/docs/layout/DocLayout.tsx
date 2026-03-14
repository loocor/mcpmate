import React from "react";
import { DocProvider } from "../context/DocContext";
import Sidebar from "./Sidebar";
import ToC from "./ToC";

export type DocMeta = {
	title: string;
	description?: string;
};

type Props = {
	meta: DocMeta;
	children: React.ReactNode;
};

export default function DocLayout({ meta, children }: Props) {
	// Minimal SEO: set document title; can be replaced by react-helmet later
	React.useEffect(() => {
		if (meta?.title) document.title = `${meta.title} · MCPMate`;
	}, [meta?.title]);

	// Fade-in effect for content to smooth page transitions
	const [fadeIn, setFadeIn] = React.useState(false);
	React.useEffect(() => {
		setFadeIn(false);
		const timer = setTimeout(() => setFadeIn(true), 10);
		return () => clearTimeout(timer);
	}, [meta?.title]);

	const anchorRef = React.useRef<HTMLDivElement>(null);
	// Sticky/top padding base distance from site header
	const [topPx, setTopPx] = React.useState<number>(104);
	const [padTop, setPadTop] = React.useState<number>(104);

	React.useLayoutEffect(() => {
		const EXTRA_GAP = 16; // extra space below header for breathing room
		const MIN_TOP = 96; // minimum to avoid overlap on small screens

		const calc = () => {
			// Measure current header height (Navbar is a fixed <header>)
			const header = document.querySelector("header");
			const headerH = header
				? Math.round(header.getBoundingClientRect().height)
				: 80;
			// Fixed offset tied to header height only — avoids sidebar micro-movement while scrolling
			const computed = Math.max(headerH + EXTRA_GAP, MIN_TOP);
			setTopPx(computed);
			setPadTop(computed); // also push the content down so H1 doesn't tuck under the header
		};

		calc();
		window.addEventListener("resize", calc, { passive: true });
		return () => {
			window.removeEventListener("resize", calc, { passive: true });
		};
	}, []);

	return (
		<DocProvider>
			<div
				className="relative mx-auto w-full max-w-7xl px-4 md:px-6 py-8 md:py-10"
				style={{ paddingTop: padTop }}
			>
				{/* Anchor for calculating sticky offset - placed before content to avoid margin interference */}
				<div ref={anchorRef} aria-hidden className="h-0" />
				{/* Left sidebar (fixed width) + main content (flex) */}
				<div className="flex gap-6 mt-2 md:mt-4">
					<aside className="hidden md:block w-72 shrink-0">
						<Sidebar topPx={topPx} />
					</aside>
					<article
						className={`group flex-1 min-w-0 prose dark:prose-invert max-w-none transition-opacity duration-300 ${fadeIn ? "opacity-100" : "opacity-0"}`}
					>
						<h1 className="mb-4 text-3xl font-bold tracking-tight">
							{meta.title}
						</h1>
						{meta.description ? (
							<p className="-mt-2 mb-6 text-slate-600 dark:text-slate-400">
								{meta.description}
							</p>
						) : null}
						<div className="space-y-5">{children}</div>
					</article>
				</div>

				{/* Floating ToC on large screens (doesn't reserve layout space) */}
				<div
					className="hidden xl:block absolute right-6"
					style={{ top: topPx }}
				>
					<ToC />
				</div>
			</div>
		</DocProvider>
	);
}
