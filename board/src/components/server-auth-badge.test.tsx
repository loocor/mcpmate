import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import i18n from "../lib/i18n/index";
import {
	resolveElevatedServerWarningLabel,
	resolveServerAuthWarningLabel,
	ServerAuthBadge,
} from "./server-auth-badge";

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
	expect(markup).toContain("Reauthorize required");
	expect(markup).not.toContain("underline");
});

test("keeps an unspecified authentication mode hidden by default", () => {
	const markup = renderToStaticMarkup(
		<ServerAuthBadge authMode={null} />,
	);

	expect(markup).toBe("");
});

test("resolveServerAuthWarningLabel returns a blocking label for expired OAuth", () => {
	const label = resolveServerAuthWarningLabel({
		authMode: "oauth",
		oauthStatus: "expired",
		t: i18n.getFixedT("en", "servers"),
	});

	expect(label).toBe("Reauthorize required");
});

test("resolveServerAuthWarningLabel stays quiet for healthy OAuth", () => {
	const label = resolveServerAuthWarningLabel({
		authMode: "oauth",
		oauthStatus: "connected",
		t: i18n.getFixedT("en", "servers"),
	});

	expect(label).toBeNull();
});

test("resolveElevatedServerWarningLabel prefers transport repair over auth", () => {
	const t = i18n.getFixedT("en", "servers");
	expect(
		resolveElevatedServerWarningLabel({
			requiresTransportRepair: true,
			authWarningLabel: "Reauthorize required",
			t,
		}),
	).toBe("Repair required");
	expect(
		resolveElevatedServerWarningLabel({
			requiresTransportRepair: false,
			authWarningLabel: "Reauthorize required",
			t,
		}),
	).toBe("Reauthorize required");
});
