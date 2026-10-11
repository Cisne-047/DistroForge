FROM debian:bookworm
LABEL org.opencontainers.image.title="mi-distro"
LABEL org.opencontainers.image.description="Distro personalizada generada con DistroForge"
ENV DEBIAN_FRONTEND=noninteractive
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends sudo git vim htop; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*; \
    useradd -m -s /bin/bash -G sudo user; \
    echo 'user:distroforge-test' | chpasswd

USER user
WORKDIR /home/user
CMD ["/bin/bash"]
