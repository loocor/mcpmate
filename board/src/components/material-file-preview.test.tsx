import { describe, expect, it } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import "../lib/i18n/index";
import { MaterialFilePreview } from "./material-file-preview";
import {
	LARGE_MATERIAL_PREVIEW_CHARS,
	materialPreviewLanguage,
	shouldHighlightMaterialSource,
} from "./material-preview-language";

describe("materialPreviewLanguage", () => {
	it("maps the supported text material extensions to loaded Prism languages", () => {
		expect(materialPreviewLanguage("md")).toBe("markdown");
		expect(materialPreviewLanguage("mjs")).toBe("javascript");
		expect(materialPreviewLanguage("py")).toBe("python");
		expect(materialPreviewLanguage("yaml")).toBe("yaml");
	});

	it("keeps TOML and unsupported extensions as plain text", () => {
		expect(materialPreviewLanguage("toml")).toBe("plaintext");
		expect(materialPreviewLanguage("sh")).toBe("plaintext");
	});
});

describe("material preview highlighting", () => {
	it("highlights source only when the full document stays within the Prism budget", () => {
		expect(shouldHighlightMaterialSource(LARGE_MATERIAL_PREVIEW_CHARS)).toBe(
			true,
		);
		expect(shouldHighlightMaterialSource(LARGE_MATERIAL_PREVIEW_CHARS + 1)).toBe(
			false,
		);
	});
});

describe("MaterialFilePreview", () => {
	it("renders Markdown formatting in preview mode, including large files", () => {
		const content = `# Preview Heading\n\nA paragraph.\n\n${"word ".repeat(12_000)}`;
		expect(content.length).toBeGreaterThan(LARGE_MATERIAL_PREVIEW_CHARS);

		const html = renderToStaticMarkup(
			<MaterialFilePreview
				content={content}
				extension="md"
				markdownMode="rendered"
			/>,
		);

		expect(html).toContain("Preview Heading");
		expect(html).toContain("<h1");
		expect(html).not.toContain("rendered Markdown disabled");
		expect(html).not.toContain("plain text in preview mode");
	});

	it("keeps blank lines in highlighted source mode", () => {
		const html = renderToStaticMarkup(
			<MaterialFilePreview
				content={"alpha\n\nbeta"}
				extension="md"
				markdownMode="source"
			/>,
		);

		expect(html).toMatch(/alpha[\s\S]*\n\n[\s\S]*beta/);
	});

	it("keeps unlabeled fenced Markdown code as a block", () => {
		const html = renderToStaticMarkup(
			<MaterialFilePreview
				content={"```\nalpha\n\nbeta\n```"}
				extension="md"
				markdownMode="rendered"
			/>,
		);

		expect(html).toContain("<pre");
		expect(html).toMatch(/alpha[\s\S]*beta/);
	});

	it("keeps the full document in large source mode instead of a windowed slice", () => {
		const marker = "UNIQUE_END_MARKER_ZYX";
		const content = `start\n${"x".repeat(LARGE_MATERIAL_PREVIEW_CHARS + 8)}\n${marker}`;
		const html = renderToStaticMarkup(
			<MaterialFilePreview
				content={content}
				extension="js"
				markdownMode="source"
			/>,
		);

		expect(html).toContain("start");
		expect(html).toContain(marker);
		expect(html).toContain("<pre");
	});
});
