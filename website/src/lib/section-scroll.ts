/** Homepage navigation items (excludes hero — logo scrolls to top). */
export const MARKETING_NAV_ITEMS = [
	{ id: "features", labelKey: "features.title" },
	{ id: "clients", labelKey: "nav.works_with" },
	{ id: "how-it-works", labelKey: "nav.how" },
	{ id: "modes", labelKey: "nav.modes" },
	{ id: "faq", labelKey: "nav.faq" },
] as const;

export const MARKETING_NAV_SECTIONS = MARKETING_NAV_ITEMS.map((item) => item.id);

export type MarketingNavSectionId = (typeof MARKETING_NAV_ITEMS)[number]["id"];

const FOOTER_SPY_CLEARANCE_PX = 48;

export const MARKETING_FAQ_NAVIGATION_EVENT = "mcpmate:faq-navigation";

interface MarketingScrollFrame {
	scrollPadding: number;
	safeTop: number;
	safeBottom: number;
	safeHeight: number;
}

function collapseFaqForNavigation(): void {
	document.querySelectorAll<HTMLDetailsElement>("#faq details[open]").forEach((details) => {
		details.open = false;
	});
	window.dispatchEvent(new Event(MARKETING_FAQ_NAVIGATION_EVENT));
}

function getMarketingScrollTarget(section: HTMLElement): HTMLElement {
	return section.querySelector<HTMLElement>("[data-marketing-scroll-content]") ?? section;
}

function getSettledHeaderHeight(header: HTMLElement | null): number {
	if (!header) {
		return 72;
	}

	const rect = header.getBoundingClientRect();
	const style = getComputedStyle(header);
	const paddingTop = Number.parseFloat(style.paddingTop) || 0;
	const paddingBottom = Number.parseFloat(style.paddingBottom) || 0;
	const rootFontSize =
		Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	const compactVerticalPadding = rootFontSize * 1.5;
	const compactHeight = rect.height - paddingTop - paddingBottom + compactVerticalPadding;

	return Math.min(rect.height, compactHeight);
}

function getMarketingScrollFrame(): MarketingScrollFrame {
	const bannerHeight =
		Number.parseInt(
			getComputedStyle(document.documentElement).getPropertyValue("--banner-height"),
			10,
		) || 0;
	const header = document.querySelector("header");
	const headerHeight = getSettledHeaderHeight(header);
	const scrollPadding = bannerHeight + headerHeight;
	const safeTop = scrollPadding + headerHeight;
	const safeBottom = Math.max(safeTop, window.innerHeight - headerHeight);

	return {
		scrollPadding,
		safeTop,
		safeBottom,
		safeHeight: Math.max(0, safeBottom - safeTop),
	};
}

export function getMarketingScrollPadding(): number {
	return getMarketingScrollFrame().scrollPadding;
}

export function syncMarketingScrollPadding(): void {
	const frame = getMarketingScrollFrame();
	document.documentElement.style.setProperty(
		"--marketing-scroll-padding",
		`${frame.scrollPadding}px`,
	);
	document.documentElement.style.setProperty(
		"--marketing-scroll-safe-top",
		`${frame.safeTop}px`,
	);
}

export function getMarketingSectionScrollTop(element: HTMLElement, id: string): number {
	if (id === "hero") {
		return 0;
	}

	const frame = getMarketingScrollFrame();
	const target = getMarketingScrollTarget(element);
	const targetRect = target.getBoundingClientRect();
	const targetTop = window.scrollY + targetRect.top;
	const targetHeight = targetRect.height;
	let targetViewportTop = frame.safeTop;

	if (frame.safeHeight > 0 && targetHeight <= frame.safeHeight) {
		targetViewportTop = frame.safeTop + (frame.safeHeight - targetHeight) / 2;
	}

	return Math.max(0, targetTop - targetViewportTop);
}

export function scrollToMarketingSection(
	id: string,
	behavior: ScrollBehavior = "smooth",
): void {
	syncMarketingScrollPadding();
	const element = document.getElementById(id);
	if (!element) {
		return;
	}

	if (id === "faq") {
		collapseFaqForNavigation();
	}

	const targetTop = getMarketingSectionScrollTop(element, id);
	window.scrollTo({ top: targetTop, behavior });
}

export function isInFooterScrollZone(): boolean {
	const footer = document.querySelector("footer");
	if (!footer) {
		const maxScroll = document.documentElement.scrollHeight - window.innerHeight;
		return window.scrollY >= maxScroll - FOOTER_SPY_CLEARANCE_PX;
	}

	const footerTop = footer.getBoundingClientRect().top;
	return footerTop <= window.innerHeight - FOOTER_SPY_CLEARANCE_PX;
}

/** Pick the section closest to the visual center of the safe area below the header. */
export function resolveActiveMarketingSection(
	sectionIds: readonly string[],
): string | null {
	if (isInFooterScrollZone()) {
		return null;
	}

	const frame = getMarketingScrollFrame();
	const anchor = frame.safeTop + frame.safeHeight * 0.5;
	let activeId: string | null = null;
	let closestDistance = Number.POSITIVE_INFINITY;

	for (const id of sectionIds) {
		const element = document.getElementById(id);
		if (!element) {
			continue;
		}

		const rect = getMarketingScrollTarget(element).getBoundingClientRect();
		if (rect.bottom <= frame.safeTop || rect.top >= frame.safeBottom) {
			continue;
		}

		const sectionCenter = rect.top + rect.height / 2;
		const distance = Math.abs(sectionCenter - anchor);
		if (distance < closestDistance) {
			closestDistance = distance;
			activeId = id;
		}
	}

	return activeId;
}
