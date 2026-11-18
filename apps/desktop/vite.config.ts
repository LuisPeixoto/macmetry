import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "node:path";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  // Two pages: the main window and the menu bar popover.
  build: {
    target: "es2022",
    chunkSizeWarningLimit: 1500,
    rollupOptions: { input: { main: resolve(__dirname, "index.html"), popover: resolve(__dirname, "popover.html") } },
  },
});
