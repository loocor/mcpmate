import { describe, expect, test } from "bun:test";
import {
	SERVER_UNI_IMPORT_TEXT_MAX_BYTES,
	extractPayloadFromDataTransfer,
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
