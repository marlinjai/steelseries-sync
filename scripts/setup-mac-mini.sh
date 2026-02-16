#!/bin/bash
set -e

# ============================================================
# SteelSeries Sync — Mac Mini Setup
# Sets up: Node.js, pm2, Cloudflare Tunnel, auto-deploy on push
# Modeled after: lola-stories/scripts/setup-mac-mini.sh
# ============================================================

REPO_URL="https://github.com/marlinjai/steelseries-sync.git"
INSTALL_DIR="$HOME/steelseries-sync"
SERVER_DIR="$INSTALL_DIR/server"
TUNNEL_NAME="steelseries-sync"
APP_PORT=3001
DOMAIN="sync.marlinjai.com"

echo "========================================="
echo "  SteelSeries Sync — Mac Mini Setup"
echo "========================================="

# -- 1. Homebrew --
if ! command -v brew &>/dev/null; then
  echo "[1/7] Installing Homebrew..."
  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
  # Add brew to PATH for Apple Silicon Macs
  if [ -f /opt/homebrew/bin/brew ]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
    echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> "$HOME/.zprofile"
  fi
else
  echo "[1/7] Homebrew already installed, skipping."
fi

# -- 2. Node.js --
if ! command -v node &>/dev/null; then
  echo "[2/7] Installing Node.js..."
  brew install node
else
  echo "[2/7] Node.js already installed ($(node -v)), skipping."
fi

# -- 3. pm2 --
if ! command -v pm2 &>/dev/null; then
  echo "[3/7] Installing pm2..."
  npm install -g pm2
else
  echo "[3/7] pm2 already installed, skipping."
fi

# -- 4. cloudflared --
if ! command -v cloudflared &>/dev/null; then
  echo "[4/7] Installing cloudflared..."
  brew install cloudflared
else
  echo "[4/7] cloudflared already installed, skipping."
fi

# -- 5. Clone/pull repo and build --
echo "[5/7] Setting up the sync server..."
if [ -d "$INSTALL_DIR" ]; then
  echo "  Repo exists, pulling latest..."
  cd "$INSTALL_DIR"
  git pull
else
  echo "  Cloning repo..."
  git clone "$REPO_URL" "$INSTALL_DIR"
  cd "$INSTALL_DIR"
fi

echo "  Installing server dependencies..."
cd "$SERVER_DIR"
npm install

echo "  Building NestJS server..."
npm run build

# Create data directory for user configs and auth store
mkdir -p "$SERVER_DIR/data"

# -- 6. Cloudflare Tunnel --
echo "[6/7] Setting up Cloudflare Tunnel..."
echo ""
echo "========================================="
echo "  INTERACTIVE STEP"
echo "  This will open a browser to authenticate"
echo "  with your Cloudflare account."
echo "========================================="
echo ""

# Login to Cloudflare (opens browser - you'll need to approve)
if [ ! -f "$HOME/.cloudflared/cert.pem" ]; then
  echo "  Logging into Cloudflare..."
  cloudflared tunnel login
else
  echo "  Already logged into Cloudflare, skipping."
fi

# Create the tunnel (skip if it already exists)
if ! cloudflared tunnel list | grep -q "$TUNNEL_NAME"; then
  echo "  Creating tunnel '$TUNNEL_NAME'..."
  cloudflared tunnel create "$TUNNEL_NAME"
else
  echo "  Tunnel '$TUNNEL_NAME' already exists, skipping."
fi

# Get the tunnel UUID
TUNNEL_ID=$(cloudflared tunnel list | grep "$TUNNEL_NAME" | awk '{print $1}')
echo "  Tunnel ID: $TUNNEL_ID"

# Write tunnel config
mkdir -p "$HOME/.cloudflared"
cat > "$HOME/.cloudflared/config-steelseries.yml" << EOF
tunnel: $TUNNEL_ID
credentials-file: $HOME/.cloudflared/$TUNNEL_ID.json

ingress:
  - hostname: $DOMAIN
    service: http://localhost:$APP_PORT
  - service: http_status:404
EOF

# Route DNS
echo "  Routing DNS for $DOMAIN..."
cloudflared tunnel route dns "$TUNNEL_NAME" "$DOMAIN"
echo "  DNS routed! A CNAME record has been created."

# -- Start everything with pm2 --
echo ""
echo "Starting services with pm2..."

# Stop existing instances if any
pm2 delete steelseries-sync 2>/dev/null || true
pm2 delete cloudflared-steelseries 2>/dev/null || true

# Generate a JWT secret if not already set
JWT_SECRET_FILE="$SERVER_DIR/data/.jwt_secret"
if [ ! -f "$JWT_SECRET_FILE" ]; then
  openssl rand -hex 32 > "$JWT_SECRET_FILE"
  echo "  Generated JWT secret."
fi
JWT_SECRET=$(cat "$JWT_SECRET_FILE")

# Start NestJS server
cd "$SERVER_DIR"
pm2 start npm --name "steelseries-sync" -- run start:prod \
  --env "PORT=$APP_PORT" \
  --env "DATA_DIR=$SERVER_DIR/data" \
  --env "JWT_SECRET=$JWT_SECRET"

# Start Cloudflare Tunnel
pm2 start cloudflared --name "cloudflared-steelseries" -- tunnel --config "$HOME/.cloudflared/config-steelseries.yml" run "$TUNNEL_NAME"

# Save pm2 config and set up auto-start on boot
pm2 save

echo ""
echo "Setting up pm2 to start on boot..."
echo "  Run the command below if prompted:"
echo ""
pm2 startup

# -- 7. Auto-deploy on push to main --
echo ""
echo "[7/7] Setting up auto-deploy..."

DEPLOY_SCRIPT="$HOME/autodeploy-steelseries.sh"
cat > "$DEPLOY_SCRIPT" << 'DEPLOY_EOF'
#!/bin/bash
export PATH="/opt/homebrew/bin:$PATH"
# Auto-deploy: checks if main has new commits and rebuilds
REPO_DIR="$HOME/steelseries-sync"
SERVER_DIR="$REPO_DIR/server"
LOG="$HOME/autodeploy-steelseries.log"

cd "$REPO_DIR" || exit 1

git fetch origin main --quiet

LOCAL=$(git rev-parse HEAD)
REMOTE=$(git rev-parse origin/main)

if [ "$LOCAL" != "$REMOTE" ]; then
  echo "$(date): New commits detected, deploying..." >> "$LOG"
  git pull origin main >> "$LOG" 2>&1
  cd "$SERVER_DIR"
  npm install >> "$LOG" 2>&1
  npm run build >> "$LOG" 2>&1
  pm2 restart steelseries-sync >> "$LOG" 2>&1
  echo "$(date): Deploy complete." >> "$LOG"
fi
DEPLOY_EOF
chmod +x "$DEPLOY_SCRIPT"

# Add cron job (every minute) if not already present
CRON_LINE="* * * * * $DEPLOY_SCRIPT"
(crontab -l 2>/dev/null | grep -v "autodeploy-steelseries" ; echo "$CRON_LINE") | crontab -
echo "  Auto-deploy cron job installed (checks every minute)."
echo "  Log: ~/autodeploy-steelseries.log"

echo ""
echo "========================================="
echo "  Setup complete!"
echo "========================================="
echo ""
echo "  Sync API:     http://localhost:$APP_PORT"
echo "  Public URL:   https://$DOMAIN"
echo ""
echo "  Auto-deploy: Push/merge to main and the"
echo "  Mac Mini will pick it up within 1 minute."
echo ""
echo "  Useful commands:"
echo "    pm2 status                    - check running services"
echo "    pm2 logs                      - view all logs"
echo "    pm2 logs steelseries-sync     - view server logs"
echo "    pm2 restart all               - restart everything"
echo "    tail -f ~/autodeploy-steelseries.log  - watch deploys"
echo ""
