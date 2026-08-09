import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath, URL } from "node:url";

// Tauri espera un puerto fijo y falla si no está disponible.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  resolve: {
    alias: {
      $domain: fileURLToPath(new URL("./src/domain", import.meta.url)),
      $usecases: fileURLToPath(new URL("./src/usecases", import.meta.url)),
      $components: fileURLToPath(new URL("./src/components", import.meta.url)),
      $views: fileURLToPath(new URL("./src/Views", import.meta.url)),
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      // No vigilar el backend Rust desde Vite.
      ignored: ["**/src-tauri/**"],
    },
  },
});
