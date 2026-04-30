FROM rust:1-slim-bookworm AS builder
RUN cargo install matchbox_server@0.14.0

FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/matchbox_server /usr/local/bin/matchbox_server
EXPOSE 3536
ENV RUST_LOG=matchbox_server=info
ENTRYPOINT ["matchbox_server"]
