import data from "../../changelog/en.json";
import { H2 } from "../../components/Headings";
import DocLayout from "../../layout/DocLayout";

type Release = {
	version: string;
	date: string; // YYYY-MM-DD
	highlights?: string[];
	changes: {
		type: "feat" | "fix" | "chore" | "docs" | "refactor" | string;
		text: string;
	}[];
	tags?: string[];
};

export default function Changelog() {
	const releases = (data as Release[]).sort((a, b) =>
		a.date < b.date ? 1 : -1,
	);
	return (
		<DocLayout
			meta={{ title: "Changelog", description: "Product updates and releases" }}
		>
			{releases.map((r) => (
				<section
					key={r.version}
					className="not-prose border rounded-md px-4 py-2 border-slate-200 dark:border-slate-700"
				>
					<div className="flex items-center justify-between mb-1">
						<H2 id={`v-${r.version}`}>v{r.version}</H2>
						<div className="text-sm text-slate-500">{r.date}</div>
					</div>
					{r.highlights && r.highlights.length > 0 && (
						<div className="mb-3 text-slate-700 dark:text-slate-300">
							{r.highlights.map((h, i) => (
								<p key={i} className="text-sm leading-6">
									{h}
								</p>
							))}
						</div>
					)}
					<ul className="space-y-2 list-none pl-0">
						{r.changes.map((c, i) => (
							<li key={i} className="text-sm">
								<div className="flex items-start gap-3">
									<span className="inline-flex w-16 justify-center rounded bg-slate-100 dark:bg-slate-800 px-2 py-0.5 text-[11px] uppercase tracking-wide text-slate-600 dark:text-slate-300">
										{c.type}
									</span>
									<span className="text-slate-700 dark:text-slate-300 leading-6">
										{c.text}
									</span>
								</div>
							</li>
						))}
					</ul>
				</section>
			))}
		</DocLayout>
	);
}
