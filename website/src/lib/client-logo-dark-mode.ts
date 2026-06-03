/**
 * Logos known to be near-black silhouettes (used when canvas sampling is blocked).
 * Prefer updating this list over loosening auto-detection heuristics.
 */
const CLIENT_LOGO_INVERT_ON_DARK_FALLBACK = new Set(["goose", "hermes", "zed"]);

/** Assets that must never use brightness-0 + invert (e.g. light mark on baked dark tile). */
const CLIENT_LOGO_NEVER_DARK_INVERT = new Set(["zencoder"]);

const SAMPLE_SIZE = 32;

/** Flat black silhouettes stay near one luminance; shaded dark marks must not be flattened by brightness-0. */
const DARK_SILHOUETTE_LUMINANCE_STD_MAX = 0.05;

function normalizeLogoIdentifier(identifier: string): string {
	return identifier.trim().toLowerCase().replace(/[^a-z0-9]+/g, "_");
}

function opaquePixelLuminance(data: Uint8ClampedArray, index: number): number {
	const r = data[index] / 255;
	const g = data[index + 1] / 255;
	const b = data[index + 2] / 255;
	return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** HSV-style saturation in [0, 1]. */
function opaquePixelSaturation(data: Uint8ClampedArray, index: number): number {
	const r = data[index] / 255;
	const g = data[index + 1] / 255;
	const b = data[index + 2] / 255;
	const max = Math.max(r, g, b);
	const min = Math.min(r, g, b);
	if (max <= 0.01) {
		return 0;
	}
	return (max - min) / max;
}

/** Baked square/rounded tile backgrounds (opaque corners) are already tuned for dark UI. */
function hasOpaqueTileBackground(data: Uint8ClampedArray, width: number, height: number): boolean {
	const cornerCoords = [
		[0, 0],
		[width - 1, 0],
		[0, height - 1],
		[width - 1, height - 1],
	];
	let opaqueCorners = 0;

	for (const [x, y] of cornerCoords) {
		const index = (y * width + x) * 4;
		if (data[index + 3] >= 200) {
			opaqueCorners += 1;
		}
	}

	return opaqueCorners >= 3;
}

export function logoMustNeverDarkInvert(identifier: string): boolean {
	return CLIENT_LOGO_NEVER_DARK_INVERT.has(normalizeLogoIdentifier(identifier));
}

/**
 * Only treat as invert-candidate when the mark is a flat dark, low-saturation silhouette
 * on transparency — not colorful icons, light tiles, baked backgrounds, or shaded dark marks.
 */
export function analyzeLogoNeedsDarkInvert(src: string): Promise<boolean | null> {
	if (typeof document === "undefined") {
		return Promise.resolve(null);
	}

	return new Promise((resolve) => {
		const image = new Image();
		// data: URLs do not need (and can mis-handle) crossOrigin for canvas sampling.
		if (!src.startsWith("data:")) {
			image.crossOrigin = "anonymous";
		}
		image.decoding = "async";

		image.onload = () => {
			try {
				const canvas = document.createElement("canvas");
				canvas.width = SAMPLE_SIZE;
				canvas.height = SAMPLE_SIZE;
				const context = canvas.getContext("2d", { willReadFrequently: true });
				if (!context) {
					resolve(null);
					return;
				}

				context.clearRect(0, 0, SAMPLE_SIZE, SAMPLE_SIZE);
				context.drawImage(image, 0, 0, SAMPLE_SIZE, SAMPLE_SIZE);
				const { data } = context.getImageData(0, 0, SAMPLE_SIZE, SAMPLE_SIZE);

				if (hasOpaqueTileBackground(data, SAMPLE_SIZE, SAMPLE_SIZE)) {
					resolve(false);
					return;
				}

				let opaqueCount = 0;
				let luminanceSum = 0;
				let luminanceSqSum = 0;
				let darkMonoCount = 0;
				let brightCount = 0;
				let colorfulCount = 0;

				for (let index = 0; index < data.length; index += 4) {
					const alpha = data[index + 3];
					if (alpha < 24) {
						continue;
					}

					const luminance = opaquePixelLuminance(data, index);
					const saturation = opaquePixelSaturation(data, index);
					opaqueCount += 1;
					luminanceSum += luminance;
					luminanceSqSum += luminance * luminance;

					if (luminance < 0.14 && saturation < 0.14) {
						darkMonoCount += 1;
					}
					if (luminance > 0.72) {
						brightCount += 1;
					}
					if (saturation > 0.22) {
						colorfulCount += 1;
					}
				}

				if (opaqueCount < 16) {
					resolve(null);
					return;
				}

				const meanLuminance = luminanceSum / opaqueCount;
				const luminanceVariance = Math.max(
					0,
					luminanceSqSum / opaqueCount - meanLuminance * meanLuminance,
				);
				const luminanceStd = Math.sqrt(luminanceVariance);
				const darkMonoRatio = darkMonoCount / opaqueCount;
				const brightRatio = brightCount / opaqueCount;
				const colorfulRatio = colorfulCount / opaqueCount;

				const isDarkSilhouette =
					darkMonoRatio >= 0.4 &&
					brightRatio <= 0.08 &&
					colorfulRatio <= 0.12 &&
					meanLuminance < 0.24 &&
					luminanceStd < DARK_SILHOUETTE_LUMINANCE_STD_MAX;

				resolve(isDarkSilhouette);
			} catch {
				resolve(null);
			}
		};

		image.onerror = () => resolve(null);
		image.src = src;
	});
}

export function logoNeedsDarkInvertFallback(identifier: string): boolean {
	if (logoMustNeverDarkInvert(identifier)) {
		return false;
	}
	return CLIENT_LOGO_INVERT_ON_DARK_FALLBACK.has(normalizeLogoIdentifier(identifier));
}

/** Dark silhouette → light mark in dark UI while preserving antialiasing and internal detail. */
export const CLIENT_LOGO_DARK_INVERT_CLASS = "dark:brightness-125 dark:contrast-90 dark:invert dark:saturate-0";
