export const SITE_URL = "https://mcp.umate.ai";
export const SITE_NAME = "MCPMate";
export const SITE_LOGO_URL = `${SITE_URL}/logo.svg`;
export const SITE_PREVIEW_IMAGE_URL = `${SITE_URL}/screenshot/dashboard-light.png`;
export const SITE_PREVIEW_IMAGE_ALT =
	"MCPMate dashboard showing resource metrics and token savings side by side.";

export function buildSiteUrl(pathname = "/") {
	return new URL(pathname, SITE_URL).toString();
}
