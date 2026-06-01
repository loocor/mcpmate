import { Check, Globe } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";
import { useLanguageNavigation } from "../../hooks/useLanguageNavigation";
import { getLanguageOption, LANGUAGE_OPTIONS } from "../../lib/languages";
import type { Language } from "../LanguageProvider";
import { useLanguage } from "../LanguageProvider";

/** Matches Navbar text links — shared for vertical alignment. */
export const NAV_UTILITY_TRIGGER_CLASS =
	"relative inline-flex items-center justify-center border-0 bg-transparent p-0 pb-1 text-brand-foreground/85 transition-colors hover:text-brand-accent cursor-pointer";

type LanguageSwitcherProps = {
	className?: string;
};

const LanguageSwitcher = ({ className = "" }: LanguageSwitcherProps) => {
	const { t } = useLanguage();
	const { language, selectLanguage } = useLanguageNavigation();
	const [open, setOpen] = useState(false);
	const rootRef = useRef<HTMLDivElement>(null);
	const listboxId = useId();
	const activeOption = getLanguageOption(language);

	const close = () => setOpen(false);

	const handleSelect = (code: Language) => {
		selectLanguage(code);
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
		<div ref={rootRef} className={`relative ${className}`}>
			<button
				type="button"
				className={`${NAV_UTILITY_TRIGGER_CLASS} ${open ? "text-brand-accent" : ""}`}
				aria-label={`${t("nav.language")}: ${activeOption.label}`}
				aria-haspopup="true"
				aria-expanded={open}
				aria-controls={listboxId}
				onClick={() => setOpen((value) => !value)}
			>
				<Globe size={18} strokeWidth={1.75} aria-hidden className="block translate-y-[3px]" />
			</button>

			{open ? (
				<ul
					id={listboxId}
					aria-label={t("nav.language")}
					className="absolute right-0 top-[calc(100%+0.625rem)] z-[100] min-w-[11rem] overflow-hidden rounded-lg border border-brand-border-subtle bg-brand-bg py-1 shadow-card"
				>
					{LANGUAGE_OPTIONS.map((option) => {
						const isActive = option.code === language;
						return (
							<li key={option.code} role="presentation">
								<button
									type="button"
									aria-current={isActive ? "true" : undefined}
									className={`flex w-full items-center gap-2 px-3 py-2 text-left text-sm transition-colors ${
										isActive
											? "bg-brand-overlay text-brand-accent"
											: "text-brand-foreground/90 hover:bg-brand-overlay-hover hover:text-brand-accent"
									}`}
									onClick={() => handleSelect(option.code)}
								>
									<span
										className={`inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-sm text-xs font-medium ${
											isActive
												? "bg-brand-accent text-brand-accent-fg"
												: "bg-brand-overlay-strong text-brand-foreground/80"
										}`}
									>
										{option.badge}
									</span>
									<span className="flex-1">{option.label}</span>
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

export default LanguageSwitcher;
