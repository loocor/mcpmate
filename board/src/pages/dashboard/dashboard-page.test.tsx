import { expect, mock, test } from "bun:test";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import type { ReactNode } from "react";
import type { ServerSummary } from "../../lib/types";

import "../../lib/i18n/index";

mock.module("../../components/metrics-trend-chart", () => ({
	MetricsTrendChart: () => <div>Metrics chart</div>,
}));
mock.module("../../components/token-savings-trend-card", () => ({
	TokenSavingsTrendCard: () => <div>Token savings chart</div>,
}));
mock.module("react-router-dom", () => ({
	Link: ({
		children,
		className,
		to,
	}: {
		children: ReactNode;
		className?: string;
		to: string;
	}) => (
		<a className={className} href={to}>
			{children}
		</a>
	),
}));

async function renderDashboardMarkup(
	servers?: ServerSummary[],
): Promise<string> {
	const { DashboardPage } = await import("./dashboard-page");
	const queryClient = new QueryClient({
		defaultOptions: {
			queries: {
				retry: false,
			},
		},
	});
	if (servers) {
		queryClient.setQueryData(["servers"], { servers });
	}
	queryClient.setQueryData(["surfaceReviews", "pending"], []);
	queryClient.setQueryData(["surfaceReviews", "summary"], {
		failed_reconciliation_count: 0,
	});
	return renderToStaticMarkup(
		<QueryClientProvider client={queryClient}>
			<DashboardPage />
		</QueryClientProvider>,
	);
}

test("keeps fixed dashboard charts ahead of expanding review todos", async () => {
	const markup = await renderDashboardMarkup();

	const metricsIndex = markup.indexOf(">Metrics<");
	const reviewTodosIndex = markup.indexOf(">Review todos<");

	expect(metricsIndex).toBeGreaterThan(-1);
	expect(reviewTodosIndex).toBeGreaterThan(-1);
	expect(metricsIndex).toBeLessThan(reviewTodosIndex);
});

test("lets review todos fill the remaining dashboard height", async () => {
	const markup = await renderDashboardMarkup();

	expect(markup).toStartWith(
		'<div class="flex min-h-full flex-col gap-4">',
	);
	expect(markup).toContain(
		"transition-shadow duration-200 flex flex-1 flex-col",
	);
});

test("adds an independent transport repair todo that links to the focused editor", async () => {
	const markup = await renderDashboardMarkup([
		{
			id: "server-invalid",
			name: "invalid-server",
			transport_validity: {
				state: "invalid",
				diagnostics: [
					{ code: "stdio_command_missing", field: "command" },
				],
			},
		},
	]);

	expect(markup).toContain("invalid-server");
	expect(markup).toContain(
		'/servers/server-invalid?edit=1&amp;focus=command',
	);
});
