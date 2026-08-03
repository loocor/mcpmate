import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

import "../lib/i18n/index";
import { Pagination } from "./pagination";

test("renders the page indicator in compact current slash total form", () => {
	const markup = renderToStaticMarkup(
		<Pagination
			currentPage={1}
			hasPreviousPage={false}
			hasNextPage={true}
			itemsPerPage={10}
			currentPageItemCount={10}
			totalPages={2}
			onGoToPage={() => undefined}
			onPreviousPage={() => undefined}
			onNextPage={() => undefined}
		/>,
	);

	expect(markup).toContain('aria-label="Go to page"');
	expect(markup).toContain('value="1"');
	expect(markup).toContain(">/</span>");
	expect(markup).toContain('aria-hidden="true">2</span>');
	expect(markup).toContain('class="sr-only">of 2</span>');
	expect(markup).not.toContain(">Page</span>");
});

test("preserves the localized page context when total pages are unknown", () => {
	const markup = renderToStaticMarkup(
		<Pagination
			currentPage={1}
			hasPreviousPage={false}
			hasNextPage={true}
			itemsPerPage={10}
			currentPageItemCount={10}
			onGoToPage={() => undefined}
			onPreviousPage={() => undefined}
			onNextPage={() => undefined}
		/>,
	);

	expect(markup).toContain(">Page</span>");
	expect(markup).toContain('aria-label="Go to page"');
	expect(markup).toContain('value="1"');
	expect(markup).not.toContain(">/</span>");
});
