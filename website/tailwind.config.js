/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        brand: {
          bg: 'var(--brand-bg)',
          surface: 'var(--brand-surface)',
          elevated: 'var(--brand-elevated)',
          foreground: 'var(--brand-foreground)',
          muted: 'var(--brand-muted)',
          'muted-soft': 'var(--brand-muted-soft)',
          accent: 'var(--brand-accent)',
          'accent-hover': 'var(--brand-accent-hover)',
          'accent-fg': 'var(--brand-accent-fg)',
          indigo: 'var(--brand-indigo)',
          border: 'var(--brand-border)',
          'border-subtle': 'var(--brand-border-subtle)',
          overlay: 'var(--brand-overlay)',
          'overlay-hover': 'var(--brand-overlay-hover)',
          'overlay-strong': 'var(--brand-overlay-strong)',
          input: 'var(--brand-input-bg)',
          dot: 'var(--brand-dot-idle)',
          'dot-hover': 'var(--brand-dot-hover)',
        },
      },
      fontFamily: {
        sans: ['"IBM Plex Sans"', 'system-ui', 'sans-serif'],
        display: ['"IBM Plex Sans"', 'system-ui', 'sans-serif'],
        mono: ['"JetBrains Mono"', 'ui-monospace', 'monospace'],
      },
      boxShadow: {
        glow: 'var(--brand-shadow-glow)',
        'glow-sm': 'var(--brand-shadow-glow-sm)',
        glass: 'var(--brand-shadow-glass)',
        card: 'var(--brand-shadow-card)',
      },
      animation: {
        'aurora-drift': 'aurora-drift 12s ease-in-out infinite alternate',
        'aurora-drift-slow': 'aurora-drift 16s ease-in-out infinite alternate-reverse',
      },
      keyframes: {
        'aurora-drift': {
          '0%': { transform: 'translate(0, 0) scale(1)' },
          '100%': { transform: 'translate(4%, -3%) scale(1.05)' },
        },
      },
    },
  },
  plugins: [],
};
