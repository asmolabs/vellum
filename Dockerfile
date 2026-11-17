# Stage 1: Build headless server binary (vellum-cli)
FROM rust:1.86-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

RUN cargo build --release -p vellum-cli

# Stage 2: Minimal runtime image (no GUI / GTK / WebKit dependencies)
FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl tini && \
    rm -rf /var/lib/apt/lists/* && \
    groupadd --gid 1000 vellum && \
    useradd --uid 1000 --gid 1000 --create-home --shell /usr/sbin/nologin vellum && \
    mkdir -p /data /books && \
    chown -R vellum:vellum /data /books

COPY --from=builder /app/target/release/vellum-cli /usr/local/bin/vellum

USER vellum:vellum
WORKDIR /data

ENV VELLUM_DATA_DIR=/data
ENV VELLUM_LIBRARY_DIR=/books
ENV VELLUM_OPDS_PORT=8080
ENV RUST_LOG=info

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
  CMD curl -fsS "http://127.0.0.1:${VELLUM_OPDS_PORT:-8080}/api/v1/health" >/dev/null || exit 1

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/vellum"]
CMD ["serve", "--host", "0.0.0.0", "--port", "8080"]
