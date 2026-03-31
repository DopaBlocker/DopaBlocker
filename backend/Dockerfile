# Build multi-stage para o backend Rust.
# Stage 1 (builder): usar rust:latest, copiar source, cargo build --release.
# Stage 2 (runtime): usar gcr.io/distroless/cc-debian12 ou debian:bookworm-slim,
# copiar binário do builder, expor porta 3000, CMD para rodar o servidor.
