# SteelSeries Sync

> **[steelseries-sync.pages.dev](https://steelseries-sync.pages.dev)** | [Download DMG](https://github.com/marlinjai/steelseries-sync/releases/download/v0.1.0/steelseries-sync_0.1.0_aarch64.dmg) | [Documentation](https://steelseries-sync.pages.dev/docs/)

Sync your SteelSeries GG configurations across multiple machines. An open-source replacement for the disabled CloudSync feature.

## Features

- Automatic file watching with debounced push
- Hosted sync via self-hosted API server
- Folder-based sync (Dropbox, iCloud, etc.)
- Last-write-wins conflict resolution with timestamped backups
- SQLite header validation before overwriting config
- Dark theme gaming UI
- System tray integration

## Tech Stack

- **Desktop**: Tauri 2 (Rust + React TypeScript)
- **Server**: NestJS with JWT authentication
- **Deployment**: PM2 + Cloudflare Tunnel

## Install

1. Download the [latest DMG](https://github.com/marlinjai/steelseries-sync/releases/download/v0.1.0/steelseries-sync_0.1.0_aarch64.dmg)
2. Open the DMG and drag the app to Applications
3. On first launch, macOS will block it (the app is unsigned). To allow it:
   - Go to **System Settings → Privacy & Security** and click **Open Anyway**
   - Or run: `xattr -cr /Applications/steelseries-sync.app`
4. You only need to do this once

## Quick Start (Development)

```bash
git clone https://github.com/marlinjai/steelseries-sync.git
cd steelseries-sync
pnpm install
pnpm tauri dev
```

## Server Setup

```bash
cd server
pnpm install
cp .env.example .env  # Edit with your JWT_SECRET and DATA_DIR
pnpm start
```

## Configuration

Config is stored at `~/Library/Application Support/steelseries-sync/config.json` (macOS).

See the [Getting Started](docs/public/getting-started.md) guide for setup instructions.

## License

MIT
