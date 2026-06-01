import { isInFooterScrollZone, MARKETING_NAV_ITEMS } from "./section-scroll";

export interface AmbientBlobLayout {
	top: number;
	left: number;
	width: number;
	height: number;
	opacity: number;
}

export type AmbientBlobTriplet = [AmbientBlobLayout, AmbientBlobLayout, AmbientBlobLayout];

/** Document order — used for scroll interpolation anchors. */
export const MARKETING_AMBIENT_TRACK = [
	"hero",
	"download",
	...MARKETING_NAV_ITEMS.map((item) => item.id),
] as const;

export type MarketingAmbientTrackId = (typeof MARKETING_AMBIENT_TRACK)[number];

/** Viewport-relative blob layouts (top/left/width/height in %, opacity is a multiplier). */
export const MARKETING_AMBIENT_KEYFRAMES: Record<MarketingAmbientTrackId | "footer", AmbientBlobTriplet> = {
	hero: [
		{ top: -10, left: -6, width: 52, height: 46, opacity: 1 },
		{ top: 56, left: 58, width: 48, height: 42, opacity: 1 },
		{ top: 40, left: 50, width: 40, height: 38, opacity: 0.55 },
	],
	clients: [
		{ top: 6, left: 4, width: 46, height: 40, opacity: 1 },
		{ top: 26, left: 52, width: 48, height: 42, opacity: 1 },
		{ top: 52, left: 26, width: 40, height: 38, opacity: 0.78 },
	],
	features: [
		{ top: 16, left: -4, width: 52, height: 46, opacity: 1 },
		{ top: 30, left: 50, width: 48, height: 42, opacity: 1 },
		{ top: 50, left: 36, width: 40, height: 38, opacity: 0.92 },
	],
	"how-it-works": [
		{ top: 20, left: 10, width: 44, height: 46, opacity: 1 },
		{ top: 36, left: 54, width: 48, height: 42, opacity: 1 },
		{ top: 54, left: 46, width: 40, height: 38, opacity: 0.9 },
	],
	modes: [
		{ top: 12, left: 50, width: 52, height: 46, opacity: 1 },
		{ top: 36, left: 4, width: 48, height: 42, opacity: 1 },
		{ top: 46, left: 40, width: 40, height: 38, opacity: 0.88 },
	],
	download: [
		{ top: 18, left: 6, width: 52, height: 46, opacity: 1.15 },
		{ top: 34, left: 52, width: 48, height: 42, opacity: 1 },
		{ top: 56, left: 30, width: 40, height: 38, opacity: 0.95 },
	],
	faq: [
		{ top: 14, left: 5, width: 46, height: 54, opacity: 1.3 },
		{ top: 22, left: 52, width: 48, height: 42, opacity: 1.2 },
		{ top: 64, left: 28, width: 40, height: 38, opacity: 1 },
	],
	footer: [
		{ top: 52, left: 18, width: 44, height: 40, opacity: 0.85 },
		{ top: 60, left: 56, width: 44, height: 38, opacity: 0.8 },
		{ top: 72, left: 42, width: 36, height: 34, opacity: 0.75 },
	],
};

export interface MarketingAmbientState {
	blobs: AmbientBlobTriplet;
	layerOpacity: number;
}

function smoothstep(value: number): number {
	const t = Math.min(1, Math.max(0, value));
	return t * t * (3 - 2 * t);
}

function lerp(a: number, b: number, t: number): number {
	return a + (b - a) * t;
}

function lerpBlob(from: AmbientBlobLayout, to: AmbientBlobLayout, t: number): AmbientBlobLayout {
	return {
		top: lerp(from.top, to.top, t),
		left: lerp(from.left, to.left, t),
		width: lerp(from.width, to.width, t),
		height: lerp(from.height, to.height, t),
		opacity: lerp(from.opacity, to.opacity, t),
	};
}

function lerpTriplet(from: AmbientBlobTriplet, to: AmbientBlobTriplet, t: number): AmbientBlobTriplet {
	return [
		lerpBlob(from[0], to[0], t),
		lerpBlob(from[1], to[1], t),
		lerpBlob(from[2], to[2], t),
	];
}

function getScrollAnchor(scrollPadding: number): number {
	const bandTop = scrollPadding;
	const bandHeight = Math.max(0, window.innerHeight - scrollPadding);
	return window.scrollY + bandTop + bandHeight * 0.4;
}

function getSectionAnchor(element: HTMLElement): number {
	return element.offsetTop + element.offsetHeight * 0.45;
}

export function computeMarketingAmbientState(scrollPadding: number): MarketingAmbientState {
	const anchor = getScrollAnchor(scrollPadding);

	const anchors = MARKETING_AMBIENT_TRACK.map((id) => {
		const element = document.getElementById(id);
		if (!element) {
			return null;
		}
		return { id, y: getSectionAnchor(element) };
	}).filter((entry): entry is { id: MarketingAmbientTrackId; y: number } => entry !== null);

	if (anchors.length === 0) {
		return { blobs: MARKETING_AMBIENT_KEYFRAMES.hero, layerOpacity: 1 };
	}

	if (anchor <= anchors[0].y) {
		return { blobs: MARKETING_AMBIENT_KEYFRAMES[anchors[0].id], layerOpacity: 1 };
	}

	for (let index = 0; index < anchors.length - 1; index += 1) {
		const current = anchors[index];
		const next = anchors[index + 1];
		if (anchor > next.y) {
			continue;
		}

		const span = Math.max(1, next.y - current.y);
		const t = smoothstep((anchor - current.y) / span);
		return {
			blobs: lerpTriplet(
				MARKETING_AMBIENT_KEYFRAMES[current.id],
				MARKETING_AMBIENT_KEYFRAMES[next.id],
				t,
			),
			layerOpacity: 1,
		};
	}

	const last = anchors[anchors.length - 1];
	const footer = document.querySelector("footer");
	let layerOpacity = 1;
	let blobs = MARKETING_AMBIENT_KEYFRAMES[last.id];

	if (footer) {
		const footerAnchor = footer instanceof HTMLElement ? footer.offsetTop : last.y + 400;
		const span = Math.max(1, footerAnchor - last.y);
		const t = smoothstep((anchor - last.y) / span);
		blobs = lerpTriplet(MARKETING_AMBIENT_KEYFRAMES[last.id], MARKETING_AMBIENT_KEYFRAMES.footer, t);
		layerOpacity = lerp(1, 0.42, t);
	}

	if (isInFooterScrollZone()) {
		layerOpacity = Math.min(layerOpacity, 0.42);
	}

	return { blobs, layerOpacity };
}

export function ambientStateToStyleProperties(
	state: MarketingAmbientState,
): Record<string, string> {
	const [a, b, c] = state.blobs;
	return {
		"--ambient-opacity": String(state.layerOpacity),
		"--blob-a-top": `${a.top}%`,
		"--blob-a-left": `${a.left}%`,
		"--blob-a-w": `${a.width}%`,
		"--blob-a-h": `${a.height}%`,
		"--blob-a-o": String(a.opacity),
		"--blob-b-top": `${b.top}%`,
		"--blob-b-left": `${b.left}%`,
		"--blob-b-w": `${b.width}%`,
		"--blob-b-h": `${b.height}%`,
		"--blob-b-o": String(b.opacity),
		"--blob-c-top": `${c.top}%`,
		"--blob-c-left": `${c.left}%`,
		"--blob-c-w": `${c.width}%`,
		"--blob-c-h": `${c.height}%`,
		"--blob-c-o": String(c.opacity),
	};
}
