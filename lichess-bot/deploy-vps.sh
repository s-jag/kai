#!/bin/bash
# Kai Chess Engine - VPS Deployment Script
# Run this on your Hostinger VPS to deploy the Lichess bot
#
# Prerequisites:
#   - Ubuntu/Debian VPS
#   - SSH access
#   - Your Lichess API token
#
# Usage:
#   1. SSH into your VPS
#   2. Run: curl -sSL https://raw.githubusercontent.com/s-jag/kai/main/lichess-bot/deploy-vps.sh | bash
#   Or clone and run locally

set -e

echo "=== Kai Lichess Bot VPS Deployment ==="
echo

# Check if running as root
if [ "$EUID" -eq 0 ]; then
    echo "Please don't run as root. Run as a regular user with sudo access."
    exit 1
fi

# Update system
echo "Updating system packages..."
sudo apt-get update
sudo apt-get upgrade -y

# Install dependencies
echo "Installing dependencies..."
sudo apt-get install -y \
    build-essential \
    curl \
    git \
    python3 \
    python3-pip \
    python3-venv

# Install Rust
if ! command -v cargo &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "Rust already installed."
fi

# Create installation directory
INSTALL_DIR="$HOME/kai"
echo "Installing to $INSTALL_DIR..."

if [ -d "$INSTALL_DIR" ]; then
    echo "Updating existing installation..."
    cd "$INSTALL_DIR"
    git pull
else
    echo "Cloning Kai repository..."
    git clone https://github.com/s-jag/kai.git "$INSTALL_DIR"
    cd "$INSTALL_DIR"
fi

# Build Kai engine
echo "Building Kai engine (this may take a few minutes)..."
source "$HOME/.cargo/env"
cargo build --release

# Setup lichess-bot
cd lichess-bot
echo "Setting up lichess-bot..."

# Clone lichess-bot if not present
if [ ! -d "lichess-bot-repo" ]; then
    git clone --depth 1 https://github.com/lichess-bot-devs/lichess-bot.git lichess-bot-repo
fi

# Create virtual environment
python3 -m venv venv
source venv/bin/activate

# Install Python dependencies
pip install --upgrade pip
pip install -r lichess-bot-repo/requirements.txt

# Create config from template
if [ ! -f "config.yml" ]; then
    cp config.yml.example config.yml
fi

# Prompt for token if .env doesn't exist
if [ ! -f ".env" ]; then
    echo
    echo "Enter your Lichess API token (get one at https://lichess.org/account/oauth/token/create):"
    read -r TOKEN
    echo "LICHESS_BOT_TOKEN=$TOKEN" > .env
    chmod 600 .env
    echo "Token saved to .env"
fi

# Install systemd service
echo "Installing systemd service..."
sudo cp kai-bot.service /etc/systemd/system/

# Update service paths
sudo sed -i "s|/opt/kai|$INSTALL_DIR|g" /etc/systemd/system/kai-bot.service
sudo sed -i "s|User=kai|User=$USER|g" /etc/systemd/system/kai-bot.service
sudo sed -i "s|Group=kai|Group=$USER|g" /etc/systemd/system/kai-bot.service

# Reload and enable service
sudo systemctl daemon-reload
sudo systemctl enable kai-bot

echo
echo "=== Deployment Complete ==="
echo
echo "Commands:"
echo "  Start bot:    sudo systemctl start kai-bot"
echo "  Stop bot:     sudo systemctl stop kai-bot"
echo "  View status:  sudo systemctl status kai-bot"
echo "  View logs:    journalctl -u kai-bot -f"
echo
echo "To start the bot now, run:"
echo "  sudo systemctl start kai-bot"
