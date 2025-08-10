# backend/Dockerfile
FROM rust:1.79 as builder
WORKDIR /app

# Copy the entire project into the container
COPY . .

# First build
RUN cargo build to cache dependencies -p backend --release || true

# Rebuild (ensures changes with actual sources are compiled)
RUN cargo build -p backend --release

# --- Runtime stage ---
FROM debian:bookworm-slim

# Install required system libs for Rust binary
RUN apt-get update && \
    apt-get install -y ca-certificates libstdc++6 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/backend /app/backend

# Expose the app port
EXPOSE 3000

# Default command
CMD ["/app/backend"]