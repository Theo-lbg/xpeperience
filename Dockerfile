# syntax=docker/dockerfile:1.7
# Image du serveur Boond by XPEHO (mode équipe). Aucun secret n'est intégré à l'image :
# tout se configure par variables d'environnement au lancement (voir .env.example).

# ── 1. Interface Vue ────────────────────────────────────────────────────────
FROM node:22-bookworm-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web/ ./
RUN npm run build

# ── 2. Serveur Rust ─────────────────────────────────────────────────────────
FROM rust:1-bookworm AS build
WORKDIR /src
# Le manifeste de l'application de bureau doit exister pour résoudre l'espace de travail ;
# elle n'est pas compilée ici.
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY desktop/Cargo.toml desktop/build.rs desktop/
COPY desktop/src desktop/src
COPY --from=web /src/web/dist web/dist
RUN cargo build --release --locked -p boond-server

# ── 3. Image finale : minuscule, sans compilateur, utilisateur sans privilèges ──
FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --home-dir /data --shell /usr/sbin/nologin boond \
    && mkdir -p /data && chown boond:boond /data
COPY --from=build /src/target/release/boond-server /usr/local/bin/boond-server

ENV BOOND_DATA_DIR=/data \
    BOOND_BIND=0.0.0.0 \
    BOOND_PORT=8080
USER boond
VOLUME /data
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 CMD ["boond-server", "healthcheck"]
ENTRYPOINT ["boond-server"]
