import { describe, expect, it } from "bun:test";
import {
	humanizeMaterialTitle,
	humanizeMaterialTitleFromFilename,
	materialFilenameStem,
	resolveMaterialUploadTitle,
} from "./material-display-name";

describe("material display title helpers", () => {
	it("strips known extensions from filename stems", () => {
		expect(materialFilenameStem("research_brief-final.MD")).toBe(
			"research_brief-final",
		);
		expect(materialFilenameStem("notes.tar.md")).toBe("notes.tar");
		expect(materialFilenameStem("readme")).toBe("readme");
	});

	it("humanizes connectors into Title Case words without the extension", () => {
		expect(humanizeMaterialTitleFromFilename("research_brief-final.md")).toBe(
			"Research Brief Final",
		);
		expect(humanizeMaterialTitle("api_auth-notes")).toBe("Api Auth Notes");
		expect(humanizeMaterialTitleFromFilename("客户调研_纪要.md")).toBe(
			"客户调研 纪要",
		);
		expect(humanizeMaterialTitleFromFilename("客户调研_纪要-v2.md")).toBe(
			"客户调研 纪要 V2",
		);
	});

	it("keeps an explicit title and only derives when empty", () => {
		expect(
			resolveMaterialUploadTitle("My Custom Title", "research_brief.md"),
		).toBe("My Custom Title");
		expect(resolveMaterialUploadTitle("  ", "research_brief.md")).toBe(
			"Research Brief",
		);
	});

	it("returns an empty string for blank humanize input", () => {
		expect(humanizeMaterialTitle("   ")).toBe("");
	});
});
