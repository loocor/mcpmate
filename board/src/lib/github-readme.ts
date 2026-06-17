import type { Root } from "hast";

const GITHUB_RAW_BASE = import.meta.env.DEV
	? "/github-raw"
	: "https://raw.githubusercontent.com";
const GITHUB_RAW_ASSET_BASE = "https://raw.githubusercontent.com";
const GITHUB_API_BASE = import.meta.env.DEV ? "/github-api" : "https://api.github.com";

const DEFAULT_BRANCHES = ["main", "master"] as const;

export interface GitHubReadmeAssetContext {
	owner: string;
	repo: string;
	branch: string;
	subfolder?: string | null;
}

export interface GitHubReadmeResult {
	markdown: string;
	assetContext: GitHubReadmeAssetContext;
}

export function parseGitHubRepositoryUrl(
	repositoryUrl: string,
): { owner: string; repo: string } | null {
	const matched = repositoryUrl.match(/^https?:\/\/github\.com\/([^/]+)\/([^/#?]+)/i);
	if (!matched) return null;
	const owner = matched[1];
	const repo = matched[2].replace(/\.git$/i, "");
	if (!owner || !repo) return null;
	return { owner, repo };
}

function normalizeSubfolder(subfolder?: string | null): string {
	return (subfolder ?? "").trim().replace(/^\/+|\/+$/g, "");
}

function buildReadmePath(subfolder?: string | null): string {
	const normalizedSubfolder = normalizeSubfolder(subfolder);
	return normalizedSubfolder ? `${normalizedSubfolder}/README.md` : "README.md";
}

function buildAssetBranchRootUrl(owner: string, repo: string, branch: string): string {
	return `${GITHUB_RAW_ASSET_BASE}/${owner}/${repo}/${branch}`;
}

function buildReadmeDirectoryUrl(context: GitHubReadmeAssetContext): string {
	const normalizedSubfolder = normalizeSubfolder(context.subfolder);
	const branchRoot = buildAssetBranchRootUrl(context.owner, context.repo, context.branch);
	return normalizedSubfolder ? `${branchRoot}/${normalizedSubfolder}` : branchRoot;
}

function buildRawReadmeUrl(owner: string, repo: string, branch: string, readmePath: string): string {
	const encodedPath = readmePath
		.split("/")
		.map((segment) => encodeURIComponent(segment))
		.join("/");
	return `${GITHUB_RAW_BASE}/${owner}/${repo}/${branch}/${encodedPath}`;
}

function decodeBase64Utf8(content: string): string {
	const normalized = content.replace(/\n/g, "");
	const binary = atob(normalized);
	const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
	return new TextDecoder().decode(bytes);
}

async function fetchRawReadme(url: string): Promise<string | null> {
	const response = await fetch(url);
	if (!response.ok) {
		return null;
	}
	const text = await response.text();
	return text.trim() ? text : null;
}

async function fetchReadmeViaContentsApi(
	owner: string,
	repo: string,
	readmePath: string,
): Promise<string | null> {
	const encodedPath = readmePath
		.split("/")
		.map((segment) => encodeURIComponent(segment))
		.join("/");
	const endpoint = `${GITHUB_API_BASE}/repos/${owner}/${repo}/contents/${encodedPath}`;
	const response = await fetch(endpoint, {
		headers: {
			Accept: "application/vnd.github+json",
		},
	});

	if (!response.ok) {
		return null;
	}

	const payload = (await response.json()) as { content?: string; encoding?: string };
	if (!payload.content || payload.encoding !== "base64") {
		return null;
	}

	const decoded = decodeBase64Utf8(payload.content).trim();
	return decoded || null;
}

export function resolveGitHubReadmeAssetUrl(
	src: string | undefined,
	context: GitHubReadmeAssetContext,
): string {
	const trimmed = src?.trim();
	if (!trimmed) {
		return "";
	}
	if (/^(?:https?:|data:|mailto:|#)/i.test(trimmed)) {
		return trimmed;
	}
	if (trimmed.startsWith("//")) {
		return `https:${trimmed}`;
	}

	const branchRoot = buildAssetBranchRootUrl(context.owner, context.repo, context.branch);
	if (trimmed.startsWith("/")) {
		return `${branchRoot}${trimmed}`;
	}

	try {
		const readmeDirectory = buildReadmeDirectoryUrl(context);
		const base = readmeDirectory.endsWith("/") ? readmeDirectory : `${readmeDirectory}/`;
		return new URL(trimmed, base).toString();
	} catch {
		return trimmed;
	}
}

export function rewriteReadmeAssetUrls(
	markdown: string,
	context: GitHubReadmeAssetContext,
): string {
	const withHtmlImages = markdown.replace(
		/(<img\b[^>]*\bsrc=)(["']?)([^"'\s>]+)\2/gi,
		(_match, prefix: string, quote: string, src: string) => {
			const q = quote || '"';
			return `${prefix}${q}${resolveGitHubReadmeAssetUrl(src, context)}${q}`;
		},
	);

	return withHtmlImages.replace(
		/!\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g,
		(_match, alt: string, src: string) =>
			`![${alt}](${resolveGitHubReadmeAssetUrl(src, context)})`,
	);
}

export function buildReadmeAssetContext(
	repositoryUrl: string,
	subfolder?: string | null,
	branch = DEFAULT_BRANCHES[0],
): GitHubReadmeAssetContext | null {
	const parsed = parseGitHubRepositoryUrl(repositoryUrl);
	if (!parsed) {
		return null;
	}
	return {
		owner: parsed.owner,
		repo: parsed.repo,
		branch,
		subfolder,
	};
}

export function rehypeGitHubReadmeAssets(context: GitHubReadmeAssetContext) {
	return (tree: Root) => {
		const walk = (node: unknown): void => {
			if (!node || typeof node !== "object") {
				return;
			}

			const current = node as {
				type?: string;
				tagName?: string;
				properties?: { src?: unknown };
				children?: unknown[];
			};

			if (
				current.type === "element"
				&& current.tagName === "img"
				&& current.properties?.src != null
			) {
				current.properties.src = resolveGitHubReadmeAssetUrl(
					String(current.properties.src),
					context,
				);
			}

			if (!Array.isArray(current.children)) {
				return;
			}

			for (const child of current.children) {
				walk(child);
			}
		};

		walk(tree);
	};
}

export async function fetchRepositoryReadmeMarkdown(
	repositoryUrl: string,
	subfolder?: string | null,
): Promise<GitHubReadmeResult> {
	const parsed = parseGitHubRepositoryUrl(repositoryUrl);
	if (!parsed) {
		throw new Error("unsupported-repository");
	}

	const readmePath = buildReadmePath(subfolder);

	// Race all candidate branches in parallel; first successful result wins.
	const branchResults = await Promise.allSettled(
		DEFAULT_BRANCHES.map(async (branch) => {
			const rawUrl = buildRawReadmeUrl(parsed.owner, parsed.repo, branch, readmePath);
			const content = await fetchRawReadme(rawUrl);
			if (!content) throw new Error("empty");
			return { markdown: content, branch } as const;
		}),
	);

	for (let i = 0; i < branchResults.length; i++) {
		const result = branchResults[i];
		if (result.status === "fulfilled") {
			return {
				markdown: result.value.markdown,
				assetContext: {
					owner: parsed.owner,
					repo: parsed.repo,
					branch: result.value.branch,
					subfolder,
				},
			};
		}
	}

	const apiContent = await fetchReadmeViaContentsApi(parsed.owner, parsed.repo, readmePath);
	if (apiContent) {
		return {
			markdown: apiContent,
			assetContext: {
				owner: parsed.owner,
				repo: parsed.repo,
				branch: DEFAULT_BRANCHES[0],
				subfolder,
			},
		};
	}

	throw new Error("readme-fetch-failed");
}
