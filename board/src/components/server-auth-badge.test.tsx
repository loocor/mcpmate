import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

import "../lib/i18n/index";
import { ServerAuthBadge } from "./server-auth-badge";

test("shows remote authentication as off when no mode is configured", () => {
	const markup = renderToStaticMarkup(
		<ServerAuthBadge authMode={null} showOff onAction={() => undefined} />,
	);

	expect(markup).toContain("Off");
	expect(markup).toContain("rounded-full");
	expect(markup).toContain("border-red-200");
	expect(markup).not.toContain("underline");
});

test("uses the same warning badge shape for expired OAuth credentials", () => {
	const markup = renderToStaticMarkup(
		<ServerAuthBadge
			authMode="oauth"
			oauthStatus="expired"
			onAction={() => undefined}
		/>,
	);

	expect(markup).toContain("rounded-full");
	expect(markup).toContain("border-red-200");
	expect(markup).not.toContain("underline");
});

test("keeps an unspecified authentication mode hidden by default", () => {
	const markup = renderToStaticMarkup(
		<ServerAuthBadge authMode={null} />,
	);

	expect(markup).toBe("");
});
