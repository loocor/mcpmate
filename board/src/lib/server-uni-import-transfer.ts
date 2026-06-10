import type { ServerIngestPayload } from "./install-normalizer";

export type ServerUniImportTransferPayload = ServerIngestPayload;

export const SERVER_UNI_IMPORT_TEXT_MAX_BYTES = 1_048_576;
const SERVER_UNI_IMPORT_MAX_FILES = 16;

function utf8ByteLength(value: string): number {
	return new TextEncoder().encode(value).byteLength;
}

function isBundleFileName(fileName: string): boolean {
	const lower = fileName.toLowerCase();
	return lower.endsWith(".mcpb") || lower.endsWith(".dxt");
}

function assertSupportedFile(file: File): void {
	if (isBundleFileName(file.name)) {
		throw new Error("MCPB and DXT bundle import is currently disabled.");
	}
	if (file.size > SERVER_UNI_IMPORT_TEXT_MAX_BYTES) {
		throw new Error("Dropped file exceeds the 1 MB import limit.");
	}
}

function assertTextWithinLimit(text: string): void {
	if (text.length <= SERVER_UNI_IMPORT_TEXT_MAX_BYTES) return;
	if (utf8ByteLength(text) > SERVER_UNI_IMPORT_TEXT_MAX_BYTES) {
		throw new Error("Dropped text exceeds the 1 MB import limit.");
	}
}

export function canIngestFromDataTransfer(dataTransfer: DataTransfer | null): boolean {
	if (!dataTransfer) return false;
	const types = Array.from(dataTransfer.types ?? []);
	return (
		types.includes("Files") ||
		types.includes("text/plain") ||
		types.includes("text/uri-list")
	);
}

export async function extractPayloadFromDataTransfer(
	dataTransfer: DataTransfer,
): Promise<ServerUniImportTransferPayload | null> {
	if (dataTransfer.files && dataTransfer.files.length > 0) {
		if (dataTransfer.files.length > SERVER_UNI_IMPORT_MAX_FILES) {
			throw new Error("Drop up to 16 files at a time.");
		}
		const payloads = await Promise.all(
			Array.from(dataTransfer.files).map(async (file) => {
				assertSupportedFile(file);
				return { text: await file.text(), fileName: file.name };
			}),
		);
		if (payloads.length === 1) {
			return payloads[0];
		}
		return { payloads };
	}

	const plainText = dataTransfer.getData("text/plain");
	if (plainText) {
		assertTextWithinLimit(plainText);
		return { text: plainText };
	}

	const uriList = dataTransfer.getData("text/uri-list");
	if (uriList) {
		assertTextWithinLimit(uriList);
		return { text: uriList };
	}

	if (dataTransfer.items && dataTransfer.items.length > 0) {
		for (const item of Array.from(dataTransfer.items)) {
			if (item.kind === "string") {
				const value = await new Promise<string | null>((resolve) => {
					item.getAsString((text) => resolve(text ?? null));
				});
				if (value) {
					assertTextWithinLimit(value);
					return { text: value };
				}
			}
		}
	}

	return null;
}
