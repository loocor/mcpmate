const extensionLanguageMap: Record<string, string> = {
	md: "markdown",
	js: "javascript",
	mjs: "javascript",
	cjs: "javascript",
	py: "python",
	json: "json",
	yaml: "yaml",
	yml: "yaml",
	toml: "plaintext",
};

/** Prism on the full document is only used at or below this size. */
export const LARGE_MATERIAL_PREVIEW_CHARS = 48_000;

export function materialPreviewLanguage(extension: string): string {
	return extensionLanguageMap[extension.toLowerCase()] ?? "plaintext";
}

export function shouldHighlightMaterialSource(contentLength: number): boolean {
	return contentLength <= LARGE_MATERIAL_PREVIEW_CHARS;
}
