# syntax=docker/dockerfile:1
FROM rust:1-bookworm AS builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app/backend
COPY backend/Cargo.toml backend/Cargo.lock backend/build.rs ./
COPY backend/src ./src
COPY backend/migrations ./migrations
COPY openapi.yaml swagger.html /app/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/backend/target \
    cargo build --release --locked \
    && mkdir -p /out \
    && cp target/release/obor-backend /out/obor-backend

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 backend \
    && useradd --uid 10001 --gid backend --no-create-home backend
COPY --from=builder /out/obor-backend /usr/local/bin/obor-backend
COPY --chmod=755 backend/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
# Normalize CRLF even when the build context came from a Windows checkout.
RUN sed -i 's/\r$//' /usr/local/bin/docker-entrypoint.sh \
    && /bin/sh -n /usr/local/bin/docker-entrypoint.sh
USER backend
EXPOSE 8000
HEALTHCHECK --interval=10s --timeout=3s --start-period=30s --retries=3 \
    CMD curl --fail --silent --output /dev/null http://127.0.0.1:8000/openapi.yaml || exit 1
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
CMD ["serve"]
