import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

import "../lib/i18n/index";
import { CapabilityEmptyState } from "./capability-empty-state";
import { CapabilityPreviewList } from "./capability-preview-list";

test("renders provided content inside an empty capability surface", () => {
	const markup = renderToStaticMarkup(
		<CapabilityPreviewList
			showHeader={false}
			framed={false}
			emptyContent={
				<CapabilityEmptyState
					title="Some capabilities could not be discovered"
					description="Capability discovery was incomplete."
					actionLabel="View discovery logs"
					onAction={() => undefined}
				/>
			}
		/>,
	);

	expect(markup).toContain("Some capabilities could not be discovered");
	expect(markup).toContain("Capability discovery was incomplete.");
	expect(markup).toContain(">View discovery logs</button>");
	expect(markup).not.toContain("No capabilities discovered for this server.");
});
