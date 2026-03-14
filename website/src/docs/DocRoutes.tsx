import React from "react";
import { Navigate, Route } from "react-router-dom";
import { docsNav, flattenPages } from "./nav";
import { DelayedFallback } from "./components/DelayedFallback";

export function renderDocRoutes() {
	const routes: React.ReactElement[] = [];
	for (const nav of docsNav) {
		const pages = flattenPages(nav);
		for (const p of pages) {
			const C = React.lazy(p.component);
			routes.push(
				<Route
					key={p.path}
					path={p.path}
					element={
						<React.Suspense fallback={<DelayedFallback />}>
							<C />
						</React.Suspense>
					}
				/>,
			);
		}
	}

	routes.push(
		<Route
			key="docs-root"
			path="/docs"
			element={<Navigate to="/docs/en/quickstart" replace />}
		/>,
	);
	routes.push(
		<Route
			key="docs-en"
			path="/docs/en"
			element={<Navigate to="/docs/en/quickstart" replace />}
		/>,
	);
	routes.push(
		<Route
			key="docs-zh"
			path="/docs/zh"
			element={<Navigate to="/docs/zh/quickstart" replace />}
		/>,
	);

	return routes;
}
