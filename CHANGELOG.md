# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-02-16

### Added

- Desktop app with Tauri 2 (Rust + React TypeScript)
- File watcher with 3-second debounce for automatic push on local changes
- Hosted sync provider (API with JWT auth, multipart file upload)
- Folder sync provider (shared folder like Dropbox/iCloud)
- Sync engine with last-write-wins conflict resolution
- Timestamped backup system with configurable retention
- SQLite header validation before overwriting config files
- Safety guard: GG process detection, file lock checks
- Pull suppression to prevent watcher feedback loops
- Smart polling: only auto-pulls when remote is newer, never auto-pushes
- Persistent config stored at `~/Library/Application Support/steelseries-sync/config.json`
- Base64 decoding for hosted provider pull responses
- Dark theme gaming UI with Status, Settings, and Backups tabs
- System tray integration (Sync Now, Open Window, View Backups, Quit)
- NestJS sync server with register/login (JWT), upload/download/meta endpoints
- PM2 deployment script for Mac Mini with Cloudflare Tunnel
- Clearify documentation site
