# Changelog

All notable changes to Transmission Remote Slint are documented in this file.

## [v0.5] — 2026-03-31

### 🌐 Internationalization
- Support for 5 languages: English, Deutsch, Русский, 中文, Español
- Language settings via Settings Dialog (⚙ button in toolbar)
- Translations based on Transmission CLI/Desktop
- Language preference persisted between sessions

### 🖥️ UI Changes
- Toolbar: ➕ 🧲 📀 🔄 buttons on left, search centered, settings ⚙ on right
- Left panel: status filters (All, Downloading, Seeding, Completed, Stopped, Active, Inactive, Checking, Error)
- Rehash button for torrents with hash errors

### 🔧 System Tray
- Resume All / Pause All buttons for batch torrent control
- Tray menu translated to all supported languages

### 🛠️ Improvements
- Magnet links via input dialog + xdg-open
- .torrent file picker + xdg-open integration
- Fixed double-Cancel bug in CreateTorrentDialog

### 📁 Technical Details
- i18n via simple dictionary in src/i18n.rs
- ~100 lines of UI translations + torrent statuses
- No external i18n dependencies (rust-i18n deferred for future release)

---
**Note:** First release fully developed with **Qwen 3.5 Plus** (previously Claude Sonnet 4.6)
