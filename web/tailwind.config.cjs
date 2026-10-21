module.exports = {
  darkMode: 'class',
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  safelist: [
    'text-emerald-400', 'text-rose-400', 'text-slate-400',
    'bg-emerald-400', 'bg-rose-400', 'bg-slate-400',
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter Variable', 'Inter', 'system-ui', '-apple-system', 'sans-serif'],
        mono: ['JetBrains Mono Variable', 'ui-monospace', 'monospace'],
      },
      colors: { slate: { 850: '#131b2e', 925: '#0b101d', 950: '#060a12' } },
    },
  },
  plugins: [],
};
