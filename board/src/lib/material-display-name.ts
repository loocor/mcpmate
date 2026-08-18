const KNOWN_MATERIAL_EXTENSIONS = new Set([
	"md",
	"js",
	"mjs",
	"cjs",
	"py",
	"pdf",
	"json",
	"yaml",
	"yml",
	"toml",
	"docx",
	"xlsx",
]);

function titleCaseWord(word: string): string {
	const characters = [...word];
	if (characters.length === 0) {
		return "";
	}
	const [first, ...rest] = characters;
	return `${first.toLocaleUpperCase()}${rest.join("").toLocaleLowerCase()}`;
}

/** Strip a known material extension from a filename leaf. */
export function materialFilenameStem(filename: string): string {
	const leaf = filename.trim().split(/[/\\]/).pop() ?? filename.trim();
	const separator = leaf.lastIndexOf(".");
	if (separator <= 0) {
		return leaf;
	}
	const extension = leaf.slice(separator + 1).toLowerCase();
	if (!KNOWN_MATERIAL_EXTENSIONS.has(extension)) {
		return leaf;
	}
	return leaf.slice(0, separator);
}

/**
 * Build a human-friendly Material title from a raw filename or stem.
 * Drops known extensions, splits on connectors, and applies Title Case.
 * Backend stores the title as sent; this is a Board-only UX helper.
 */
export function humanizeMaterialTitle(raw: string): string {
	const source = raw.trim();
	if (!source) {
		return "";
	}
	const withoutExtension = materialFilenameStem(source);
	const words = withoutExtension
		.split(/[\s_.-]+/u)
		.filter(Boolean)
		.map(titleCaseWord);
	return words.join(" ");
}

export function humanizeMaterialTitleFromFilename(filename: string): string {
	return humanizeMaterialTitle(filename);
}

/** Prefer an explicit title; otherwise derive one from the upload filename. */
export function resolveMaterialUploadTitle(
	title: string,
	filename: string,
): string {
	const explicit = title.trim();
	if (explicit) {
		return explicit;
	}
	const humanized = humanizeMaterialTitleFromFilename(filename);
	return humanized || filename.trim();
}
