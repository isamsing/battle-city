# Stage 1: Build matchbox_server
FROM rust:1-slim-bookworm AS matchbox-builder
RUN cargo install matchbox_server@0.14.0

# Stage 2: Build WASM game with trunk
FROM rust:1-slim-bookworm AS wasm-builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY assets ./assets
COPY index.html Trunk.toml ./

RUN trunk build --release

# Stage 3: Runtime
FROM nginx:stable-bookworm

COPY --from=matchbox-builder /usr/local/cargo/bin/matchbox_server /usr/local/bin/matchbox_server
COPY --from=wasm-builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/nginx.conf
COPY entrypoint.sh /entrypoint.sh

EXPOSE 8080
ENTRYPOINT ["/entrypoint.sh"]
