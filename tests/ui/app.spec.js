// Pruebas de interfaz de DistroForge.
//
// Cargan la aplicación **construida de verdad** (el WASM, no un mock) y
// comprueban lo que un usuario ve y pulsa. Es lo que faltaba: hasta ahora la UI
// solo se compilaba, así que un fallo como la ventana en blanco del escritorio no
// lo detectaba nada.
//
//   cd tests/ui && npm install && npx playwright install --with-deps chromium
//   npm test                       # contra rust/web-ui/dist servido en /
//
// Con `PW_CHANNEL=msedge` usa el Edge instalado en Windows en lugar de descargar
// Chromium (útil para probar en local).

const { test, expect } = require("@playwright/test");

test.beforeEach(async ({ page }) => {
  await page.goto("/");
});

test("la app arranca: cabecera, seis pasos y contenido del primer paso", async ({ page }) => {
  // Si el WASM no monta, aquí no hay nada de esto: es la prueba que detecta una
  // página en blanco.
  await expect(page.locator("header")).toContainText("DistroForge", { timeout: 15000 });

  const pasos = page.locator("nav button");
  await expect(pasos).toHaveCount(6);
  await expect(pasos.first()).toContainText("Base");
  await expect(pasos.nth(5)).toContainText("Construir");

  await expect(page.locator("h2").first()).toContainText("Elige la base");
});

test("elegir una base la refleja en el resumen", async ({ page }) => {
  await expect(page.locator("h2").first()).toContainText("Elige la base");

  await page.getByText("Debian 12 (bookworm)", { exact: false }).first().click();

  // el resumen de la izquierda tiene que decir Debian y su motor
  const resumen = page.locator("aside, .resumen").first();
  await expect(resumen).toContainText("Debian");
  await expect(resumen).toContainText("livebuild");
});

test("el catálogo incluye las distros nuevas", async ({ page }) => {
  // El catálogo se pinta desde el WASM: si falta una base, la tarjeta no existe.
  for (const nombre of [
    "Kali Linux",
    "MX Linux",
    "AlmaLinux",
    "Rocky Linux",
    "Oracle Linux",
    "Garuda",
    "SteamOS",
  ]) {
    await expect(page.getByText(nombre, { exact: false }).first()).toBeVisible();
  }
});

test("se puede navegar entre pasos", async ({ page }) => {
  await page.locator("nav button").nth(2).click();
  await expect(page.locator("h2").first()).not.toContainText("Elige la base");

  // y volver al primero
  await page.locator("nav button").first().click();
  await expect(page.locator("h2").first()).toContainText("Elige la base");
});

test("el último paso deja construir el proyecto", async ({ page }) => {
  await page.locator("nav button").nth(5).click();
  // el botón de descarga del ZIP del proyecto
  const descarga = page.getByRole("button", { name: /descargar|zip|generar/i });
  await expect(descarga.first()).toBeVisible();
});
