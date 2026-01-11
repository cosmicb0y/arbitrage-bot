/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      animation: {
        "row-flash": "row-flash 0.5s ease-out",
        "live-dot": "live-dot 0.6s ease-in-out infinite",
      },
      keyframes: {
        "row-flash": {
          "0%": { backgroundColor: "rgba(59, 130, 246, 0.3)" },
          "100%": { backgroundColor: "transparent" },
        },
        "live-dot": {
          "0%, 100%": { opacity: "1", transform: "scale(1)" },
          "50%": { opacity: "0.4", transform: "scale(0.7)" },
        },
      },
      colors: {
        primary: {
          50: "#eff6ff",
          100: "#dbeafe",
          400: "#60a5fa",
          500: "#3b82f6",
          600: "#2563eb",
          700: "#1d4ed8",
        },
        success: {
          400: "#4ade80",
          500: "#22c55e",
          600: "#16a34a",
        },
        danger: {
          400: "#f87171",
          500: "#ef4444",
          600: "#dc2626",
        },
        orange: {
          400: "#fb923c",
          500: "#f97316",
        },
        dark: {
          600: "#4b5563",
          700: "#374151",
          750: "#2d3748",
          800: "#1f2937",
          850: "#1a202c",
          900: "#111827",
        },
      },
    },
  },
  plugins: [],
};
