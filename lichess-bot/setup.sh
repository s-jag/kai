#!/bin/bash
# Kai Chess Engine - Lichess Bot Setup
# Run this script once to set up the bot environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Kai Lichess Bot Setup ==="
echo

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo not found. Install from https://rustup.rs/"
    exit 1
fi

# Check for Python
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 not found. Please install Python 3.8+"
    exit 1
fi

# Build Kai engine
echo "Building Kai engine (release mode)..."
cd ..
cargo build --release
cd "$SCRIPT_DIR"

if [ ! -f "../target/release/kai" ]; then
    echo "Error: Build failed - kai binary not found"
    exit 1
fi
echo "Engine built successfully."
echo

# Clone lichess-bot if not present
if [ ! -d "lichess-bot-repo" ]; then
    echo "Cloning lichess-bot repository..."
    git clone --depth 1 https://github.com/lichess-bot-devs/lichess-bot.git lichess-bot-repo
else
    echo "lichess-bot-repo already exists, updating..."
    cd lichess-bot-repo
    git pull
    cd ..
fi
echo

# Create virtual environment
echo "Setting up Python virtual environment..."
python3 -m venv venv
source venv/bin/activate

# Install dependencies
echo "Installing Python dependencies..."
pip install --upgrade pip -q
pip install -r lichess-bot-repo/requirements.txt -q
echo

# Create config from template if not exists
if [ ! -f "config.yml" ]; then
    echo "Creating config.yml from template..."
    cp config.yml.example config.yml
fi

# Create .env from template if not exists
if [ ! -f ".env" ]; then
    echo "Creating .env from template..."
    cp .env.example .env
    echo
    echo "IMPORTANT: Edit .env and add your Lichess API token!"
fi

echo
echo "=== Setup Complete ==="
echo
echo "Next steps:"
echo "1. Edit .env and add your Lichess API token"
echo "   (Get one at: https://lichess.org/account/oauth/token/create)"
echo "2. Run the bot: ./run.sh"
echo
echo "Or use environment variable directly:"
echo "   export LICHESS_BOT_TOKEN='your_token_here'"
echo "   ./run.sh"
