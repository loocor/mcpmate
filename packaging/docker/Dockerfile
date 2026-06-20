ARG RUST_IMAGE=rust:1-bookworm
ARG RUNTIME_IMAGE=debian:bookworm-slim

FROM oven/bun:1 AS board-builder

WORKDIR /workspace/board
COPY board/package.json board/bun.lock ./
RUN bun install --frozen-lockfile
COPY board ./
RUN NODE_OPTIONS=--max-old-space-size=4096 bun run build

FROM ${RUST_IMAGE} AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends libdbus-1-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY backend ./backend

WORKDIR /workspace/backend
RUN cargo build --release --bin mcpmate --bin bridge

FROM ${RUNTIME_IMAGE} AS runtime

LABEL org.opencontainers.image.title="MCPMate"
LABEL org.opencontainers.image.description="Local MCP proxy and management server for AI clients"
LABEL org.opencontainers.image.source="https://github.com/loocor/MCPMate"
LABEL io.modelcontextprotocol.server.name="io.github.loocor/mcpmate"

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libdbus-1-3 nginx-light \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --home-dir /data --shell /usr/sbin/nologin mcpmate

COPY --from=builder /workspace/backend/target/release/mcpmate /usr/local/bin/mcpmate
COPY --from=builder /workspace/backend/target/release/bridge /usr/local/bin/bridge
COPY --from=board-builder /workspace/board/dist /opt/mcpmate/board
COPY packaging/docker/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh

ENV MCPMATE_API_PORT=8080
ENV MCPMATE_MCP_PORT=8000
ENV MCPMATE_DASHBOARD_PORT=3000
ENV MCPMATE_DATA_DIR=/data/.mcpmate
ENV MCPMATE_LOG=info
ENV MCPMATE_TRANSPORT=uni
ENV RUST_LOG=info

WORKDIR /data
RUN mkdir -p /data/.mcpmate /var/lib/nginx /var/log/nginx /tmp/nginx \
    && chown -R mcpmate:mcpmate /data /opt/mcpmate /var/lib/nginx /var/log/nginx /tmp/nginx \
    && chmod +x /usr/local/bin/docker-entrypoint.sh

USER mcpmate
EXPOSE 3000 8080 8000

ENTRYPOINT ["docker-entrypoint.sh"]
