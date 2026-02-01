#!/bin/bash
# Kai Chess Engine - Lichess Bot Runner
# Runs the Lichess bot with the Kai engine

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Load .env file if it exists
if [ -f ".env" ]; then
    export $(grep -v '^#' .env | xargs)
fi

# Check for token
if [ -z "$LICHESS_BOT_TOKEN" ]; then
    echo "Error: LICHESS_BOT_TOKEN not set"
    echo
    echo "Either:"
    echo "  1. Add it to .env file: LICHESS_BOT_TOKEN=your_token"
    echo "  2. Export it: export LICHESS_BOT_TOKEN='your_token'"
    echo
    echo "Get a token at: https://lichess.org/account/oauth/token/create"
    echo "(Select 'bot:play' scope)"
    exit 1
fi

# Check if setup has been run
if [ ! -d "lichess-bot-repo" ]; then
    echo "Error: lichess-bot not found. Run ./setup.sh first"
    exit 1
fi

if [ ! -d "venv" ]; then
    echo "Error: Virtual environment not found. Run ./setup.sh first"
    exit 1
fi

# Check if engine exists
if [ ! -f "../target/release/kai" ]; then
    echo "Error: Kai engine not found. Run ./setup.sh first"
    exit 1
fi

# Activate virtual environment
source venv/bin/activate

# Copy config to lichess-bot directory
cp config.yml lichess-bot-repo/config.yml

# Run the bot
echo "Starting Kai on Lichess..."
echo "Press Ctrl+C to stop"
echo
cd lichess-bot-repo
python lichess-bot.py
