# Multi-stage build: templates/migrations are compiled into the binary by
# askama/sqlx at build time, so the runtime image only needs the binary
# itself plus static/ (served at runtime via tower_http::ServeDir).

FROM rust:1-bookworm AS builder
WORKDIR /build

# Cheap insurance in case any dependency resolves to an openssl-sys backend;
# harmless if the build ends up pure-rustls (which it currently does).
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY templates ./templates
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/optistor-server ./optistor-server
COPY static ./static

# SQLite file lives here; mount a persistent volume at this path in production.
ENV OPTISTOR_DATABASE_URL=sqlite:///data/optistor.db
VOLUME ["/data"]

EXPOSE 8000
CMD ["./optistor-server"]
