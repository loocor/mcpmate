import { useLanguage } from "../../components/LanguageProvider";

export default function CornerRibbon() {
  const { t } = useLanguage();
  // Accessible label for screen readers; no visual tooltip
  const a11y = t("notice.construction.ribbon");

  return (
    <div className="fixed bottom-0 right-0 z-[99999] print:hidden pointer-events-none">
      {/* Wrapper for the corner wedge */}
      <div className="relative h-[72px] w-[72px] select-none">
        {/* Triangle badge */}
        <div
          role="note"
          aria-label={a11y}
          className="absolute inset-0"
        >
          <div
            className="absolute bottom-0 right-0 h-[72px] w-[72px] bg-yellow-400 shadow-lg ring-1 ring-yellow-500/40"
            style={{ clipPath: "polygon(100% 0, 0 100%, 100% 100%)" }}
          />
          {/* White construction barrier icon inside the wedge */}
          <div className="pointer-events-none absolute bottom-2 right-2 text-white drop-shadow">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              width="32"
              height="32"
              aria-hidden="true"
              focusable="false"
              className="block corner-breathe"
              fill="currentColor"
            >
              {/* Simplified water barrier silhouette */}
              <path d="M4 10c0-.55.45-1 1-1h14c.55 0 1 .45 1 1v2.5c0 .28-.22.5-.5.5H18l1.2 3H20a1 1 0 1 1 0 2H4a1 1 0 1 1 0-2h.8L6 13h-1.5a.5.5 0 0 1-.5-.5V10Zm4.2 0L7 12h3l1.2-2H8.2Zm4 0L11 12h3l1.2-2h-2Zm4 0L15 12h3l1.2-2h-2Z" />
            </svg>
          </div>
        </div>
      </div>
    </div>
  );
}
