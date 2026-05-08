import { useEffect } from "react";
import { useAppStore } from "../lib/store";

export function ThemeSynchronizer() {
  const theme = useAppStore((state) => state.theme);

  useEffect(() => {
    const apply = () => {
      const isDark =
        theme === "dark" ||
        (theme === "system" &&
          window.matchMedia("(prefers-color-scheme: dark)").matches);
      document.documentElement.classList.toggle("dark", isDark);
    };

    apply();

    let mediaQuery: MediaQueryList | null = null;
    const onChange = (event: MediaQueryListEvent) => {
      if (theme === "system") {
        document.documentElement.classList.toggle("dark", event.matches);
      }
    };

    if (theme === "system") {
      mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
      mediaQuery.addEventListener("change", onChange);
    }

    return () => {
      if (mediaQuery) {
        mediaQuery.removeEventListener("change", onChange);
      }
    };
  }, [theme]);

  return null;
}
