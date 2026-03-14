import React from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { type DocNav, type DocPage, type DocGroup, docsNav, findRouteByPath, flattenPages } from "../nav";
import {
	Sidebar as UISidebar,
	SidebarContent,
	SidebarGroup,
	SidebarGroupContent,
	SidebarMenu,
	SidebarMenuButton,
	SidebarMenuItem,
} from "../../components/ui/sidebar";

const WipTag = () => (
	<span className="ml-1 inline-flex items-center rounded-sm bg-amber-100 px-1 py-px text-[9px] font-semibold uppercase tracking-wide text-amber-800 leading-none opacity-0 transition-opacity duration-150 group-hover:opacity-100 group-focus-visible:opacity-100">
		wip
	</span>
);

const renderMenuLabel = (title: React.ReactNode) => (
	<span className="inline-flex items-center gap-1">
		<span className="leading-tight">{title}</span>
		<WipTag />
	</span>
);

export default function Sidebar({ topPx }: { topPx?: number }) {
	const location = useLocation();
	const locale = location.pathname.startsWith("/docs/zh") ? "zh" : "en";
	const nav = React.useMemo<DocNav | undefined>(
		() => docsNav.find((n) => n.locale === locale),
		[locale],
	);
	const navigate = useNavigate();
	const [, startTransition] = React.useTransition();
	const [query, setQuery] = React.useState("");

	const flat = React.useMemo(() => (nav ? flattenPages(nav) : []), [nav]);
	const current = findRouteByPath(flat, location.pathname);

	// Ref for the scrollable sidebar content container
	const scrollContainerRef = React.useRef<HTMLDivElement>(null);

	const [openGroup, setOpenGroup] = React.useState<string | null>(() => {
		// Default to the first group being open
		if (nav && nav.groups.length > 0) {
			const firstGroup = nav.groups.find(
				(g) => g.group && g.group.trim() !== "",
			);
			return firstGroup ? firstGroup.group : null;
		}
		return null;
	});

	// Auto-collapse when navigating to root-level pages, or auto-expand when navigating to group pages
	React.useEffect(() => {
		if (current) {
			// Check if current page is a root-level page (quickstart, changelog, roadmap)
			const isRootLevelPage =
				current.path.includes("/quickstart") ||
				current.path.includes("/changelog") ||
				current.path.includes("/roadmap");

			if (isRootLevelPage) {
				setOpenGroup(null);
			} else {
				// Auto-expand the group that contains the current page
				if (nav) {
					for (const group of nav.groups) {
						if (group.group && group.group.trim() !== "") {
							// Check if current page is in this group
							const isInGroup = group.pages.some((p: DocPage | DocGroup) => {
								if ("path" in p) {
									return p.path === current.path;
								}
								// Check sub-groups
								if ("group" in p) {
									return p.pages.some((sp: DocPage) => sp.path === current.path);
								}
								return false;
							});
							
							if (isInGroup) {
								setOpenGroup(group.group);
								break;
							}
						}
					}
				}
			}
		}
	}, [current, nav]);

	// Scroll to the active menu item when the route changes
	React.useEffect(() => {
		if (current && scrollContainerRef.current) {
			// Small delay to allow DOM updates from group expand/collapse
			const timer = setTimeout(() => {
				const activeButton = scrollContainerRef.current?.querySelector(
					'[data-active="true"]',
				);
				if (activeButton) {
					activeButton.scrollIntoView({
						behavior: "smooth",
						block: "nearest",
					});
				}
			}, 100);
			return () => clearTimeout(timer);
		}
	}, [current]);

	// Measure container box to pin a fixed sidebar aligned with the layout grid
	const containerRef = React.useRef<HTMLDivElement>(null);
	const [box, setBox] = React.useState<{ left: number; width: number } | null>(
		null,
	);
	const [bottomPad, setBottomPad] = React.useState<number>(16);

	React.useLayoutEffect(() => {
		const calc = () => {
			const el = containerRef.current;
			if (el) {
				const rect = el.getBoundingClientRect();
				setBox({ left: Math.round(rect.left), width: Math.round(rect.width) });
			}
			// Compute safe bottom padding so the fixed box won't overlap footer
			const footer = document.querySelector("footer");
			const vh = window.innerHeight;
			const footerTop = footer
				? (footer as HTMLElement).getBoundingClientRect().top
				: Number.POSITIVE_INFINITY;
			const overlap = Math.max(0, vh - Math.floor(footerTop));
			setBottomPad(16 + overlap); // base gap 16px + any overlap amount
		};
		calc();
		window.addEventListener("resize", calc, { passive: true });
		window.addEventListener("scroll", calc, { passive: true });
		return () => {
			window.removeEventListener("resize", calc, { passive: true });
			window.removeEventListener("scroll", calc, { passive: true });
		};
	}, []);

	if (!nav) return null;

	return (
		<div ref={containerRef}>
			<div
				className="not-prose rounded-xl border border-slate-200 dark:border-slate-700 bg-white/70 dark:bg-slate-900/40 p-3 flex flex-col min-h-0"
				style={{
					position: "fixed",
					top: topPx ?? 96,
					left: box?.left ?? 0,
					width: box?.width ?? 288,
					bottom: bottomPad,
				}}
			>
				<div className="mb-3">
					<input
						value={query}
						onChange={(e) => setQuery(e.target.value)}
						placeholder={locale === "zh" ? "搜索文档…" : "Search docs…"}
						className="w-full rounded-md border border-slate-300 dark:border-slate-700 bg-white/70 dark:bg-slate-800/70 px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-blue-500"
					/>
					{query && (
						<div className="mt-2 rounded-md border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 max-h-72 overflow-auto">
							{flat
								.filter((p) =>
									(
										p.title +
										" " +
										(p.summary || "") +
										" " +
										(p.keywords?.join(" ") || "")
									)
										.toLowerCase()
										.includes(query.toLowerCase()),
								)
								.slice(0, 8)
								.map((p) => (
									<button
										key={p.path}
										type="button"
										onClick={() => {
											startTransition(() => {
												navigate(p.path);
											});
										}}
										className="block w-full text-left px-3 py-2 text-sm hover:bg-slate-50 dark:hover:bg-slate-700"
									>
										<div className="font-medium">{p.title}</div>
										{p.summary ? (
											<div className="text-xs text-slate-500 line-clamp-1">
												{p.summary}
											</div>
										) : null}
									</button>
								))}
							{!flat.some((p) =>
								(p.title + (p.summary || ""))
									.toLowerCase()
									.includes(query.toLowerCase()),
							) && (
								<div className="px-3 py-2 text-xs text-slate-500">
									{locale === "zh" ? "无匹配结果" : "No results"}
								</div>
							)}
						</div>
					)}
				</div>

				<div ref={scrollContainerRef} className="flex-1 min-h-0 overflow-auto pr-1">
					<UISidebar>
						<SidebarContent>
							{nav.groups.map((g) => {
								const isRoot = !g.group || g.group.trim() === "";
								if (isRoot) {
									return (
										<SidebarGroup key={g.group || "__root"}>
											<SidebarMenu>
												{g.pages.map((p: DocPage | DocGroup) => (
													"path" in p ? (
														<SidebarMenuItem key={p.path}>
															<Link to={p.path}>
																<SidebarMenuButton
																	active={current?.path === p.path}
																	onClick={() => {
																		// Collapse all collapsible menu items when clicking root-level items
																		setOpenGroup(null);
																	}}
																	onMouseEnter={() => {
																		// Preload the component on hover
																		p.component();
																	}}
																>
																	{renderMenuLabel(p.title)}
																</SidebarMenuButton>
															</Link>
														</SidebarMenuItem>
													) : null
												))}
											</SidebarMenu>
										</SidebarGroup>
									);
								}

								const collapsed = openGroup !== g.group;
								const indent =
									g.group === (locale === "zh" ? "功能特性" : "Features") ||
									g.group === (locale === "zh" ? "操作指南" : "Guides");
								return (
									<SidebarGroup key={g.group}>
										<button
											type="button"
											onClick={() => {
												// Toggle current group: if it's open, close it; if it's closed, open it
												setOpenGroup(openGroup === g.group ? null : g.group);
											}}
											className="group w-full text-left rounded-md px-2 py-2.5 transition-colors hover:bg-slate-100 dark:hover:bg-slate-800 flex items-center justify-between"
											aria-expanded={!collapsed}
										>
											<span className="inline-flex items-center gap-1">
												<span className="leading-tight">{g.group}</span>
												<WipTag />
											</span>
											<span
												className={`transition-transform ${collapsed ? "rotate-180" : ""}`}
											>
												▾
											</span>
										</button>
										{!collapsed && (
											<SidebarGroupContent
												className={`${indent ? "ml-2 pl-2 border-l border-slate-200 dark:border-slate-700" : ""}`}
											>
												<SidebarMenu>
													{g.pages.map((p) => {
														if ("group" in p) {
															const sg: DocGroup = p;
															const key = `${g.group}/${sg.group}`;
															const opened = openGroup === key;
															return (
																<div
																	key={sg.group}
																	className="mb-2 ml-3 pl-2 border-l border-slate-200 dark:border-slate-700"
																>
																	<button
																		type="button"
																		onClick={() => {
																			// Toggle sub-group: if it's open, close it; if it's closed, open it
																			setOpenGroup(opened ? null : key);
																		}}
																		className="group w-full flex items-center justify-between px-2 py-1 text-[11px] font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400 hover:text-slate-300"
																		aria-expanded={opened}
																	>
																		<span className="inline-flex items-center gap-1">
																			<span>{sg.group}</span>
																			<WipTag />
																		</span>
																		<span
																			className={`transition-transform ${opened ? "" : "rotate-180"}`}
																		>
																			▾
																		</span>
																	</button>
																	{opened && (
																		<div className="mt-1">
																			{sg.pages.map((sp: DocPage) => (
																				<SidebarMenuItem key={sp.path}>
																					<Link to={sp.path}>
																					<SidebarMenuButton
																							active={current?.path === sp.path}
																							onMouseEnter={() => {
																								// Preload the component on hover
																								sp.component();
																							}}
																						>
																							{renderMenuLabel(sp.title)}
																						</SidebarMenuButton>
																					</Link>
																				</SidebarMenuItem>
																			))}
																		</div>
																	)}
																</div>
															);
														}
														const sp: DocPage = p as DocPage;
														return (
															<SidebarMenuItem key={sp.path}>
																<Link to={sp.path}>
																	<SidebarMenuButton
																		active={current?.path === sp.path}
																		onMouseEnter={() => {
																			// Preload the component on hover
																			sp.component();
																		}}
																	>
																		{renderMenuLabel(sp.title)}
																	</SidebarMenuButton>
																</Link>
															</SidebarMenuItem>
														);
													})}
												</SidebarMenu>
											</SidebarGroupContent>
										)}
									</SidebarGroup>
								);
							})}
						</SidebarContent>
					</UISidebar>
				</div>

				{/* Language switch is unified in global Footer; no per-sidebar controls */}
			</div>
		</div>
	);
}
