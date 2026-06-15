import type { ServerIngestPayload } from "./install-normalizer";

export type ServerUniImportTransferPayload = ServerIngestPayload;

export const SERVER_UNI_IMPORT_TEXT_MAX_BYTES = 1_048_576;
export const SERVER_UNI_IMPORT_TRANSFER_TYPES = [
	"Files",
	"text/plain",
	"text/uri-list",
] as const;
const SERVER_UNI_IMPORT_MAX_FILES = 16;

export type ServerUniImportTransferErrorCode =
	| "bundleDisabled"
	| "fileTooLarge"
	| "textTooLarge"
	| "tooManyFiles";

type ServerUniImportTransferErrorParams = {
	maxFiles?: number;
	maxMb?: number;
};

type TranslationFunction = (
	key: string,
	options?: Record<string, unknown>,
) => string;

export class ServerUniImportTransferError extends Error {
	readonly code: ServerUniImportTransferErrorCode;
	readonly params: ServerUniImportTransferErrorParams;

	constructor(
		code: ServerUniImportTransferErrorCode,
		message: string,
		params: ServerUniImportTransferErrorParams = {},
	) {
		super(message);
		this.name = "ServerUniImportTransferError";
		this.code = code;
		this.params = params;
	}
}

export function formatServerUniImportTransferError(
	error: unknown,
	t: TranslationFunction,
	keyPrefix = "notifications.importRejections",
): string {
	if (error instanceof ServerUniImportTransferError) {
		return t(`${keyPrefix}.${error.code}`, {
			defaultValue: error.message,
			...error.params,
		});
	}
	return error instanceof Error ? error.message : String(error);
}

function utf8ByteLength(value: string): number {
	return new TextEncoder().encode(value).byteLength;
}

function hasNonAscii(value: string): boolean {
	for (let index = 0; index < value.length; index += 1) {
		if (value.charCodeAt(index) > 0x7f) {
			return true;
		}
	}
	return false;
}

function isBundleFileName(fileName: string): boolean {
	const lower = fileName.toLowerCase();
	return lower.endsWith(".mcpb") || lower.endsWith(".dxt");
}

function assertSupportedFile(file: File): void {
	if (isBundleFileName(file.name)) {
		throw new ServerUniImportTransferError(
			"bundleDisabled",
			"MCPB and DXT bundle import is currently disabled.",
		);
	}
	if (file.size > SERVER_UNI_IMPORT_TEXT_MAX_BYTES) {
		throw new ServerUniImportTransferError(
			"fileTooLarge",
			"Dropped file exceeds the 1 MB import limit.",
			{ maxMb: 1 },
		);
	}
}

function assertTextWithinLimit(text: string): void {
	if (text.length > SERVER_UNI_IMPORT_TEXT_MAX_BYTES) {
		throw new ServerUniImportTransferError(
			"textTooLarge",
			"Dropped text exceeds the 1 MB import limit.",
			{ maxMb: 1 },
		);
	}
	if (
		hasNonAscii(text) &&
		utf8ByteLength(text) > SERVER_UNI_IMPORT_TEXT_MAX_BYTES
	) {
		throw new ServerUniImportTransferError(
			"textTooLarge",
			"Dropped text exceeds the 1 MB import limit.",
			{ maxMb: 1 },
		);
	}
}

export function hasDataTransferType(
	types: DataTransfer["types"] | null | undefined,
	type: string,
): boolean {
	if (!types) return false;
	return "contains" in types
		? (types as unknown as DOMStringList).contains(type)
		: (types as readonly string[]).includes(type);
}

export function canIngestFromDataTransfer(dataTransfer: DataTransfer | null): boolean {
	return SERVER_UNI_IMPORT_TRANSFER_TYPES.some((type) =>
		hasDataTransferType(dataTransfer?.types, type),
	);
}

export async function extractPayloadFromDataTransfer(
	dataTransfer: DataTransfer,
): Promise<ServerUniImportTransferPayload | null> {
	if (dataTransfer.files && dataTransfer.files.length > 0) {
		if (dataTransfer.files.length > SERVER_UNI_IMPORT_MAX_FILES) {
			throw new ServerUniImportTransferError(
				"tooManyFiles",
				"Drop up to 16 files at a time.",
				{ maxFiles: SERVER_UNI_IMPORT_MAX_FILES },
			);
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
