import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const defaultOutputFile = resolve(scriptDir, "..", "build-info.js");

function datePart(parts, type) {
	return parts.find((part) => part.type === type)?.value;
}

export function formatBuildDate(date = new Date(), timeZone = "Asia/Singapore") {
	const parts = new Intl.DateTimeFormat("en-CA", {
		day: "2-digit",
		hour: "2-digit",
		hour12: false,
		minute: "2-digit",
		month: "2-digit",
		second: "2-digit",
		timeZone,
		year: "numeric",
	}).formatToParts(date);

	return `${datePart(parts, "year")}${datePart(parts, "month")}${datePart(
		parts,
		"day",
	)}${datePart(parts, "hour")}${datePart(parts, "minute")}${datePart(parts, "second")}`;
}

export function buildInfoSource(buildDate) {
	return `globalThis.MCPMATE_EXTENSION_BUILD = Object.freeze({\n\tbuildDate: ${JSON.stringify(
		buildDate,
	)},\n});\n`;
}

export async function writeBuildInfo(outputFile = defaultOutputFile) {
	const buildDate = formatBuildDate();
	await mkdir(dirname(outputFile), { recursive: true });
	await writeFile(outputFile, buildInfoSource(buildDate));
	return { buildDate, outputFile };
}

if (import.meta.main) {
	const outputFile = process.argv[2] ? resolve(process.argv[2]) : defaultOutputFile;
	const result = await writeBuildInfo(outputFile);
	console.log(`Wrote ${result.outputFile}`);
	console.log(`Build Date: ${result.buildDate}`);
}
