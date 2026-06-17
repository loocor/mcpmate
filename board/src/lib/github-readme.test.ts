import { describe, expect, it } from "bun:test";
import { resolveGitHubReadmeAssetUrl, rewriteReadmeAssetUrls } from "./github-readme";

const context = {
	owner: "frumu-ai",
	repo: "tandem",
	branch: "main",
	subfolder: null,
};

describe("resolveGitHubReadmeAssetUrl", () => {
	it("resolves relative image paths against the readme directory", () => {
		expect(resolveGitHubReadmeAssetUrl(".github/assets/logo.png", context)).toBe(
			"https://raw.githubusercontent.com/frumu-ai/tandem/main/.github/assets/logo.png",
		);
	});

	it("resolves repo-root absolute paths", () => {
		expect(resolveGitHubReadmeAssetUrl("/docs/logo.png", context)).toBe(
			"https://raw.githubusercontent.com/frumu-ai/tandem/main/docs/logo.png",
		);
	});

	it("keeps absolute remote urls unchanged", () => {
		expect(resolveGitHubReadmeAssetUrl("https://example.com/logo.png", context)).toBe(
			"https://example.com/logo.png",
		);
	});

	it("resolves paths relative to a repository subfolder", () => {
		expect(
			resolveGitHubReadmeAssetUrl("../assets/logo.png", {
				...context,
				subfolder: "packages/mcp",
			}),
		).toBe("https://raw.githubusercontent.com/frumu-ai/tandem/main/packages/assets/logo.png");
	});
});

describe("rewriteReadmeAssetUrls", () => {
	it("rewrites relative html img src attributes", () => {
		const input = '<img src=".github/assets/logo.png" alt="Tandem Logo" width="500">';
		const output = rewriteReadmeAssetUrls(input, context);
		expect(output).toContain(
			'src="https://raw.githubusercontent.com/frumu-ai/tandem/main/.github/assets/logo.png"',
		);
	});

	it("rewrites markdown image syntax", () => {
		const input = "![Logo](./assets/logo.png)";
		const output = rewriteReadmeAssetUrls(input, context);
		expect(output).toBe(
			"![Logo](https://raw.githubusercontent.com/frumu-ai/tandem/main/assets/logo.png)",
		);
	});

	it("rewrites unquoted html img src attributes", () => {
		const input = '<img src=.github/assets/logo.png alt="Logo">';
		const output = rewriteReadmeAssetUrls(input, context);
		expect(output).toContain(
			'src="https://raw.githubusercontent.com/frumu-ai/tandem/main/.github/assets/logo.png"',
		);
	});
});
