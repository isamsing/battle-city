# Stage 1: Build matchbox_server
FROM rust:1-slim-bookworm AS matchbox-builder
RUN cargo install matchbox_server@0.14.0

# Stage 2: Runtime — nginx serves pre-built WASM + proxies WebSocket to matchbox
FROM nginx:stable-bookworm

COPY --from=matchbox-builder /usr/local/cargo/bin/matchbox_server /usr/local/bin/matchbox_server
COPY dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/nginx.conf
COPY entrypoint.sh /entrypoint.sh

EXPOSE 8080
ENTRYPOINT ["/entrypoint.sh"]
