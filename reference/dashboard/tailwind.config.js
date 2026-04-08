/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        dark: {
          DEFAULT: '#0a0a0f',
          lighter: '#1a1a1f',
        },
        slate: {
          850: '#1e293b',
          900: '#0f172a',
          950: '#020617',
        },
        finance: {
          primary: '#3b82f6',
          secondary: '#64748b',
          success: '#10b981',
          warning: '#f59e0b',
          danger: '#ef4444',
          info: '#06b6d4',
        },
      },
      fontFamily: {
        sans: ['Inter', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'sans-serif'],
        mono: ['JetBrains Mono', 'Consolas', 'Monaco', 'Courier New', 'monospace'],
      },
    },
  },
  plugins: [
    require('@tailwindcss/typography'),
  ],
}
