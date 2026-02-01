# Kai Chess Engine - Lichess Bot Docker Image
#
# Build: docker build -t kai-lichess-bot -f lichess-bot/Dockerfile .
# Run:   docker run -e LICHESS_BOT_TOKEN='your_token' kai-lichess-bot

# Stage 1: Build Kai engine
FROM rust:1.75-slim-bullseye AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Copy source files
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build release binary
RUN cargo build --release

# Stage 2: Runtime
FROM python:3.11-slim-bullseye

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    git \
    && rm -rf /var/lib/apt/lists/*

# Clone lichess-bot
RUN git clone --depth 1 https://github.com/lichess-bot-devs/lichess-bot.git /lichess-bot

WORKDIR /lichess-bot

# Install Python dependencies
RUN pip install --no-cache-dir -r requirements.txt

# Create engines directory
RUN mkdir -p /lichess-bot/engines

# Copy Kai binary from builder
COPY --from=builder /build/target/release/kai /lichess-bot/engines/kai
RUN chmod +x /lichess-bot/engines/kai

# Copy config template (token via env var)
COPY lichess-bot/config.yml.example /lichess-bot/config.yml

# Modify config to use Docker paths
RUN sed -i 's|dir: "../target/release"|dir: "./engines"|g' /lichess-bot/config.yml

# Token must be passed via environment variable
ENV LICHESS_BOT_TOKEN=""

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD pgrep -f "lichess_bot.py" || exit 1

# Run the bot
CMD ["python", "lichess_bot.py"]
