# Build matchbox_server
FROM rust:1-slim-bookworm AS builder
RUN cargo install matchbox_server@0.14.0

# Runtime — just the signaling server
FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/matchbox_server /usr/local/bin/matchbox_server
EXPOSE 3536
CMD ["matchbox_server"]
