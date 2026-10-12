# Servidor DistroForge — galería con cuentas, búsqueda y Test Drive VNC
FROM node:24-slim

# QEMU para las sesiones de Test Drive (opcional: la API degrada a 503 sin él)
RUN apt-get update && apt-get install -y --no-install-recommends \
    qemu-system-x86 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm install
COPY . .
RUN npm run build

ENV PORT=8080
EXPOSE 8080
CMD ["node", "dist/index.js"]
