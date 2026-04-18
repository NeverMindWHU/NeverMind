import react from "@vitejs/plugin-react";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;
const __dirname = path.dirname(fileURLToPath(import.meta.url));

// https://vite.dev/config/
export default defineConfig({
  // Tauri 打包后从 file:// 或 app 协议加载，必须用相对资源路径，否则 JS/CSS 加载失败会白屏
  base: "./",
  plugins: [react()],
  clearScreen: false,
  build: {
    rollupOptions: {
      input: {
        main: path.resolve(__dirname, "index.html"),
        landing: path.resolve(__dirname, "landing.html"),
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
