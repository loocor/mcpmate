/** MCPMate community and feedback on GitHub Discussions. */
export const MCPMATE_GITHUB_DISCUSSIONS_HREF =
	"https://github.com/loocor/mcpmate/discussions" as const;

/** Opens the MCPMate GitHub Discussions page in a new tab. */
export function openMcpmateGithubDiscussions(): void {
	if (typeof window === "undefined") {
		return;
	}
	window.open(
		MCPMATE_GITHUB_DISCUSSIONS_HREF,
		"_blank",
		"noopener,noreferrer",
	);
}
