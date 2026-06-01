import { useEffect, useState } from "react";
import {
	resolveActiveMarketingSection,
	syncMarketingScrollPadding,
} from "../lib/section-scroll";

export function useScrollSpy(sectionIds: readonly string[], enabled: boolean): string | null {
	const [activeId, setActiveId] = useState<string | null>(null);

	useEffect(() => {
		if (!enabled) {
			setActiveId(null);
			return;
		}

		let frame = 0;
		let retryTimer: ReturnType<typeof setTimeout> | undefined;
		let attached = false;

		const update = () => {
			syncMarketingScrollPadding();
			const next = resolveActiveMarketingSection(sectionIds);
			setActiveId(next);
		};

		const onScroll = () => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(update);
		};

		const attach = () => {
			const hasSections = sectionIds.some((id) => document.getElementById(id));
			if (!hasSections) {
				retryTimer = setTimeout(attach, 50);
				return;
			}

			attached = true;
			update();
			window.addEventListener("scroll", onScroll, { passive: true });
			window.addEventListener("resize", onScroll);
		};

		attach();

		return () => {
			cancelAnimationFrame(frame);
			clearTimeout(retryTimer);
			if (attached) {
				window.removeEventListener("scroll", onScroll);
				window.removeEventListener("resize", onScroll);
			}
		};
	}, [enabled, sectionIds]);

	return activeId;
}
