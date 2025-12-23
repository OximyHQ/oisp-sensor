import type { Config } from 'tailwindcss';

const config: Config = {
  content: [
    './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
    './src/components/**/*.{js,ts,jsx,tsx,mdx}',
    './src/app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {
      colors: {
        // Refined dark theme - sophisticated and professional
        bg: {
          primary: '#0a0c10',
          secondary: '#12151a',
          tertiary: '#1a1e24',
          elevated: '#22262e',
          hover: '#2a2f38',
        },
        border: {
          DEFAULT: '#2a2f38',
          muted: '#1e2228',
          subtle: '#1a1e24',
        },
        text: {
          primary: '#f0f2f5',
          secondary: '#9ca3af',
          muted: '#6b7280',
        },
        accent: {
          blue: '#3b82f6',
          green: '#22c55e',
          yellow: '#eab308',
          orange: '#f97316',
          red: '#ef4444',
          purple: '#a855f7',
          cyan: '#06b6d4',
          pink: '#ec4899',
        },
      },
      fontFamily: {
        mono: [
          'IBM Plex Mono',
          'SF Mono',
          'Fira Code',
          'Monaco',
          'Consolas',
          'monospace',
        ],
        sans: [
          'IBM Plex Sans',
          '-apple-system',
          'BlinkMacSystemFont',
          'Segoe UI',
          'Roboto',
          'sans-serif',
        ],
      },
      animation: {
        'fade-in': 'fadeIn 0.2s ease-out',
        'slide-up': 'slideInUp 0.25s ease-out',
        'slide-left': 'slideInLeft 0.25s ease-out',
        'pulse-subtle': 'pulseSubtle 2s infinite',
        'spin-slow': 'spin 2s linear infinite',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideInUp: {
          '0%': { transform: 'translateY(8px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        slideInLeft: {
          '0%': { transform: 'translateX(-8px)', opacity: '0' },
          '100%': { transform: 'translateX(0)', opacity: '1' },
        },
        pulseSubtle: {
          '0%, 100%': { opacity: '1' },
          '50%': { opacity: '0.6' },
        },
      },
      boxShadow: {
        'glow-blue': '0 0 20px rgba(59, 130, 246, 0.15)',
        'glow-green': '0 0 20px rgba(34, 197, 94, 0.15)',
        'glow-purple': '0 0 20px rgba(168, 85, 247, 0.15)',
      },
    },
  },
  plugins: [],
};

export default config;
