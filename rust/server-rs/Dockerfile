# Servidor DistroForge (Rust) con QEMU para el Test Drive y la interfaz web.
#
# El contexto es la RAÍZ del repositorio (la imagen incluye la UI):
#
#   cd rust/web-ui && trunk build --release --public-url /
#   docker build -f rust/server-rs/Dockerfile -t distroforge-server .
#   docker run --rm -p 8080:8080 -v df-data:/data distroforge-server
#
# Con la UI dentro, el contenedor sirve la aplicación y la API en el mismo
# origen, que es lo que hace que la cookie de sesión viaje (ver
# docs/DESPLIEGUE.md). El Test Drive arranca QEMU: sin /dev/kvm funciona, pero
# muy lento.

# El compilador va en la MISMA base que el runtime (bookworm): `rust:1-slim` es
# trixie (glibc 2.39) y el binario salía pidiendo símbolos que bookworm (2.36) no
# tiene. Hoy solo avisaba de una versión débil y arrancaba, pero es frágil:
# emparejar las bases elimina el problema de raíz.
FROM rust:1-slim-bookworm AS build
WORKDIR /app

# rusqlite "bundled" compila SQLite en C: hace falta toolchain de C.
RUN apt-get update \
    && apt-get install -y --no-install-recommends gcc libc6-dev pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 1) Compila solo las dependencias con un src mínimo, para que la capa quede en
#    caché y los cambios de código no recompilen todo el árbol.
COPY rust/server-rs/Cargo.toml rust/server-rs/Cargo.lock ./
RUN mkdir -p src \
    && echo '' > src/lib.rs \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY rust/server-rs/src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends qemu-system-x86 curl ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home distroforge

COPY --from=build /app/target/release/distroforge-server /usr/local/bin/distroforge-server

# Interfaz ya compilada: DF_WEB_DIR la sirve junto a la API.
COPY rust/web-ui/dist /app/web

ENV PORT=8080 \
    DF_DB_PATH=/data/df.db \
    QEMU_BIN=/usr/bin/qemu-system-x86_64 \
    DF_WEB_DIR=/app/web

RUN mkdir -p /data && chown -R distroforge:distroforge /data /app
VOLUME ["/data"]
USER distroforge
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -fsS "http://127.0.0.1:${PORT}/api/health" || exit 1

CMD ["distroforge-server"]
