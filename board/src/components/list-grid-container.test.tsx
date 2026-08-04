import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

import { useAppStore } from "../lib/store";
import { ListGridContainer } from "./list-grid-container";

test("explicit view mode overrides the stored default layout", () => {
	useAppStore.getState().setDashboardSetting("defaultView", "grid");

	const markup = renderToStaticMarkup(
		<ListGridContainer viewMode="list">
			<div>List item</div>
		</ListGridContainer>,
	);

	expect(markup).toContain("space-y-4");
	expect(markup).not.toContain("grid-cols");
});
