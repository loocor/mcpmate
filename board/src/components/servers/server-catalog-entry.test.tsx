import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

import "../../lib/i18n/index";
import { ServerCatalogEntry } from "./server-catalog-entry";

test("list entries keep the enable switch without the legacy inspect action", () => {
	const legacyDebugProps = {
		enableServerDebug: true,
		onOpenDebug: () => undefined,
	};
	const markup = renderToStaticMarkup(
		<ServerCatalogEntry
			{...legacyDebugProps}
			variant="list"
			server={{
				id: "server-one",
				name: "Server One",
				status: "Ready",
				enabled: true,
			}}
			statsLabels={{
				tools: "Tools",
				prompts: "Prompts",
				resources: "Resources",
				templates: "Templates",
			}}
			onOpen={() => undefined}
			onToggle={() => undefined}
			isToggleDisabled={false}
		/>,
	);

	expect(markup).toContain('role="switch"');
	expect(markup).not.toContain("Open inspect view");
});
