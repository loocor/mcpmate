import type { InspectorCapabilityKind } from "./inspector-capability";

export type InspectorPreviewBlock =
	| {
			kind: "text";
			text: string;
			format: "plain" | "markdown";
	  }
	| {
			kind: "image";
			src: string;
			mimeType?: string;
			alt?: string;
	  };

function isRecord(value: unknown): value is Record<string, unknown> {
	return value != null && typeof value === "object" && !Array.isArray(value);
}

function stringValue(value: unknown): string | null {
	return typeof value === "string" && value.length > 0 ? value : null;
}

function asArray(value: unknown): unknown[] {
	return Array.isArray(value) ? value : [];
}

function isMarkdownContent(entry: Record<string, unknown>, text: string): boolean {
	const mimeType = stringValue(entry.mimeType) ?? stringValue(entry.mime_type);
	if (mimeType?.toLowerCase().includes("markdown")) {
		return true;
	}
	return text.trimStart().startsWith("#") || text.includes("\n```");
}

function imageSource(entry: Record<string, unknown>): string | null {
	const uri = stringValue(entry.uri) ?? stringValue(entry.url);
	if (uri?.startsWith("data:") || uri?.startsWith("http://") || uri?.startsWith("https://")) {
		return uri;
	}

	const data = stringValue(entry.data) ?? stringValue(entry.blob);
	if (!data) {
		return null;
	}
	const mimeType = stringValue(entry.mimeType) ?? stringValue(entry.mime_type) ?? "image/png";
	return data.startsWith("data:") ? data : `data:${mimeType};base64,${data}`;
}

function pushContentBlock(blocks: InspectorPreviewBlock[], entry: unknown): void {
	if (!isRecord(entry)) {
		return;
	}

	const type = stringValue(entry.type)?.toLowerCase();
	if (type === "image") {
		const src = imageSource(entry);
		if (src) {
			blocks.push({
				kind: "image",
				src,
				mimeType: stringValue(entry.mimeType) ?? stringValue(entry.mime_type) ?? undefined,
				alt: stringValue(entry.alt) ?? undefined,
			});
		}
		return;
	}

	const text =
		stringValue(entry.text) ??
		stringValue(entry.content) ??
		stringValue(entry.value) ??
		null;
	if (text) {
		blocks.push({
			kind: "text",
			text,
			format: isMarkdownContent(entry, text) ? "markdown" : "plain",
		});
	}
}

function collectPromptMessageBlocks(blocks: InspectorPreviewBlock[], messages: unknown): void {
	for (const message of asArray(messages)) {
		if (!isRecord(message)) {
			continue;
		}
		const content = message.content;
		if (Array.isArray(content)) {
			for (const entry of content) {
				pushContentBlock(blocks, entry);
			}
		} else {
			pushContentBlock(blocks, content);
		}
	}
}

function collectResourceBlocks(blocks: InspectorPreviewBlock[], contents: unknown): void {
	for (const entry of asArray(contents)) {
		pushContentBlock(blocks, entry);
	}
}

export function parseInspectorResponsePreview(
	response: unknown,
	_kind: InspectorCapabilityKind,
): InspectorPreviewBlock[] {
	if (!isRecord(response)) {
		return [];
	}

	const blocks: InspectorPreviewBlock[] = [];

	for (const entry of asArray(response.content)) {
		pushContentBlock(blocks, entry);
	}
	collectPromptMessageBlocks(blocks, response.messages);
	collectResourceBlocks(blocks, response.contents);

	if (blocks.length === 0) {
		pushContentBlock(blocks, response);
	}

	return blocks;
}

export function extractInspectorResponseText(
	response: unknown,
	kind: InspectorCapabilityKind,
): string | null {
	const textBlocks = parseInspectorResponsePreview(response, kind)
		.filter((block): block is Extract<InspectorPreviewBlock, { kind: "text" }> => block.kind === "text")
		.map((block) => block.text);
	return textBlocks.length ? textBlocks.join("\n\n") : null;
}

export function hasInspectorResponsePreview(
	response: unknown,
	kind: InspectorCapabilityKind,
): boolean {
	return parseInspectorResponsePreview(response, kind).length > 0;
}
