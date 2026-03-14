export type ThemeMode = 'dark' | 'light' | 'system';
export type Theme = 'dark' | 'light';

export function getResolvedTheme(mode: ThemeMode): Theme {
	if (mode === 'system') {
		if (typeof window !== 'undefined') {
			return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
		}
		return 'light';
	}
	return mode;
}

export function applyThemeToDocument(resolvedTheme: Theme) {
	const root = window.document.documentElement;
	
	root.classList.remove('light', 'dark');
	root.classList.add(resolvedTheme);
	
	const metaThemeColor = document.querySelector('meta[name="theme-color"]');
	if (metaThemeColor) {
		metaThemeColor.setAttribute(
			'content',
			resolvedTheme === 'dark' ? '#0f172a' : '#ffffff'
		);
	}
}
