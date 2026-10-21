# syntax=docker/dockerfile:1.7
# Image du serveur XpePerience (mode équipe). Aucun secret n'est intégré à l'image :
# tout se configure par variables d'environnement au lancement (voir .env.example).

# ── 1. Interface Vue ────────────────────────────────────────────────────────
FROM node:25-bookworm-slim AS web
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
RUN cargo build --release --locked -p xpeperience-server

# ── 3. Image finale : minuscule, sans compilateur, utilisateur sans privilèges ──
FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --home-dir /data --shell /usr/sbin/nologin xpeperience \
    && mkdir -p /data && chown xpeperience:xpeperience /data
COPY --from=build /src/target/release/xpeperience-server /usr/local/bin/xpeperience-server

ENV XPEPERIENCE_DATA_DIR=/data \
    XPEPERIENCE_BIND=0.0.0.0 \
    XPEPERIENCE_PORT=8080
USER xpeperience
VOLUME /data
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 CMD ["xpeperience-server", "healthcheck"]
ENTRYPOINT ["xpeperience-server"]
