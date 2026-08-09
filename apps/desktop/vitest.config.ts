import { defineConfig } from "vitest/config";
import { fileURLToPath, URL } from "node:url";

export default defineConfig({
  resolve: {
    alias: {
      $domain: fileURLToPath(new URL("./src/domain", import.meta.url)),
      $usecases: fileURLToPath(new URL("./src/usecases", import.meta.url)),
      $components: fileURLToPath(new URL("./src/components", import.meta.url)),
      $views: fileURLToPath(new URL("./src/Views", import.meta.url)),
    },
  },
  test: {
    environment: "node",
    include: ["src/**/*.{test,spec}.ts"],
  },
});
