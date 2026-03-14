import { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { getResolvedTheme, applyThemeToDocument, type ThemeMode, type Theme } from '../utils/theme-utils';

interface ThemeContextType {
  mode: ThemeMode;
  theme: Theme;
  setMode: (mode: ThemeMode) => void;
  toggleTheme: () => void;
}

export const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [mode, setMode] = useState<ThemeMode>(() => {
    if (typeof window !== 'undefined') {
      const saved = (localStorage.getItem('theme') || localStorage.getItem('themeMode')) as ThemeMode | null;
      if (saved === 'light' || saved === 'dark' || saved === 'system') return saved;
    }
    return 'system';
  });

  const resolvedTheme: Theme = getResolvedTheme(mode);

  useEffect(() => {
    applyThemeToDocument(resolvedTheme);
    localStorage.setItem('theme', mode);
    localStorage.setItem('themeMode', mode);
  }, [mode, resolvedTheme]);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    
    const handleChange = () => {
      const saved = (localStorage.getItem('theme') || localStorage.getItem('themeMode')) as ThemeMode | null;
      if (!saved || saved === 'system') {
        setMode('system');
      }
    };
    
    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, []);

  const toggleTheme = () => {
    setMode(prev => (prev === 'dark' ? 'light' : 'dark'));
  };

  return (
    <ThemeContext.Provider value={{ mode, theme: resolvedTheme, setMode, toggleTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const context = useContext(ThemeContext);
  if (context === undefined) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
}
