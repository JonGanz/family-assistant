# syntax=docker/dockerfile:1

# ---- Rust: cargo-chef dependency caching ----
# Only Rust inputs are copied into these stages, so web/docs/script edits never invalidate them.
FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY .cargo .cargo
COPY crates crates
COPY bins bins
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ENV SQLX_OFFLINE=true
# Cooked from the recipe alone (manifests + lockfile): rebuilt only when dependencies change.
COPY --from=planner /app/recipe.json recipe.json
COPY rust-toolchain.toml ./
COPY .cargo .cargo
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json -p fa-server
# Only workspace crates recompile below, and only when Rust sources / sqlx data / config change.
COPY Cargo.toml Cargo.lock ./
COPY .sqlx .sqlx
COPY config config
COPY crates crates
COPY bins bins
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    cargo build --release -p fa-server

# ---- Web bundle (independent of Rust; builds in parallel under BuildKit) ----
FROM node:24-bookworm-slim AS web
RUN corepack enable
WORKDIR /web
COPY apps/web/package.json apps/web/pnpm-lock.yaml ./
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    corepack pnpm@9 install --frozen-lockfile
COPY apps/web/ ./
RUN corepack pnpm@9 run build

# ---- Runtime ----
FROM debian:bookworm-slim AS runtime
# hadolint ignore=DL3008
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home --shell /usr/sbin/nologin fa
COPY --from=builder /app/target/release/fa-server /usr/local/bin/fa-server
COPY --from=web /web/dist /srv/web
ENV FA_WEB_DIST_DIR=/srv/web \
    FA_BIND_ADDR=0.0.0.0:8080
USER 10001
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD ["curl", "-fsS", "http://127.0.0.1:8080/healthz"]
ENTRYPOINT ["fa-server"]
CMD ["serve", "all"]
