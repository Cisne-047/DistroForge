# Servidor DistroForge (Rust) con QEMU para el test drive.
FROM rust:1-slim AS build
WORKDIR /app
# rusqlite "bundled" compila SQLite en C: hace falta toolchain de C.
RUN apt-get update \
    && apt-get install -y --no-install-recommends gcc libc6-dev \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY tests ./tests
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends qemu-system-x86 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/distroforge-server /usr/local/bin/distroforge-server
ENV PORT=8080
EXPOSE 8080
CMD ["distroforge-server"]
