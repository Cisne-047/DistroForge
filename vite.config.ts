import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // rutas relativas: la app funciona igual en Pages (/DistroForge/) que en cualquier otra ruta
  base: "./",
});
