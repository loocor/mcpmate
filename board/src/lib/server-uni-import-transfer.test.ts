import { describe, expect, test } from "bun:test";
import {
	SERVER_UNI_IMPORT_TEXT_MAX_BYTES,
	ServerUniImportTransferError,
	extractPayloadFromDataTransfer,
	formatServerUniImportTransferError,
} from "./server-uni-import-transfer";

function dataTransferWithFiles(files: unknown[]): DataTransfer {
	return {
		files: {
			length: files.length,
			...files,
			[Symbol.iterator]: function* () {
				yield* files;
			},
		},
	} as unknown as DataTransfer;
}

function dataTransferWithText(text: string): DataTransfer {
	return {
		files: { length: 0 },
		getData: (type: string) => (type === "text/plain" ? text : ""),
		types: ["text/plain"],
	} as unknown as DataTransfer;
}

function fileLike({
	name,
	size,
	text,
}: {
	name: string;
	size: number;
	text?: () => Promise<string>;
}): File {
	return {
		name,
		size,
		text: text ?? (async () => "{}"),
		arrayBuffer: async () => {
			throw new Error("bundle should not be read");
		},
	} as unknown as File;
}

describe("server uni-import transfer", () => {
	test("rejects MCPB bundles before reading them", async () => {
		await expect(
			extractPayloadFromDataTransfer(
				dataTransferWithFiles([
					fileLike({
						name: "server.mcpb",
						size: 128,
					}),
				]),
			),
		).rejects.toMatchObject({
			message: "MCPB and DXT bundle import is currently disabled.",
		});
	});

	test("rejects oversized text files before reading them", async () => {
		await expect(
			extractPayloadFromDataTransfer(
				dataTransferWithFiles([
					fileLike({
						name: "server.json",
						size: SERVER_UNI_IMPORT_TEXT_MAX_BYTES + 1,
						text: async () => {
							throw new Error("oversized file should not be read");
						},
					}),
				]),
			),
		).rejects.toThrow("Dropped file exceeds the 1 MB import limit.");
	});

	test("rejects non-ASCII dropped text by UTF-8 byte size", async () => {
		const oversizedText = "界".repeat(
			Math.floor(SERVER_UNI_IMPORT_TEXT_MAX_BYTES / 3) + 1,
		);

		await expect(
			extractPayloadFromDataTransfer(dataTransferWithText(oversizedText)),
		).rejects.toMatchObject({
			code: "textTooLarge",
			message: "Dropped text exceeds the 1 MB import limit.",
		});
	});

	test("formats transfer errors with translated messages", () => {
		const message = formatServerUniImportTransferError(
			new ServerUniImportTransferError(
				"tooManyFiles",
				"Drop up to 16 files at a time.",
				{ maxFiles: 16 },
			),
			(key, options) => `${key}:${options?.maxFiles}`,
		);

		expect(message).toBe("notifications.importRejections.tooManyFiles:16");
	});

	test("extracts multiple text files as payloads", async () => {
		await expect(
			extractPayloadFromDataTransfer(
				dataTransferWithFiles([
					fileLike({
						name: "one.json",
						size: 128,
						text: async () => "{\"name\":\"one\",\"command\":\"uvx\"}",
					}),
					fileLike({
						name: "two.json",
						size: 128,
						text: async () => "{\"name\":\"two\",\"command\":\"node\"}",
					}),
				]),
			),
		).resolves.toEqual({
			payloads: [
				{ text: "{\"name\":\"one\",\"command\":\"uvx\"}", fileName: "one.json" },
				{ text: "{\"name\":\"two\",\"command\":\"node\"}", fileName: "two.json" },
			],
		});
	});
});
