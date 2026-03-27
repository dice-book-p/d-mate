# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

D-Mate (다온플레이스 업무 도우미) — a Tauri 2 desktop app for task notifications, real-time messaging, and email monitoring. Connects to SWORK (swork.kr) for task management, Desk server for MQTT messaging, and POP3 for email alerts. Targets macOS 10.15+ and Windows.

## Tech Stack

- **Frontend:** Svelte 5 + Vite 6 (ES modules)
- **Backend:** Rust 2021 edition with Tauri 2
- **Database:** SQLite (rusqlite, bundled, WAL mode) at `~/.local/share/d-mate/`
- **Credentials:** OS keyring (macOS Keychain / Windows Credential Manager)
- **Real-time:** MQTT via rumqttc
- **Encryption:** X25519 key exchange + AES-GCM (E2E for DM)

## Build & Dev Commands

```bash
# Development (frontend + backend hot-reload)
npm run tauri dev

# Build production app
npm run tauri build

# Frontend only
npm run dev          # Vite dev server on :1420
npm run build        # Vite build to ./build/

# Rust checks (from src-tauri/)
cd src-tauri && cargo check
cd src-tauri && cargo clippy
cd src-tauri && cargo build
```

No test framework is configured. Verify changes with `cargo check` / `cargo clippy` and manual testing.

## Architecture

### IPC Bridge

Frontend calls backend via Tauri `invoke()`. All commands are defined in `src-tauri/src/commands.rs` (60+ functions) and registered in `src-tauri/src/lib.rs`. Frontend wrappers live in `src/lib/api.js`.

Backend-to-frontend communication uses Tauri event emission (`AppHandle::emit()`), listened via `@tauri-apps/api/event`.

### Frontend (src/)

- **Routing:** Manual page switching via `currentPage` Svelte store (no router library)
- **State:** Svelte writable stores in `src/lib/stores.js`
- **Pages:** `src/pages/` — Dashboard, ConnectionManager, WorkerAlerts, ManagerAlerts, MailAlerts, MessagePage, ChatPage, SystemPage, FeedbackPage
- **Components:** `src/components/` — Sidebar, Toast, Toggle, ConfirmDialog, ConnBanner, etc.

### Backend (src-tauri/src/)

| Module | Purpose |
|--------|---------|
| `lib.rs` | App setup, tray icon, plugin registration, startup init |
| `commands.rs` | All Tauri IPC command handlers |
| `models.rs` | Shared data types (Task, Settings, etc.) |
| `database.rs` | SQLite schema, CRUD, settings persistence |
| `scheduler.rs` | 5 parallel background loops (overdue, deadline, approval, task, mail) |
| `checker.rs` | Task/mail check logic called by scheduler |
| `swork_client.rs` | SWORK API HTTP client (login, task fetch) |
| `desk_client.rs` | Desk server API client (join, health, key exchange) |
| `mqtt_client.rs` | MQTT connection, pub/sub, presence |
| `mail_checker.rs` | POP3 email polling |
| `crypto.rs` | E2E encryption (X25519 + AES-GCM) |
| `keyring_store.rs` | OS keyring read/write for credentials |
| `notification_rules.rs` | Notification dedup by slot_key |
| `native_notify.rs` | OS-level notifications |
| `telegram.rs` | Telegram bot API integration |
| `alert_hub.rs` | Active alert state tracking |

### Security

- CSP in `tauri.conf.json` restricts external connections to `swork.kr` and `api.telegram.org`
- Capabilities defined in `src-tauri/capabilities/default.json`
- Credentials never stored in plaintext — always OS keyring
- Update signatures verified via tauri-plugin-updater

## CI/CD

GitHub Actions (`.github/workflows/build.yml`): triggered by `v*` tags or manual dispatch. Builds for macOS (aarch64 + x86_64) and Windows (x86_64). Creates GitHub Releases with signed update artifacts.

## Conventions

- Korean comments and commit messages
- Commit format: `feat:`, `fix:` prefix with Korean description
- Tauri commands use snake_case, frontend API wrappers mirror the command names
- Settings stored in SQLite as JSON blobs, not in config files
