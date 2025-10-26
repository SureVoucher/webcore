# syntax=docker/dockerfile:1.6
FROM rust:1.82-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY config ./config
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
WORKDIR /app
COPY --from=builder /app/target/release/webcored /usr/local/bin/webcored
USER nonroot:nonroot
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/webcored"]