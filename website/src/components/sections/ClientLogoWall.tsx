import { useEffect, useMemo, useState } from "react";
import { Plus } from "lucide-react";
import { useNavigate } from "react-router-dom";
import {
	analyzeLogoNeedsDarkInvert,
	CLIENT_LOGO_DARK_INVERT_CLASS,
	logoMustNeverDarkInvert,
	logoNeedsDarkInvertFallback,
} from "../../lib/client-logo-dark-mode";
import { loadWebsiteClientPresets, type WebsiteClientPreset } from "../../lib/admin-discovery";
import { useLanguage } from "../LanguageProvider";
import Section from "../ui/Section";
import {
	CLIENT_TILE_GROUP_HOVER_CLASS,
	CLIENT_TILE_HOVER_CLASS,
	CLIENT_TILE_SHELL_TALL_CLASS,
} from "../marketing/clientTileStyles";

const CLIENT_TILE_ICON_SHELL_CLASS =
	"flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-xl bg-brand-overlay-strong ring-1 ring-brand-border-subtle";

function ClientTile({
	client,
	failedLogos,
	onLogoError,
}: {
	client: WebsiteClientPreset;
	failedLogos: Set<string>;
	onLogoError: (identifier: string) => void;
}) {
	const { t } = useLanguage();
	const showLogo = Boolean(client.logoUrl) && !failedLogos.has(client.identifier);
	const initial = client.displayName.trim().charAt(0).toUpperCase() || "?";
	const neverInvert = logoMustNeverDarkInvert(client.identifier);
	const [invertOnDark, setInvertOnDark] = useState(() =>
		neverInvert ? false : logoNeedsDarkInvertFallback(client.identifier),
	);

	useEffect(() => {
		if (!client.logoUrl || neverInvert) {
			return;
		}

		let cancelled = false;
		void analyzeLogoNeedsDarkInvert(client.logoUrl).then((needsInvert) => {
			if (cancelled) {
				return;
			}
			if (needsInvert === null) {
				setInvertOnDark(logoNeedsDarkInvertFallback(client.identifier));
				return;
			}
			// Successful sample: trust detection; do not keep a stale allowlist true.
			setInvertOnDark(needsInvert);
		});

		return () => {
			cancelled = true;
		};
	}, [client.identifier, client.logoUrl, neverInvert]);

	const logoClassName = ["h-8 w-8 rounded-lg object-contain", invertOnDark ? CLIENT_LOGO_DARK_INVERT_CLASS : ""]
		.filter(Boolean)
		.join(" ");

	const inner = (
		<div
			className={`${CLIENT_TILE_SHELL_TALL_CLASS} h-full ${CLIENT_TILE_GROUP_HOVER_CLASS}`}
		>
			<div
				className={`${CLIENT_TILE_ICON_SHELL_CLASS} transition-transform duration-200 group-hover:scale-105`}
			>
				{showLogo ? (
					<img
						src={client.logoUrl}
						alt={t("clients.logoAlt").replace("{name}", client.displayName)}
						className={logoClassName}
						loading="lazy"
						onError={() => onLogoError(client.identifier)}
					/>
				) : (
					<span className="text-base font-semibold text-brand-foreground">{initial}</span>
				)}
			</div>
			<p className="w-full truncate text-center text-xs font-medium text-brand-foreground opacity-60 transition-opacity duration-200 group-hover:opacity-100">
				{client.displayName}
			</p>
		</div>
	);

	const tileClassName =
		"group block h-full focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-accent focus-visible:ring-offset-2 focus-visible:ring-offset-brand-bg";

	if (client.homepageUrl) {
		return (
			<a
				href={client.homepageUrl}
				target="_blank"
				rel="noopener noreferrer"
				className={tileClassName}
			>
				{inner}
			</a>
		);
	}

	return <div className={tileClassName}>{inner}</div>;
}

const CLIENT_GRID_CLASS =
	"grid grid-cols-3 gap-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 lg:[grid-template-columns:repeat(6,9.5rem)] lg:justify-center";

/** Rough loading placeholder count (~4 rows at lg with 6 columns). */
const CLIENT_SKELETON_COUNT = 24;

const ClientLogoWall = () => {
	const { t, language } = useLanguage();
	const navigate = useNavigate();
	const [clients, setClients] = useState<WebsiteClientPreset[]>([]);
	const [status, setStatus] = useState<"loading" | "ready">("loading");
	const [failedLogos, setFailedLogos] = useState<Set<string>>(() => new Set());

	const loadPresets = () => {
		let cancelled = false;
		setStatus("loading");

		loadWebsiteClientPresets()
			.then(({ clients: items }) => {
				if (cancelled) {
					return;
				}
				setClients(items);
				setStatus("ready");
			});

		return () => {
			cancelled = true;
		};
	};

	useEffect(() => {
		const cancelLoad = loadPresets();
		return () => {
			if (cancelLoad) {
				cancelLoad();
			}
		};
	}, []);

	const docsPath = useMemo(() => {
		if (language === "zh") return "/docs/zh/clients";
		if (language === "ja") return "/docs/ja/clients";
		return "/docs/en/clients";
	}, [language]);

	return (
		<Section
			id="clients"
			title={t("clients.title")}
			subtitle={t("clients.subtitle")}
			centered
			className="py-16 md:py-20"
			titleClassName="text-3xl md:text-4xl text-brand-foreground"
			subtitleClassName="section-muted"
		>
			{status === "loading" ? (
				<div className={CLIENT_GRID_CLASS}>
					{Array.from({ length: CLIENT_SKELETON_COUNT }).map((_, index) => (
						<div key={index} className="min-h-[108px] animate-pulse rounded-xl bg-brand-overlay ring-1 ring-brand-border-subtle" />
					))}
				</div>
			) : null}

			{status === "ready" ? (
				<div className={CLIENT_GRID_CLASS}>
					{clients.map((client) => (
						<ClientTile
							key={client.identifier}
							client={client}
							failedLogos={failedLogos}
							onLogoError={(identifier) => setFailedLogos((previous) => new Set(previous).add(identifier))}
						/>
					))}
					<button
						type="button"
						onClick={() => navigate(docsPath)}
						className={`group h-full ${CLIENT_TILE_SHELL_TALL_CLASS} border-dashed bg-brand-elevated text-center ${CLIENT_TILE_HOVER_CLASS} focus:outline-none focus-visible:ring-2 focus-visible:ring-brand-accent focus-visible:ring-offset-2 focus-visible:ring-offset-brand-bg`}
					>
						<div
							className={`${CLIENT_TILE_ICON_SHELL_CLASS} bg-brand-accent/10 text-brand-accent ring-brand-accent/20 transition-transform duration-200 group-hover:scale-105`}
						>
							<Plus size={20} aria-hidden />
						</div>
						<span className="w-full truncate text-xs font-medium text-brand-foreground opacity-60 transition-opacity duration-200 group-hover:opacity-100">
							{t("clients.more")}
						</span>
					</button>
				</div>
			) : null}
		</Section>
	);
};

export default ClientLogoWall;
