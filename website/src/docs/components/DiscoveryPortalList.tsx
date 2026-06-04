import { ExternalLink } from "lucide-react";
import { useEffect, useState } from "react";
import {
	fetchWebsiteDiscoveryPortals,
	type WebsiteDiscoveryPortal,
} from "../../lib/admin-discovery";

interface DiscoveryPortalListCopy {
	loading: string;
	error: string;
	empty: string;
	visit: string;
	source: string;
}

export default function DiscoveryPortalList({
	copy,
}: {
	copy: DiscoveryPortalListCopy;
}) {
	const [portals, setPortals] = useState<WebsiteDiscoveryPortal[]>([]);
	const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");

	useEffect(() => {
		let cancelled = false;
		setStatus("loading");
		fetchWebsiteDiscoveryPortals()
			.then((items) => {
				if (cancelled) return;
				setPortals(items);
				setStatus("ready");
			})
			.catch(() => {
				if (cancelled) return;
				setStatus("error");
			});
		return () => {
			cancelled = true;
		};
	}, []);

	if (status === "loading") {
		return <p className="leading-7 text-brand-muted">{copy.loading}</p>;
	}

	if (status === "error") {
		return <p className="leading-7 text-brand-muted">{copy.error}</p>;
	}

	if (!portals.length) {
		return <p className="leading-7 text-brand-muted">{copy.empty}</p>;
	}

	return (
		<div className="not-prose grid gap-3 sm:grid-cols-2">
			{portals.map((portal) => (
				<a
					key={portal.id}
					href={portal.url}
					target="_blank"
					rel="noopener noreferrer"
					className="rounded-md border border-brand-border-subtle bg-brand-surface/70 p-4 transition-colors hover:border-brand-accent/60 hover:bg-brand-surface"
				>
					<div className="flex items-start gap-3">
						{portal.iconUrl ? (
							<img
								src={portal.iconUrl}
								alt=""
								className="mt-0.5 h-8 w-8 rounded-md object-contain"
								loading="lazy"
							/>
						) : (
							<div className="mt-0.5 flex h-8 w-8 items-center justify-center rounded-md bg-brand-overlay text-sm font-semibold text-brand-foreground">
								{portal.title.charAt(0).toUpperCase()}
							</div>
						)}
						<div className="min-w-0 flex-1">
							<div className="flex items-center gap-2 text-sm font-semibold text-brand-foreground">
								<span className="truncate">{portal.title}</span>
								<ExternalLink className="h-3.5 w-3.5 shrink-0" />
							</div>
							{portal.description ? (
								<p className="mt-1 line-clamp-3 text-sm leading-6 text-brand-muted">
									{portal.description}
								</p>
							) : null}
							<div className="mt-3 flex flex-wrap gap-2 text-xs text-brand-muted-soft">
								{portal.signal ? <span>{portal.signal}</span> : null}
								{portal.meta ? <span>{portal.meta}</span> : null}
								{portal.source ? <span>{`${copy.source}: ${portal.source}`}</span> : null}
							</div>
						</div>
					</div>
					<span className="sr-only">{copy.visit}</span>
				</a>
			))}
		</div>
	);
}
