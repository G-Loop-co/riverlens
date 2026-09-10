import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 1420,
    strictPort: true,
    proxy: { "/api": "http://127.0.0.1:4789" },
    watch: {
      ignored: [
        "**/target/**",
        "**/.local/**",
        "**/docs/**",
        "**/crates/**",
        "**/src-tauri/**",
        "**/dist/**",
        "**/deliverables/**",
      ].map(
        (pattern) => `${process.cwd().replace(/\\/g, "/")}/${pattern.slice(3)}`,
      ),
    },
  },
  clearScreen: false,
  envPrefix: ["VITE_", "TAURI_ENV_"],
});
