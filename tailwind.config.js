/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          "'Segoe UI'",
          "'PingFang SC'",
          "'Microsoft YaHei'",
          "'Helvetica Neue'",
          "Helvetica",
          "Arial",
          "sans-serif",
        ],
      },
      colors: {
        brand: {
          50: "#eef7ff",
          100: "#d9ecff",
          200: "#bcdeff",
          300: "#8ec9ff",
          400: "#5aacff",
          500: "#2f8cff",
          600: "#1871ef",
          700: "#155adb",
          800: "#174cb0",
          900: "#18438b",
        },
        ink: {
          50: "#f8fafc",
          100: "#f1f5f9",
          200: "#e2e8f0",
          300: "#cbd5e1",
          400: "#94a3b8",
          500: "#64748b",
          600: "#475569",
          700: "#334155",
          800: "#1e293b",
          900: "#0f172a",
        },
      },
      boxShadow: {
        card: "0 1px 3px 0 rgb(15 23 42 / 0.06), 0 1px 2px -1px rgb(15 23 42 / 0.04)",
        "card-hover":
          "0 10px 30px -10px rgb(15 23 42 / 0.15), 0 4px 10px -4px rgb(15 23 42 / 0.08)",
      },
      keyframes: {
        "fade-in": {
          "0%": { opacity: "0", transform: "translateY(4px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
        flip: {
          "0%": { transform: "rotateY(0deg)" },
          "100%": { transform: "rotateY(180deg)" },
        },
      },
      animation: {
        "fade-in": "fade-in 180ms ease-out",
      },
    },
  },
  plugins: [],
};
