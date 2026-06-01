import { useEffect, useState } from "react";
import {
	ambientStateToStyleProperties,
	computeMarketingAmbientState,
	type MarketingAmbientState,
} from "../lib/marketing-ambient";
import { getMarketingScrollPadding, syncMarketingScrollPadding } from "../lib/section-scroll";

const INITIAL_STATE: MarketingAmbientState = {
	blobs: [
		{ top: -10, left: -6, width: 52, height: 46, opacity: 1 },
		{ top: 56, left: 58, width: 48, height: 42, opacity: 1 },
		{ top: 40, left: 50, width: 40, height: 38, opacity: 0.55 },
	],
	layerOpacity: 1,
};

export function useMarketingAmbientScroll(enabled: boolean): Record<string, string> {
	const [style, setStyle] = useState<Record<string, string>>(() =>
		ambientStateToStyleProperties(INITIAL_STATE),
	);

	useEffect(() => {
		if (!enabled) {
			return;
		}

		let frame = 0;
		let retryTimer: ReturnType<typeof setTimeout> | undefined;
		let attached = false;
		const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

		const update = () => {
			syncMarketingScrollPadding();
			const state = computeMarketingAmbientState(getMarketingScrollPadding());
			setStyle(ambientStateToStyleProperties(state));
		};

		const onScroll = () => {
			if (reducedMotion) {
				update();
				return;
			}
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(update);
		};

		const attach = () => {
			if (!document.getElementById("hero")) {
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
	}, [enabled]);

	return style;
}
