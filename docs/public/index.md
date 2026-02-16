---
title: SteelSeries Sync
description: Sync your SteelSeries GG configurations across machines
order: 0
---

# SteelSeries Sync

An open-source desktop app that syncs your SteelSeries GG configuration files across multiple machines. Built because SteelSeries disabled CloudSync with no replacement.

## How It Works

1. **Watches** your local SteelSeries GG config directory for changes
2. **Pushes** updated config files to a sync server (or shared folder)
3. **Pulls** config from the server to other machines
4. **Backs up** your config before every sync operation

## Features

- Automatic file watching with debounced push
- Hosted sync via API (self-hosted on your own server)
- Folder-based sync as an alternative (Dropbox, iCloud, etc.)
- Last-write-wins conflict resolution with timestamped backups
- SQLite header validation before overwriting config
- System tray integration
- Dark theme gaming UI

## Supported Platforms

- macOS
- Windows (planned)
- Linux (planned)
