/** Entrada del servidor: escucha en PORT (8080 por defecto). */
import { buildServer } from "./routes.js";

const port = Number(process.env.PORT ?? 8080);
const app = buildServer({ qemuBin: process.env.QEMU_BIN });

app
  .listen({ port, host: "0.0.0.0" })
  .then(() => console.log(`DistroForge server en http://0.0.0.0:${port}`))
  .catch((e: unknown) => {
    console.error(e);
    process.exit(1);
  });
