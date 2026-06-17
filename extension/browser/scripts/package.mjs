import { execFileSync } from "node:child_process";
import {
	cp,
	mkdir,
	readdir,
	rm,
	writeFile,
} from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { writeBuildInfo } from "./write-build-info.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const rootDir = resolve(scriptDir, "..");
const distDir = join(rootDir, "dist");

const EXCLUDE = new Set([
	".DS_Store",
	".gitignore",
	"dist",
	"scripts",
	"mock",
	"README.md",
	"package.json",
]);

function isTestFile(name) {
	return name.endsWith(".test.mjs") || name.endsWith(".test.js");
}

async function collectFiles(dir, base = dir) {
	const entries = await readdir(dir, { withFileTypes: true });
	const files = [];
	for (const entry of entries) {
		if (EXCLUDE.has(entry.name) || isTestFile(entry.name)) continue;
		const full = join(dir, entry.name);
		const rel = full.slice(base.length + 1);
		if (entry.isDirectory()) {
			files.push(...(await collectFiles(full, base)));
		} else {
			files.push(rel);
		}
	}
	return files;
}

async function copyToStage(files, stageDir) {
	await mkdir(stageDir, { recursive: true });
	for (const rel of files) {
		const dest = join(stageDir, rel);
		await mkdir(dirname(dest), { recursive: true });
		await cp(join(rootDir, rel), dest);
	}
}

function makeZip(stageDir, outPath) {
	// ditto --keepParent includes the stage dir itself; use cd + zip for flat layout
	execFileSync("ditto", ["-c", "-k", "--sequesterRsrc", stageDir + "/", outPath], {
		stdio: "inherit",
		cwd: stageDir,
	});
}

async function main() {
	await mkdir(distDir, { recursive: true });

	// 1. Refresh build-info.js
	const { buildDate } = await writeBuildInfo();
	console.log(`build-info.js → ${buildDate}`);

	// 2. Collect publishable files
	const files = await collectFiles(rootDir);
	console.log(`Files to package: ${files.length}`);

	// 3. Stage
	const stageDir = join(distDir, ".stage");
	await rm(stageDir, { recursive: true, force: true });
	await copyToStage(files, stageDir);

	// 4. Read version from manifest
	const manifest = JSON.parse(
		await new Response(
			(await import("node:fs")).createReadStream(join(rootDir, "manifest.json")),
		).text(),
	);
	const version = manifest.version || "0.0.0";

	// 5. Create zip
	const zipName = `mcpmate-extension-${version}.zip`;
	const zipPath = join(distDir, zipName);
	await rm(zipPath, { force: true });
	makeZip(stageDir, zipPath);
	console.log(`Created ${zipPath}`);

	// 6. Cleanup stage
	await rm(stageDir, { recursive: true, force: true });

	// 7. Write a manifest for the dist
	await writeFile(
		join(distDir, "build-manifest.json"),
		JSON.stringify({ version, buildDate, files }, null, 2) + "\n",
	);
	console.log(`\nDone! Upload ${zipName} to Chrome Web Store / Edge Add-ons.`);
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
