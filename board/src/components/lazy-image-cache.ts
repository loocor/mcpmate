export interface CachedImage {
	data: string;
	timestamp: number;
}

export const CACHE_DURATION_MS = 24 * 60 * 60 * 1000;

export const imageCache = new Map<string, CachedImage>();
export const loadingStates = new Map<string, Promise<string | null>>();

export const clearExpiredCache = () => {
	const now = Date.now();
	for (const [key, value] of imageCache.entries()) {
		if (now - value.timestamp > CACHE_DURATION_MS) {
			imageCache.delete(key);
		}
	}
};

export const clearAllCache = () => {
	imageCache.clear();
	loadingStates.clear();
};
