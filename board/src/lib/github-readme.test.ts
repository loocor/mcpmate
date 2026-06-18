import { describe, expect, it } from "bun:test";
import {
	fetchRepositoryReadmeMarkdown,
	resolveGitHubReadmeAssetUrl,
	rewriteReadmeAssetUrls,
} from "./github-readme";

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

describe("fetchRepositoryReadmeMarkdown", () => {
	it("uses the Contents API download URL branch for README asset context", async () => {
		const originalFetch = globalThis.fetch;
		const requests: string[] = [];

		globalThis.fetch = ((input: string | URL | Request) => {
			const url = String(input);
			requests.push(url);

			if (url.startsWith("https://raw.githubusercontent.com/")) {
				return Promise.resolve(new Response("Not found", { status: 404 }));
			}

			return Promise.resolve(
				new Response(
					JSON.stringify({
						content: btoa("# Server\n\n![Logo](./assets/logo.png)"),
						download_url:
							"https://raw.githubusercontent.com/frumu-ai/tandem/develop/packages/mcp/README.md",
						encoding: "base64",
					}),
					{ status: 200, headers: { "content-type": "application/json" } },
				),
			);
		}) as typeof fetch;

		try {
			const result = await fetchRepositoryReadmeMarkdown(
				"https://github.com/frumu-ai/tandem",
				"packages/mcp",
			);

			expect(result.assetContext.branch).toBe("develop");
			expect(resolveGitHubReadmeAssetUrl("./assets/logo.png", result.assetContext)).toBe(
				"https://raw.githubusercontent.com/frumu-ai/tandem/develop/packages/mcp/assets/logo.png",
			);
		} finally {
			globalThis.fetch = originalFetch;
		}

		expect(requests).toEqual([
			"https://raw.githubusercontent.com/frumu-ai/tandem/main/packages/mcp/README.md",
			"https://raw.githubusercontent.com/frumu-ai/tandem/master/packages/mcp/README.md",
			"https://api.github.com/repos/frumu-ai/tandem/contents/packages/mcp/README.md",
		]);
	});
});
