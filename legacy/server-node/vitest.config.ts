import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // node:sqlite (Node 24) no es un builtin conocido por vite 5: externaliza todos los módulos node:
    server: { deps: { external: [/^node:/] } },
  },
});
