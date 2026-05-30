export type ServerUniImportTransferPayload = {
	text?: string;
	buffer?: ArrayBuffer;
	fileName?: string;
};

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
		const file = dataTransfer.files[0];
		if (file.name.endsWith(".mcpb") || file.name.endsWith(".dxt")) {
			return { buffer: await file.arrayBuffer(), fileName: file.name };
		}
		return { text: await file.text(), fileName: file.name };
	}

	const plainText = dataTransfer.getData("text/plain");
	if (plainText) {
		return { text: plainText };
	}

	const uriList = dataTransfer.getData("text/uri-list");
	if (uriList) {
		return { text: uriList };
	}

	if (dataTransfer.items && dataTransfer.items.length > 0) {
		for (const item of Array.from(dataTransfer.items)) {
			if (item.kind === "string") {
				const value = await new Promise<string | null>((resolve) => {
					item.getAsString((text) => resolve(text ?? null));
				});
				if (value) {
					return { text: value };
				}
			}
		}
	}

	return null;
}
