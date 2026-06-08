import {
	buildBearerSecretValue,
	containsSecretPlaceholder,
	extractWholeSecretAlias,
	isRedactedMask,
} from "./secure-field";

const INLINE_SECRET_PATTERN = /\[\[secret:([^\]]+)\]\]/g;

export type InlineSecretSegment =
	| { kind: "text"; text: string }
	| { kind: "secret"; alias: string };

export interface InlineInsertTarget {
	segmentIndex: number;
	offset: number;
}

export type InlineDisplayItem =
	| { key: "prefix"; kind: "prefix" }
	| { key: "trailing"; kind: "trailing" }
	| { key: string; kind: "text"; storedIndex: number; text: string }
	| { key: string; kind: "secret"; storedIndex: number; alias: string };

export function parseInlineSecretValue(value: string): InlineSecretSegment[] {
	if (!value) {
		return [{ kind: "text", text: "" }];
	}

	const segments: InlineSecretSegment[] = [];
	let lastIndex = 0;

	for (const match of value.matchAll(INLINE_SECRET_PATTERN)) {
		const start = match.index ?? 0;
		if (start > lastIndex) {
			segments.push({ kind: "text", text: value.slice(lastIndex, start) });
		}
		segments.push({ kind: "secret", alias: match[1] });
		lastIndex = start + match[0].length;
	}

	if (lastIndex < value.length) {
		segments.push({ kind: "text", text: value.slice(lastIndex) });
	}

	return segments.length > 0 ? segments : [{ kind: "text", text: "" }];
}

export function buildInlineDisplayItems(value: string): InlineDisplayItem[] {
	const stored = parseInlineSecretValue(value);
	const items: InlineDisplayItem[] = [];

	if (stored[0]?.kind === "secret") {
		items.push({ key: "prefix", kind: "prefix" });
	}

	for (let index = 0; index < stored.length; index += 1) {
		const segment = stored[index];
		if (segment.kind === "secret") {
			items.push({
				key: `secret-${index}`,
				kind: "secret",
				storedIndex: index,
				alias: segment.alias,
			});
			continue;
		}

		items.push({
			key: `text-${index}`,
			kind: "text",
			storedIndex: index,
			text: segment.text,
		});
	}

	const last = stored[stored.length - 1];
	if (!last || last.kind === "secret") {
		items.push({ key: "trailing", kind: "trailing" });
	}

	return items;
}

export type InlineFocusTarget =
	| { mode: "inline"; inputKey: string; caretOffset: number }
	| { mode: "plain"; caretOffset: number };

export function resolveInlineFocusTargetAfterUpdate(
	value: string,
): InlineFocusTarget {
	if (!shouldUseInlineEditor(value)) {
		return { mode: "plain", caretOffset: value.length };
	}

	const items = buildInlineDisplayItems(value);
	if (items.some((item) => item.kind === "trailing")) {
		return { mode: "inline", inputKey: "trailing", caretOffset: 0 };
	}

	const textItems = items.filter(
		(item): item is Extract<InlineDisplayItem, { kind: "text" }> =>
			item.kind === "text",
	);
	const lastText = textItems[textItems.length - 1];
	if (lastText) {
		return {
			mode: "inline",
			inputKey: lastText.key,
			caretOffset: lastText.text.length,
		};
	}

	return { mode: "inline", inputKey: "trailing", caretOffset: 0 };
}

/** Caret position after backspace removed the secret before a text segment. */
export function resolveFocusAfterBackspaceAtTextBoundary(
	newValue: string,
	storedTextIndex: number,
	previousValue: string,
): InlineFocusTarget {
	if (!shouldUseInlineEditor(newValue)) {
		const stored = parseInlineSecretValue(previousValue);
		let caretOffset = 0;
		for (let index = 0; index < storedTextIndex; index += 1) {
			const segment = stored[index];
			if (segment.kind === "text") {
				caretOffset += segment.text.length;
			}
		}
		return { mode: "plain", caretOffset };
	}

	const shiftedIndex = storedTextIndex - 1;
	const segment = parseInlineSecretValue(newValue)[shiftedIndex];
	if (segment?.kind === "text") {
		return {
			mode: "inline",
			inputKey: `text-${shiftedIndex}`,
			caretOffset: 0,
		};
	}

	return resolveInlineFocusTargetAfterUpdate(newValue);
}

export function valueStartsWithSecret(value: string): boolean {
	return parseInlineSecretValue(value)[0]?.kind === "secret";
}

export function isFlexibleInlineTextSlot(
	item: InlineDisplayItem,
	items: InlineDisplayItem[],
): boolean {
	if (item.kind === "trailing") {
		return true;
	}
	if (item.kind !== "text") {
		return false;
	}
	if (items.some((entry) => entry.kind === "trailing")) {
		return false;
	}
	const textItems = items.filter(
		(entry): entry is Extract<InlineDisplayItem, { kind: "text" }> =>
			entry.kind === "text",
	);
	const lastText = textItems[textItems.length - 1];
	return lastText?.key === item.key;
}

export function serializeInlineSecretSegments(
	segments: InlineSecretSegment[],
): string {
	return segments
		.map((segment) =>
			segment.kind === "text"
				? segment.text
				: `[[secret:${segment.alias}]]`,
		)
		.join("");
}

export function shouldUseInlineEditor(value: string): boolean {
	const trimmed = value.trim();
	if (!trimmed) return false;
	if (isRedactedMask(trimmed)) return false;
	return containsSecretPlaceholder(trimmed);
}

export function resolveFocusAfterAppendInlineText(
	nextValue: string,
): InlineFocusTarget {
	const stored = parseInlineSecretValue(nextValue);
	const lastIndex = stored.length - 1;
	const last = stored[lastIndex];
	if (last?.kind === "text") {
		return {
			mode: "inline",
			inputKey: `text-${lastIndex}`,
			caretOffset: last.text.length,
		};
	}
	return resolveInlineFocusTargetAfterUpdate(nextValue);
}

export function resolveFocusAfterPrependInlineText(
	nextValue: string,
): InlineFocusTarget {
	const first = parseInlineSecretValue(nextValue)[0];
	if (first?.kind === "text") {
		return {
			mode: "inline",
			inputKey: "text-0",
			caretOffset: first.text.length,
		};
	}
	return resolveInlineFocusTargetAfterUpdate(nextValue);
}

export function prependInlineText(value: string, text: string): string {
	if (!text) return value;
	return `${text}${value}`;
}

export function appendInlineText(value: string, text: string): string {
	if (!text) return value;
	const stored = parseInlineSecretValue(value);
	const last = stored[stored.length - 1];
	if (last?.kind === "text") {
		return updateInlineSecretTextSegment(
			value,
			stored.length - 1,
			`${last.text}${text}`,
		);
	}
	return serializeInlineSecretSegments([...stored, { kind: "text", text }]);
}

export function insertInlineSecretPlaceholder(
	value: string,
	placeholder: string,
	target?: InlineInsertTarget,
): string {
	const normalized = placeholder.trim();
	const alias = extractWholeSecretAlias(normalized);
	if (!alias) {
		return target
			? insertTextAtTarget(value, normalized, target)
			: value
				? `${value}${normalized}`
				: normalized;
	}

	if (target) {
		return insertInlineSecretAtTarget(value, placeholder, target);
	}

	return appendInlineSecretPlaceholder(value, alias);
}

/** Append a secret badge to the end of an inline value. */
export function appendInlineSecretPlaceholder(
	value: string,
	alias: string,
): string {
	const segments = parseInlineSecretValue(value);
	const last = segments[segments.length - 1];

	if (segments.length === 1 && last?.kind === "text" && !last.text) {
		return `[[secret:${alias}]]`;
	}

	const next: InlineSecretSegment[] = [...segments];
	if (last?.kind === "secret") {
		next.push({ kind: "text", text: "" });
	}
	next.push({ kind: "secret", alias });
	next.push({ kind: "text", text: "" });

	return serializeInlineSecretSegments(next);
}

/** Insert a secret placeholder into a stored field value (plain or inline). */
export function insertSecretPlaceholderIntoFieldValue(
	value: string,
	placeholder: string,
	options?: {
		headerKey?: string | null;
		target?: InlineInsertTarget;
	},
): string {
	if (isRedactedMask(value.trim())) {
		return buildBearerSecretValueIfNeeded("", options?.headerKey, placeholder);
	}
	if (shouldUseInlineEditor(value)) {
		return insertInlineSecretPlaceholder(value, placeholder, options?.target);
	}
	return insertSecretIntoPlainValue(value, placeholder, options);
}

/** Insert a secret into plain text; appends when no explicit target is provided. */
export function insertSecretIntoPlainValue(
	value: string,
	placeholder: string,
	options?: {
		headerKey?: string | null;
		target?: InlineInsertTarget;
	},
): string {
	const normalized = placeholder.trim();
	const alias = extractWholeSecretAlias(normalized);
	const secretToken = alias ? `[[secret:${alias}]]` : normalized;

	if (!value.trim()) {
		return buildBearerSecretValueIfNeeded(value, options?.headerKey, placeholder);
	}

	if (!options?.target) {
		return `${value}${secretToken}`;
	}

	const offset = Math.max(0, Math.min(options.target.offset, value.length));
	return `${value.slice(0, offset)}${secretToken}${value.slice(offset)}`;
}

export function insertInlineSecretAtTarget(
	value: string,
	placeholder: string,
	target: InlineInsertTarget,
): string {
	const alias = extractWholeSecretAlias(placeholder.trim());
	if (!alias) {
		return insertTextAtTarget(value, placeholder.trim(), target);
	}

	const segments = parseInlineSecretValue(value);
	const { segmentIndex, offset } = target;

	if (segmentIndex >= segments.length) {
		if (segments.length > 0 && segments[segments.length - 1]?.kind === "secret") {
			return serializeInlineSecretSegments([
				...segments,
				{ kind: "secret", alias },
				{ kind: "text", text: "" },
			]);
		}
		return insertInlineSecretPlaceholder(value, placeholder);
	}

	const segment = segments[segmentIndex];
	if (segment.kind === "secret") {
		const next = [
			...segments.slice(0, segmentIndex + 1),
			{ kind: "secret" as const, alias },
			{ kind: "text" as const, text: "" },
			...segments.slice(segmentIndex + 1),
		];
		return serializeInlineSecretSegments(next);
	}

	const before = segment.text.slice(0, offset);
	const after = segment.text.slice(offset);
	const next: InlineSecretSegment[] = [
		...segments.slice(0, segmentIndex),
		...(before ? [{ kind: "text" as const, text: before }] : []),
		{ kind: "secret" as const, alias },
		...(after ? [{ kind: "text" as const, text: after }] : []),
		...segments.slice(segmentIndex + 1),
	];
	return serializeInlineSecretSegments(next);
}

export function insertTextAtTarget(
	value: string,
	text: string,
	target: InlineInsertTarget,
): string {
	const segments = parseInlineSecretValue(value);
	const { segmentIndex, offset } = target;

	if (segmentIndex >= segments.length) {
		return `${value}${text}`;
	}

	const segment = segments[segmentIndex];
	if (segment.kind !== "text") {
		return `${value}${text}`;
	}

	const nextText =
		segment.text.slice(0, offset) + text + segment.text.slice(offset);
	return updateInlineSecretTextSegment(value, segmentIndex, nextText);
}

export function updateInlineSecretTextSegment(
	value: string,
	segmentIndex: number,
	text: string,
): string {
	const segments = parseInlineSecretValue(value);
	if (segmentIndex >= segments.length || segmentIndex < 0) {
		return value;
	}

	const target = segments[segmentIndex];
	if (target.kind !== "text") {
		return value;
	}

	const next = segments.map((segment, index) =>
		index === segmentIndex ? { kind: "text" as const, text } : segment,
	);

	return serializeInlineSecretSegments(next);
}

export function removeInlineSecretSegment(
	value: string,
	segmentIndex: number,
): string {
	const segments = parseInlineSecretValue(value).filter(
		(_, index) => index !== segmentIndex,
	);

	if (!segments.length) {
		return "";
	}

	return serializeInlineSecretSegments(segments);
}

/** Delete the secret immediately before a text boundary (leading/trailing/mid start). */
export function backspaceSecretBeforeTextBoundary(
	value: string,
	storedTextIndex: number,
): string | null {
	const stored = parseInlineSecretValue(value);
	if (storedTextIndex <= 0) return null;
	if (stored[storedTextIndex - 1]?.kind !== "secret") return null;
	return removeInlineSecretSegment(value, storedTextIndex - 1);
}

/** Backspace at end of value: trim last text char or remove trailing secret. */
export function backspaceInlineAtEnd(value: string): string {
	const stored = parseInlineSecretValue(value);
	const last = stored[stored.length - 1];
	if (last?.kind === "text" && last.text.length > 0) {
		return updateInlineSecretTextSegment(
			value,
			stored.length - 1,
			last.text.slice(0, -1),
		);
	}
	if (last?.kind === "secret") {
		return removeInlineSecretSegment(value, stored.length - 1);
	}
	return value;
}

export function buildBearerSecretValueIfNeeded(
	value: string,
	headerKey?: string | null,
	placeholder?: string,
): string {
	if (!placeholder) return value;
	const alias = extractWholeSecretAlias(placeholder.trim());
	if (!alias) return placeholder;

	const trimmed = value.trim();
	if (trimmed) {
		return value;
	}

	const normalizedKey = headerKey?.trim().toLowerCase();
	if (
		normalizedKey === "authorization" ||
		normalizedKey === "proxy-authorization"
	) {
		return buildBearerSecretValue(placeholder);
	}

	return placeholder.trim();
}

