FROM --platform=linux/amd64 rust:bookworm as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency files and build dependencies (cached layer)
COPY Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

# Copy source code
COPY src ./src
COPY templates ./templates

# Build application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM --platform=linux/amd64 debian:bookworm-slim

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Install runtime dependencies including Pandoc
RUN apt-get update && apt-get install -y \
    ca-certificates \
    pandoc \
    texlive-xetex \
    texlive-fonts-recommended \
    texlive-fonts-extra \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/document-generation-service /app/
COPY --from=builder /app/templates /app/templates

# Verify Pandoc installation
RUN pandoc --version

# Create non-root user
RUN useradd -m -u 1001 docgen && \
    chown -R docgen:docgen /app

USER docgen

EXPOSE 8080

CMD ["./document-generation-service"]
