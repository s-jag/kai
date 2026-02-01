# Kai Lichess Bot

Run Kai as a bot on [Lichess](https://lichess.org) so anyone can play against it!

## Prerequisites

- **Rust** 1.70+ (for building Kai)
- **Python** 3.8+ (for lichess-bot)
- **Git** (for cloning lichess-bot)
- A **Lichess BOT account** with API token

## Quick Start

### 1. Create a Lichess Bot Account

1. Create a **new** Lichess account (not your personal one)
2. Go to https://lichess.org/account/oauth/token/create
3. Create a token with the `bot:play` scope
4. Save the token securely

5. **Upgrade to BOT account** (this is irreversible!):
   ```bash
   curl -X POST https://lichess.org/api/bot/account/upgrade \
     -H "Authorization: Bearer YOUR_TOKEN_HERE"
   ```

### 2. Setup

```bash
# Run the setup script
cd lichess-bot
./setup.sh
```

This will:
- Build Kai in release mode
- Clone the lichess-bot repository
- Install Python dependencies
- Create config files from templates

### 3. Configure Token

**Option A: Using .env file (recommended)**
```bash
# Edit .env and add your token
echo "LICHESS_BOT_TOKEN=your_token_here" > .env
```

**Option B: Using environment variable**
```bash
export LICHESS_BOT_TOKEN='your_token_here'
```

### 4. Run the Bot

```bash
./run.sh
```

The bot will connect to Lichess and start accepting challenges!

## Docker Deployment

For server deployment, use Docker:

```bash
# Build the image (from kai root directory)
docker build -t kai-lichess-bot -f lichess-bot/Dockerfile .

# Run with token
docker run -d \
  --name kai-bot \
  -e LICHESS_BOT_TOKEN='your_token_here' \
  --restart unless-stopped \
  kai-lichess-bot
```

Or use Docker Compose:

```bash
# Set token in environment or .env file
export LICHESS_BOT_TOKEN='your_token_here'

# Start the bot
cd lichess-bot
docker-compose up -d

# View logs
docker-compose logs -f
```

## SystemD Deployment (Linux Servers)

For 24/7 operation on a Linux server:

### 1. Install Kai

```bash
# Create kai user
sudo useradd -r -s /bin/false kai

# Clone and build
sudo mkdir -p /opt/kai
sudo chown kai:kai /opt/kai
cd /opt/kai
git clone https://github.com/s-jag/kai.git .
cargo build --release

# Setup lichess-bot
cd lichess-bot
./setup.sh
```

### 2. Configure Token

```bash
# Create .env with your token
sudo -u kai bash -c 'echo "LICHESS_BOT_TOKEN=your_token" > /opt/kai/lichess-bot/.env'
sudo chmod 600 /opt/kai/lichess-bot/.env
```

### 3. Install Service

```bash
# Copy service file
sudo cp /opt/kai/lichess-bot/kai-bot.service /etc/systemd/system/

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable kai-bot
sudo systemctl start kai-bot

# Check status
sudo systemctl status kai-bot

# View logs
journalctl -u kai-bot -f
```

## Configuration

Edit `config.yml` to customize:

- **Challenge acceptance**: Which time controls, variants, rated/casual
- **Greeting messages**: What Kai says at start/end of games
- **Resign/draw behavior**: When to resign or offer draws
- **Matchmaking**: Auto-seek games (disabled by default)

See `config.yml.example` for all options.

## Files

| File | Description |
|------|-------------|
| `config.yml.example` | Configuration template (safe to commit) |
| `config.yml` | Your configuration (gitignored) |
| `.env.example` | Environment template (safe to commit) |
| `.env` | Your token (gitignored, NEVER commit!) |
| `setup.sh` | One-time setup script |
| `run.sh` | Start the bot |
| `Dockerfile` | Docker build configuration |
| `docker-compose.yml` | Docker Compose configuration |
| `kai-bot.service` | SystemD service file |

## Security

**NEVER commit your token!**

The token is kept secure by:
- `.env` file is gitignored
- Token passed via environment variable
- Docker uses `-e` flag or compose env
- SystemD uses protected `EnvironmentFile`

If you accidentally expose your token:
1. Immediately revoke it at https://lichess.org/account/oauth/token
2. Create a new token
3. Update your `.env` file

## Troubleshooting

### Bot not connecting
- Check your token is valid
- Ensure the account is upgraded to BOT
- Check internet connection

### Engine errors
- Run `cargo build --release` to rebuild Kai
- Check `../target/release/kai` exists and is executable

### Permission denied
- Run `chmod +x setup.sh run.sh`
- For systemd, check file ownership matches service user

### View detailed logs
```bash
# Local
./run.sh 2>&1 | tee bot.log

# Docker
docker logs -f kai-bot

# SystemD
journalctl -u kai-bot -f
```

## Links

- [Kai Chess Engine](https://github.com/s-jag/kai)
- [lichess-bot Documentation](https://github.com/lichess-bot-devs/lichess-bot/wiki)
- [Lichess API](https://lichess.org/api)
