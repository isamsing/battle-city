# Build matchbox_server
FROM rust:1-slim-bookworm AS builder
RUN cargo install matchbox_server@0.14.0

# Runtime — signaling server + TURN relay
FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends coturn && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/matchbox_server /usr/local/bin/matchbox_server
COPY turnserver.conf /etc/turnserver.conf
COPY start.sh /usr/local/bin/start.sh
RUN chmod +x /usr/local/bin/start.sh

EXPOSE 3536 3478/udp 3478/tcp

CMD ["/usr/local/bin/start.sh"]
