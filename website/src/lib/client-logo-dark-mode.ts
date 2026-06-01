/**
 * Logos known to be near-black silhouettes (used when canvas sampling is blocked).
 * Prefer updating this list over loosening auto-detection heuristics.
 */
const CLIENT_LOGO_INVERT_ON_DARK_FALLBACK = new Set(["goose", "hermes", "zed"]);

const SAMPLE_SIZE = 32;

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

/**
 * Only treat as invert-candidate when the mark is a dark, low-saturation silhouette
 * on transparency — not colorful icons or marks on a light tile in the asset.
 */
export function analyzeLogoNeedsDarkInvert(src: string): Promise<boolean | null> {
	if (typeof document === "undefined") {
		return Promise.resolve(null);
	}

	return new Promise((resolve) => {
		const image = new Image();
		image.crossOrigin = "anonymous";
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

				let opaqueCount = 0;
				let luminanceSum = 0;
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
				const darkMonoRatio = darkMonoCount / opaqueCount;
				const brightRatio = brightCount / opaqueCount;
				const colorfulRatio = colorfulCount / opaqueCount;

				const isDarkSilhouette =
					darkMonoRatio >= 0.32 &&
					brightRatio <= 0.1 &&
					colorfulRatio <= 0.12 &&
					meanLuminance < 0.24;

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
	return CLIENT_LOGO_INVERT_ON_DARK_FALLBACK.has(identifier);
}

/** Black silhouette → white in dark UI. Do not apply to colorful or already-light marks. */
export const CLIENT_LOGO_DARK_INVERT_CLASS = "dark:brightness-0 dark:invert";
