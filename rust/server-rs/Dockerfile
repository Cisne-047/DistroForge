# Servidor DistroForge (Rust) con QEMU para el Test Drive.
#
#   docker build -t distroforge-server rust/server-rs
#   docker run --rm -p 8080:8080 -v df-data:/data distroforge-server
#
# El Test Drive arranca QEMU dentro del contenedor: sin /dev/kvm funciona, pero
# muy lento. Ver docs/DESPLIEGUE.md.

FROM rust:1-slim AS build
WORKDIR /app

# rusqlite "bundled" compila SQLite en C: hace falta toolchain de C.
RUN apt-get update \
    && apt-get install -y --no-install-recommends gcc libc6-dev pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 1) Compila solo las dependencias con un src mínimo, para que la capa quede en
#    caché y los cambios de código no recompilen todo el árbol.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo '' > src/lib.rs \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends qemu-system-x86 curl ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home distroforge

COPY --from=build /app/target/release/distroforge-server /usr/local/bin/distroforge-server

ENV PORT=8080 \
    DF_DB_PATH=/data/df.db \
    QEMU_BIN=/usr/bin/qemu-system-x86_64

RUN mkdir -p /data && chown -R distroforge:distroforge /data
VOLUME ["/data"]
USER distroforge
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -fsS "http://127.0.0.1:${PORT}/api/health" || exit 1

CMD ["distroforge-server"]
