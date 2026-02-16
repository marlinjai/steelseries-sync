---
title: Getting Started
description: How to set up SteelSeries Sync
order: 1
---

# Getting Started

## Prerequisites

- SteelSeries GG installed on your machine
- A sync server running (or a shared folder like Dropbox/iCloud)

## Installation

Download the latest `.dmg` from the releases page, or build from source:

```bash
git clone https://github.com/marlinjai/steelseries-sync.git
cd steelseries-sync
pnpm install
pnpm tauri build
```

## Configuration

The app stores its config at `~/Library/Application Support/steelseries-sync/config.json` (macOS).

### Setting up Hosted Sync via CLI

Write the config file directly:

```bash
mkdir -p ~/Library/Application\ Support/steelseries-sync
cat > ~/Library/Application\ Support/steelseries-sync/config.json << 'EOF'
{
  "steelseries_db_path": "/Library/Application Support/SteelSeries GG/apps/engine/data/db",
  "backup_dir": "~/Library/Application Support/steelseries-sync/backups",
  "max_backups": 20,
  "debounce_secs": 3,
  "provider": {
    "type": "Hosted",
    "api_url": "https://your-sync-server.example.com",
    "api_key": "your-jwt-token-here"
  },
  "device_name": "my-machine"
}
EOF
```

### Getting an API Key

Register on the sync server:

```bash
curl -X POST https://your-sync-server.example.com/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"email":"you@example.com","password":"your-password"}'
```

The response contains your `access_token` (JWT) — use this as the `api_key` in the config.

### Setting up Folder Sync

Use the default Folder provider with a shared folder:

```json
{
  "provider": {
    "type": "Folder",
    "sync_dir": "/path/to/shared/folder"
  }
}
```

## Usage

- **Push**: Upload your local config to the server
- **Pull**: Download config from the server (restart GG to apply)
- **Sync Now**: Compare timestamps and push or pull as needed

The app automatically watches for local changes and pushes them. Remote changes are polled every 30 seconds.
