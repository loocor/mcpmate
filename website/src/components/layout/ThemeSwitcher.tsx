import { Check, Monitor, Moon, Sun } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";
import type { ThemeMode } from "../../utils/theme-utils";
import { useLanguage } from "../LanguageProvider";
import { useTheme } from "../ThemeProvider";

const FOOTER_THEME_TRIGGER_CLASS =
	"footer-utility-trigger inline-flex h-8 w-8 items-center justify-center rounded-lg border-0 bg-transparent transition-all duration-150 hover:scale-105";

const THEME_OPTIONS = [
	{ mode: "light", icon: Sun, labelKey: "theme.light" },
	{ mode: "dark", icon: Moon, labelKey: "theme.dark" },
	{ mode: "system", icon: Monitor, labelKey: "theme.system" },
] as const;

const ThemeSwitcher = () => {
	const { t } = useLanguage();
	const { mode, setMode } = useTheme();
	const [open, setOpen] = useState(false);
	const rootRef = useRef<HTMLDivElement>(null);
	const listboxId = useId();
	const activeOption =
		THEME_OPTIONS.find((option) => option.mode === mode) ?? THEME_OPTIONS[0];
	const ActiveIcon = activeOption.icon;

	const close = () => setOpen(false);

	const handleSelect = (themeMode: ThemeMode) => {
		setMode(themeMode);
		close();
	};

	useEffect(() => {
		if (!open) {
			return;
		}

		const handlePointerDown = (event: MouseEvent) => {
			if (rootRef.current && !rootRef.current.contains(event.target as Node)) {
				close();
			}
		};

		const handleKeyDown = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				close();
			}
		};

		document.addEventListener("mousedown", handlePointerDown);
		document.addEventListener("keydown", handleKeyDown);
		return () => {
			document.removeEventListener("mousedown", handlePointerDown);
			document.removeEventListener("keydown", handleKeyDown);
		};
	}, [open]);

	return (
		<div ref={rootRef} className="relative">
			<button
				type="button"
				className={`${FOOTER_THEME_TRIGGER_CLASS} ${open ? "text-brand-accent opacity-100" : ""}`}
				aria-label={`${t("theme.label")}: ${t(activeOption.labelKey)}`}
				aria-haspopup="true"
				aria-expanded={open}
				aria-controls={listboxId}
				onClick={() => setOpen((value) => !value)}
			>
				<ActiveIcon size={18} strokeWidth={1.75} aria-hidden />
			</button>

			{open ? (
				<ul
					id={listboxId}
					aria-label={t("theme.label")}
					className="absolute bottom-[calc(100%+0.625rem)] right-0 z-[100] min-w-[11rem] overflow-hidden rounded-lg border border-brand-border-subtle bg-brand-bg py-1 shadow-card"
				>
					{THEME_OPTIONS.map((option) => {
						const isActive = option.mode === mode;
						const Icon = option.icon;
						return (
							<li key={option.mode} role="presentation">
								<button
									type="button"
									aria-current={isActive ? "true" : undefined}
									className={`flex w-full items-center gap-2 px-3 py-2 text-left text-sm transition-colors ${
										isActive
											? "bg-brand-overlay text-brand-accent"
											: "text-brand-foreground/90 hover:bg-brand-overlay-hover hover:text-brand-accent"
									}`}
									onClick={() => handleSelect(option.mode)}
								>
									<span
										className={`inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-sm ${
											isActive
												? "bg-brand-accent text-brand-accent-fg"
												: "bg-brand-overlay-strong text-brand-foreground/80"
										}`}
									>
										<Icon size={13} aria-hidden />
									</span>
									<span className="flex-1">{t(option.labelKey)}</span>
									{isActive ? <Check size={14} className="shrink-0 text-brand-accent" aria-hidden /> : null}
								</button>
							</li>
						);
					})}
				</ul>
			) : null}
		</div>
	);
};

export default ThemeSwitcher;
