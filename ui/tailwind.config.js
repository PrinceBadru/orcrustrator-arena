/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        arena: {
          bg: '#0a0d14',
          card: '#111520',
          border: '#1e2433',
          accent: '#ff5500', // Orcrustrator orange
          naive: '#ef4444',  // Red
          handrolled: '#3b82f6', // Blue
          orcrustrator: '#10b981', // Emerald
        },
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
    },
  },
  plugins: [],
};
