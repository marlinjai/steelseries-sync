---
title: Architecture
description: Internal architecture and design decisions
order: 0
---

# Architecture

## Tech Stack

- **Desktop**: Tauri 2 (Rust backend + React TypeScript frontend)
- **Server**: NestJS with JWT auth, multipart file upload
- **Deployment**: PM2 + Cloudflare Tunnel on Mac Mini

## Data Flow

```
SteelSeries GG → database.db changes
    ↓ (file watcher, 3s debounce)
Sync Engine → push to provider
    ↓
Provider (Hosted API / Folder)
    ↓
Other machines poll every 30s → pull if remote newer
    ↓
Write to local config dir → restart GG to apply
```

## Key Design Decisions

- **File-level sync** (opaque blob) — we don't parse the SQLite schema, just copy the files
- **Last-write-wins** — timestamps decide push vs pull, with backup before every overwrite
- **Safety guard** — SQLite header validation before writing, backup before every pull
- **Pull suppression** — file watcher skips auto-push immediately after a pull to prevent feedback loops
- **Polling only pulls** — the 30s poll never auto-pushes; pushing is only triggered by the file watcher or explicit user action

## SteelSeries GG Config Location

| Platform | Path |
|----------|------|
| macOS | `/Library/Application Support/SteelSeries GG/apps/engine/data/db/` |
| Windows | `C:\ProgramData\SteelSeries\SteelSeries GG\apps\engine\data\db\` |
| Linux | `/etc/steelseries-engine-3/db/` |

Files: `database.db`, `database.db-shm`, `database.db-wal`
