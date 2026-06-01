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

export function getMarketingScrollPadding(): number {
	const bannerHeight =
		Number.parseInt(
			getComputedStyle(document.documentElement).getPropertyValue("--banner-height"),
			10,
		) || 0;
	const header = document.querySelector("header");
	const headerHeight = header?.getBoundingClientRect().height ?? 72;

	return bannerHeight + headerHeight;
}

export function syncMarketingScrollPadding(): void {
	document.documentElement.style.setProperty(
		"--marketing-scroll-padding",
		`${getMarketingScrollPadding()}px`,
	);
}

export function getMarketingSectionScrollTop(element: HTMLElement, id: string): number {
	if (id === "hero") {
		return 0;
	}

	const scrollPadding = getMarketingScrollPadding();
	const sectionTop = window.scrollY + element.getBoundingClientRect().top;
	return Math.max(0, sectionTop - scrollPadding);
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

/** Pick the section whose top is closest to the header anchor and still intersects the viewport. */
export function resolveActiveMarketingSection(
	sectionIds: readonly string[],
): string | null {
	if (isInFooterScrollZone()) {
		return null;
	}

	const anchor = getMarketingScrollPadding();
	let activeId: string | null = null;
	let closestDistance = Number.POSITIVE_INFINITY;

	for (const id of sectionIds) {
		const element = document.getElementById(id);
		if (!element) {
			continue;
		}

		const rect = element.getBoundingClientRect();
		if (rect.bottom <= anchor || rect.top >= window.innerHeight) {
			continue;
		}

		const distance = Math.abs(rect.top - anchor);
		if (distance < closestDistance) {
			closestDistance = distance;
			activeId = id;
		}
	}

	return activeId;
}
