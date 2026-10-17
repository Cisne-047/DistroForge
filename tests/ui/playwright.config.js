// Configuración de Playwright para las pruebas de interfaz.
//
// Sirve `rust/web-ui/dist` (la app compilada) y lanza los tests contra ella. El
// directorio se puede cambiar con `UI_DIST` y el puerto con `UI_PORT`.

const { defineConfig, devices } = require("@playwright/test");
const path = require("path");

const dist = process.env.UI_DIST || path.resolve(__dirname, "../../rust/web-ui/dist");
const port = Number(process.env.UI_PORT || 4173);

module.exports = defineConfig({
  testDir: __dirname,
  timeout: 45000,
  expect: { timeout: 10000 },
  reporter: [["list"]],
  use: {
    baseURL: `http://127.0.0.1:${port}/`,
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    // `PW_CHANNEL=msedge` evita descargar Chromium (Windows con Edge instalado)
    channel: process.env.PW_CHANNEL || undefined,
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    // el mismo servidor estático que usa la vista previa local
    command: `python3 -m http.server ${port} --bind 127.0.0.1 --directory "${dist}"`,
    port,
    reuseExistingServer: true,
    timeout: 30000,
  },
});
