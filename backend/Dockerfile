# Build multi-stage para o backend Rust.
# Stage 1 (builder): usar rust:latest, copiar source, cargo build --release.
#   Nota: a feature bundled-sqlcipher compila o SQLCipher dentro do binário,
#   então não é necessário instalar o SQLCipher como dependência do sistema.
# Stage 2 (runtime): usar gcr.io/distroless/cc-debian12 ou debian:bookworm-slim,
# copiar binário do builder, expor porta 3000, CMD para rodar o servidor.
# A variável SQLCIPHER_KEY deve ser passada via env (ver compose.yml).
