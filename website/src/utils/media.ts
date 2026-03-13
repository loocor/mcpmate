export function getIntroVideoUrl(): string | null {
  const env = import.meta.env as Record<string, string | undefined>;
  return env.VITE_INTRO_VIDEO_URL || null;
}

export function getIntroPoster(lightOrDark: 'light' | 'dark'): string | null {
  const env = import.meta.env as Record<string, string | undefined>;
  if (lightOrDark === 'light') return env.VITE_INTRO_POSTER_LIGHT || null;
  return env.VITE_INTRO_POSTER_DARK || null;
}

