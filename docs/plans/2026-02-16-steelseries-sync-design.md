# SteelSeries Sync — Design Document

**Date:** 2026-02-16
**Status:** Approved

## Problem

SteelSeries removed CloudSync from GG with no replacement and no timeline for return. Power users with complex macro setups across multiple machines have no way to sync their configurations. The only workaround is manually copying database files — error-prone and unsustainable.

## Product

An open-source desktop app that syncs SteelSeries GG configuration files across machines. Ships with a GUI (system tray + settings window) and supports multiple sync backends.

**Target audience:** SteelSeries power users with complex macro setups across multiple PCs (gamers, streamers, professionals).

**Business model:** Free and open source. Optional paid hosted sync tier for users without their own cloud storage.

## Architecture

```
+-----------------------------------------------------+
|                    Tauri Desktop App                  |
|                                                      |
|  +----------+   +-------------+   +------------+    |
|  | Tray UI  |   | Settings UI |   | Backup UI  |    |
|  +----+-----+   +------+------+   +-----+------+    |
|       +-----------+----+-----------------+           |
|                   v                                  |
|           +-----------------+                        |
|           |   Sync Engine   |                        |
|           | +-------------+ |                        |
|           | | File Watcher| |                        |
|           | +-------------+ |                        |
|           | | Debouncer   | |  (3s quiet period)     |
|           | +-------------+ |                        |
|           | | Safety Guard| |  (GG process check)    |
|           | +-------------+ |                        |
|           | | Backup Mgr  | |  (timestamped copies)  |
|           | +-------------+ |                        |
|           +--------+--------+                        |
|                    v                                 |
|        +---------------------+                       |
|        | Provider Adapter IF |                       |
|        +--+------+------+---+                        |
|           v      v      v                            |
|        +----+ +----+ +--------+                      |
|        |Drop| |One | |Hosted  |                      |
|        |box | |Drv | |API     |                      |
|        +----+ +----+ +--------+                      |
+------------------------------------------------------+
```

### Core Components

- **File Watcher** — monitors SteelSeries GG config directory for changes to `database.db*` files
- **Debouncer** — waits for writes to settle (3 seconds of no changes) before triggering sync
- **Safety Guard** — checks file locks and GG process state before reading or overwriting config files
- **Backup Manager** — creates timestamped backup before every overwrite (e.g. `database.db.2026-02-16T14-30-00.bak`)
- **Provider Adapter Interface** — trait that each sync backend implements: `push(files)`, `pull() -> files`, `status() -> SyncState`

### Config File Locations

**Windows:**
```
C:\ProgramData\SteelSeries\SteelSeries Engine 3\db\database.db
C:\ProgramData\SteelSeries\SteelSeries Engine 3\db\database.db-shm
C:\ProgramData\SteelSeries\SteelSeries Engine 3\db\database.db-wal
```

## Data Flow

### Outbound Sync (local change detected)

1. GG writes to `database.db`
2. File Watcher detects change
3. Debouncer waits 3s for writes to settle
4. Safety Guard confirms GG isn't mid-write
5. Backup Manager saves timestamped copy locally
6. Provider Adapter pushes files to sync destination
7. Tray icon shows "Synced" with timestamp

### Inbound Sync (remote change detected)

1. Provider Adapter detects newer remote files (polling or FS event from cloud folder)
2. Safety Guard checks GG isn't running, or prompts user to close it
3. Backup Manager saves current local config as backup
4. Files are pulled and written to SteelSeries config directory
5. Tray notification: "Config updated from [Device Name]"
6. User restarts GG (or app restarts it automatically)

### Conflict Resolution

- **Strategy:** Last-write-wins by timestamp
- Losing version is always saved as a backup with a descriptive label (e.g. `conflict-backup-desktop-2026-02-16T14-30-00`)
- Tray notification informs user that a conflict was resolved and backup was saved

## Hosted Sync Tier

For users who don't have their own cloud storage:

- Lightweight HTTP API running on a Mac Mini behind a **Cloudflare Tunnel** (automatic HTTPS, no cert management)
- Process managed by **PM2** (same pattern as the Lola Stories project)
- Auto-deploy via cron job checking for new commits on `main`

```
App --HTTPS--> Cloudflare Tunnel --HTTP (local)--> Mac Mini API (PM2 managed)
                                                    |-- /data/users/{id}/database.db
                                                    |-- /data/users/{id}/database.db-shm
                                                    |-- /data/users/{id}/database.db-wal
```

### API Endpoints

- `PUT /sync/{user_id}` — upload config files
- `GET /sync/{user_id}` — pull config files
- `GET /sync/{user_id}/meta` — get timestamps/metadata

### Authentication

Magic link or simple email + password. Low friction for a utility tool.

## Error Handling

| Scenario | Behavior |
|----------|----------|
| GG is writing mid-sync | Debouncer waits for writes to settle. Safety Guard checks file locks. |
| GG is running during inbound sync | Prompt user to close GG, or queue sync for next GG restart. |
| Cloud folder unavailable | Retry with backoff. Tray icon shows "offline" state. Syncs on reconnect. |
| Hosted API unreachable | Same retry logic. Local config is never blocked by network issues. |
| Corrupt database file pulled | Validate file size > 0 and basic SQLite header before overwriting. Reject bad files. |
| Backup folder growing too large | Configurable retention (default: last 20 backups or 30 days). Oldest pruned automatically. |
| SteelSeries changes config path | Config path is user-configurable in settings. Sensible defaults shipped. |

## Tech Stack

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| Desktop app | Tauri (Rust + WebView) | Lightweight (~5MB), native performance, built-in system tray |
| Frontend UI | React + TypeScript | Settings window, backup browser |
| Sync engine | Rust (Tauri backend) | File watching (notify crate), file ops, process detection |
| Hosted API | NestJS | Consistent with existing Mac Mini stack (Lola Stories) |
| Process mgmt | PM2 + Cloudflare Tunnel | Proven pattern |
| Auth | Magic link or email + password | Low friction |

## Project Structure

```
steelseries-sync/
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── main.rs
│   │   ├── watcher.rs          # File watcher + debouncer
│   │   ├── sync_engine.rs      # Core sync logic
│   │   ├── backup.rs           # Backup manager
│   │   ├── providers/
│   │   │   ├── mod.rs          # Provider trait
│   │   │   ├── folder.rs       # Cloud folder adapter
│   │   │   └── hosted.rs       # Hosted API adapter
│   │   └── safety.rs           # GG process detection, file lock checks
│   └── Cargo.toml
├── src/                        # React frontend
│   ├── App.tsx
│   ├── pages/
│   │   ├── Settings.tsx
│   │   ├── BackupBrowser.tsx
│   │   └── Status.tsx
│   └── components/
├── server/                     # Hosted sync API
│   ├── src/
│   │   ├── main.ts
│   │   ├── auth/
│   │   └── sync/
│   └── package.json
├── scripts/
│   └── setup-mac-mini.sh       # PM2 + Cloudflare Tunnel setup
└── package.json
```

## Design Decisions

- **File-level sync only** — treat `database.db` as an opaque blob. No parsing of SteelSeries schema. This makes the tool resilient to GG updates.
- **Adapter pattern for providers** — adding new sync backends (Syncthing, Google Drive, etc.) is just implementing a trait.
- **Last-write-wins with backups** — simple conflict resolution with full recoverability.
- **Cloudflare Tunnel for hosted tier** — zero port forwarding, automatic HTTPS, free tier sufficient.
