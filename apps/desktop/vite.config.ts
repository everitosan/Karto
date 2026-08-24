import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { paraglideVitePlugin } from "@inlang/paraglide-js";
import { fileURLToPath, URL } from "node:url";

// Tauri espera un puerto fijo y falla si no está disponible.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [
    svelte(),
    // i18n: compila messages/{locale}.json a funciones tipadas en src/paraglide.
    // La detección real del idioma la hace $i18n/detect (plugin-os del SO); aquí
    // solo dejamos localStorage (elección persistida) → baseLocale (es) de fallback.
    paraglideVitePlugin({
      project: "./project.inlang",
      outdir: "./src/paraglide",
      strategy: ["localStorage", "baseLocale"],
    }),
  ],
  clearScreen: false,
  resolve: {
    alias: {
      $domain: fileURLToPath(new URL("./src/domain", import.meta.url)),
      $usecases: fileURLToPath(new URL("./src/usecases", import.meta.url)),
      $components: fileURLToPath(new URL("./src/components", import.meta.url)),
      $views: fileURLToPath(new URL("./src/Views", import.meta.url)),
      $config: fileURLToPath(new URL("./src/config", import.meta.url)),
      $i18n: fileURLToPath(new URL("./src/i18n", import.meta.url)),
      $paraglide: fileURLToPath(new URL("./src/paraglide", import.meta.url)),
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
