# Transmission Remote — Slint

A lightweight native desktop GUI for **Transmission daemon** built with **Rust + Slint**.  
No GTK, no Qt — pure Rust rendering via Skia/OpenGL or Vulkan.

> **Developed with Qwen 3.6 Plus (Alibaba).**

**Languages:** English | [Русский](README.ru.md)

<img width="1064" height="736" alt="Снимок экрана_2026-04-01_16-26-36" src="https://github.com/user-attachments/assets/907ca892-76d8-4ab5-a862-06b5be5f188a" />


---

## UI Performance Comparison

GTK and Qt frontends share a well-known problem with large torrent libraries. Both render the torrent list on the **main UI thread** and rebuild the entire model on every poll cycle. The GTK 4 frontend is especially aggressive: it fires `gtk_list_store_clear()` + re-inserts all rows every few seconds, which causes the GTK main loop to stall completely.

Real-world reports confirm this:

- **GTK 4.1 with ~4,700 torrents** — a single click takes up to a minute; window artifacts appear on top of other applications. ([#8359](https://github.com/transmission/transmission/issues/8359))
- **Qt and GTK with 3,200+ torrents** — searching, opening, or altering a torrent can take all night to complete. ([#4193](https://github.com/transmission/transmission/issues/4193))

The Qt client behaves somewhat better in practice because Qt's `QAbstractItemModel` with `dataChanged` signals is more surgical — it can update individual cells without a full reset. However the underlying issue remains: all polling and model updates still happen on the main thread, and with thousands of active torrents firing rapid updates, the UI event loop gets saturated. Issue #4193 affecting both GTK and Qt was closed as a core regression, not fixed in the frontend.

**This project takes a different approach:**

```
┌──────────────────────────────────────────────────────────┐
│  Slint UI thread (event loop)                            │
│  MainWindow ◄── update_rx (torrents + stats)  50ms pump │
│             ◄── status_rx (status bar text)              │
│             ──► cmd_tx   (Command enum)                  │
└─────────────────────────┬────────────────────────────────┘
                          │  std::sync::mpsc
┌─────────────────────────▼────────────────────────────────┐
│  Tokio async runtime                                     │
│  backend_task: tokio::select!                            │
│    cmd_rx  → immediate RPC call                          │
│    interval tick → recently-active delta every 2s        │
│  TransmissionClient (reqwest, 409 session retry)         │
└──────────────────────────────────────────────────────────┘
```

- **Tokio async runtime** handles all network I/O in a separate thread — the UI never blocks on RPC calls
- **`recently-active` delta updates** — only torrents that changed in the last interval are fetched and pushed to the UI; the full list is never re-rendered unless explicitly requested
- **Slint virtual scrolling** — only visible rows are rendered, regardless of total library size
- The UI thread only receives a small diff via `mpsc` channel and applies it; it never touches the network

The result: the UI stays responsive at 1,000+ or 4,000+ torrents because the main thread simply never does the work that kills GTK and Qt at scale.

---

## Comparison

| Feature | **transmission-remote-slint** | transmission-remote-gtk | transmission-qt | Transmission GTK 4.x |
|---|---|---|---|---|
| Type | Remote only | Remote only | Standalone + Remote | Standalone |
| Toolkit | Slint (Rust) | GTK 3 | Qt 5/6 | GTK 4 |
| UI thread blocked on poll? | ✅ Never | ❌ Always | ⚠️ Partially | ❌ Always |
| Update strategy | `recently-active` delta | Full list rebuild | Partial via signals | Full list rebuild |
| Virtual scrolling | ✅ | ❌ | ❌ | ❌ |
| System tray | ✅ Works (SNI/D-Bus) | ✅ Works | ✅ Works | ⚠️ Broken in GTK 4¹ |
| License | GPL-2.0-or-later | GPL-2.0-or-later | GPL-2.0-or-later | GPL-2.0-or-later |

> ¹ GTK 4 dropped tray support. The fix is in development but not yet merged as of early 2026.

---

## Features

- **Torrent list** — name, status, progress, ↓/↑ speed, inline error messages
- **Per-torrent actions** — Start / Pause / Recheck / Open folder / Remove / Delete with files
- **Bulk actions** — Start All / Stop All with confirmation dialog
- **Status filters** — filter torrents by status (All, Downloading, Seeding, Completed, Stopped, Active, Inactive, Checking, Error)
- **Instant search** — filter by torrent name without waiting for RPC
- **System tray** — StatusNotifierItem via D-Bus (native zbus 4, no ksni/GTK)
  - Resume All / Pause All buttons
  - Translated menu items
- **Desktop notifications** — download complete, recheck done, torrent errors
- **Single instance** — second launch focuses the window or adds a `.torrent` file
- **Auto-detect Transmission** — reads daemon status, connects automatically
- **`.torrent` file handler** — open from file manager or pass as argument
- **Magnet link support** — input dialog + xdg-open
- **i18n** — 5 languages (EN/DE/RU/ZH/ES), configurable via Settings Dialog
- **App icon** — embedded in binary, installed to hicolor theme via PKGBUILD
- **Autostart** — optional `.desktop` entry in `~/.config/autostart/`
- **Render backend** — auto-selects Vulkan → OpenGL → Software

---

## Installation

### AUR (Arch Linux) — build from source

```bash
paru -S transmission-remote-slint
# or manually:
git clone https://aur.archlinux.org/transmission-remote-slint.git
cd transmission-remote-slint
makepkg -si
```

### AUR — prebuilt binary

```bash
paru -S transmission-remote-slint-bin
```

### Build from source

```bash
# Arch
sudo pacman -S rust base-devel libxcb libxkbcommon fontconfig freetype2

# Debian/Ubuntu
sudo apt install -y build-essential cargo pkg-config \
  libfontconfig1-dev libfreetype-dev \
  libxcb-shape0-dev libxcb-xfixes0-dev libxcb-render0-dev \
  libxkbcommon-dev

git clone https://github.com/guglovich/Transmission-Remote-Slint.git
cd Transmission-Remote-Slint
cargo build --release
./target/release/transmission-remote-slint
```

---

## Optional runtime dependencies

| Package | Purpose |
|---|---|
| `zenity` or `kdialog` | File picker dialogs |
| `libnotify` | Desktop notifications |
| `snixembed` | Tray support in XFCE / Openbox |
| `xfce4-statusnotifier-plugin` | Tray support in XFCE (alternative) |
| `xdotool` | Taskbar icon via `_NET_WM_ICON` |

---

## Configuration

Config file: `~/.config/transmission-gui/config.toml`  
Created automatically on first launch:

```toml
language = "en"                 # "en", "de", "ru", "zh", "es"
suspend_on_hide = false         # freeze process when minimized to tray
start_minimized = false         # start hidden in tray
refresh_interval_secs = 2       # poll interval
delete_torrent_after_add = true # delete .torrent file after adding (like Transmission GTK)
autostart = false
```

Transmission connection is auto-detected from daemon status.

---

## Command-line options

```
transmission-remote-slint [FILE.torrent] [--gl|--vk|--sw|--wl]

--gl    Force OpenGL renderer
--vk    Force Vulkan renderer
--sw    Force software renderer (CPU)
--wl    Force Wayland backend
```

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  Slint UI thread (event loop)                            │
│  MainWindow ◄── update_rx (torrents + stats)  500ms pump │
│             ◄── status_rx (status bar text)              │
│             ──► cmd_tx   (Command enum)                  │
└─────────────────────────┬────────────────────────────────┘
                          │  std::sync::mpsc
┌─────────────────────────▼────────────────────────────────┐
│  Tokio async runtime                                     │
│  backend_task: tokio::select!                            │
│    cmd_rx  → immediate RPC call                          │
│    interval tick → recently-active delta every 2s        │
│  TransmissionClient (reqwest, 409 session retry)         │
└──────────────────────────────────────────────────────────┘
```

---

## File structure

```
├── Cargo.toml
├── Cargo.lock
├── build.rs
├── PKGBUILD
├── .SRCINFO
├── ui/
│   ├── main.slint
│   └── app-icon.png
└── src/
    ├── main.rs            ← UI wiring, timers, model updates
    ├── rpc.rs             ← async Transmission JSON-RPC client
    ├── config.rs          ← reads Transmission settings.json
    ├── app_config.rs      ← application config
    ├── daemon.rs          ← auto-start/stop transmission-daemon
    ├── disks.rs           ← physical disk detection via lsblk
    ├── tray.rs            ← StatusNotifierItem (native zbus 4)
    ├── notify.rs          ← desktop notifications
    ├── filepicker.rs      ← zenity/kdialog file dialogs
    ├── single_instance.rs ← Unix socket single-instance lock
    ├── wm_icon.rs         ← _NET_WM_ICON taskbar icon (X11)
    ├── suspend.rs         ← SIGSTOP/SIGCONT process suspend
    └── i18n.rs            ← multi-language static strings (5 langs)
```

---

## License

GPL-2.0-or-later. See [LICENSE](LICENSE).

### Component licenses:
- **Slint** — GPLv3 (UI toolkit)
- **zbus** — MIT/Apache-2.0 (D-Bus)
- **tokio** — MIT (async runtime)
- **reqwest** — MIT/Apache-2.0 (HTTP client)
- **serde** — MIT/Apache-2.0 (serialization)
- **image** — MIT (icon processing)

---

## Release Notes

See [GitHub Releases](https://github.com/guglovich/Transmission-Remote-Slint/releases) for changelog.
