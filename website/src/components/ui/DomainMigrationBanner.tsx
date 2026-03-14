import { X } from "lucide-react";
import { useState, useEffect } from "react";
import { useLanguage } from "../LanguageProvider";

export default function DomainMigrationBanner() {
  const [isVisible, setIsVisible] = useState(true);
  const { language } = useLanguage();

  useEffect(() => {
    document.documentElement.style.setProperty("--banner-height", isVisible ? "36px" : "0px");
    return () => {
      document.documentElement.style.setProperty("--banner-height", "0px");
    };
  }, [isVisible]);

  if (!isVisible) return null;

  const message = language === "zh"
    ? "域名迁移通知：mcpmate.io 即将停用，请使用新域名 "
    : "Domain Migration: mcpmate.io will be deprecated. Please use ";

  return (
    <div className="fixed top-0 left-0 right-0 z-[100] bg-amber-500 text-white text-sm py-2 px-4 print:hidden">
      <div className="container mx-auto flex items-center justify-center gap-2">
        <span className="font-medium">
          {message}
          <a
            href="https://mcp.umate.ai"
            className="underline hover:no-underline"
          >
            mcp.umate.ai
          </a>
        </span>
        <button
          onClick={() => setIsVisible(false)}
          className="ml-2 p-1 rounded hover:bg-amber-600 transition-colors"
          aria-label="Close notification"
        >
          <X size={14} />
        </button>
      </div>
    </div>
  );
}
