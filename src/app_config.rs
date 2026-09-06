// src/app_config.rs
// Конфиг приложения: ~/.config/transmission-remote-slint/config.toml

use image::ImageEncoder;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Пользовательский хост (профиль подключения), добавленный вручную
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct HostProfile {
    #[serde(default)]
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Замораживать процесс (SIGSTOP) при скрытии в трей.
    /// При открытии — SIGCONT через процесс-страж.
    /// Даёт ~0 МБ ОЗУ пока окно скрыто.
    #[serde(default = "default_false")]
    pub suspend_on_hide: bool,

    /// Запускать свёрнутым в трей
    #[serde(default = "default_false")]
    pub start_minimized: bool,

    /// Интервал опроса демона в секундах (1–60)
    #[serde(default = "default_refresh")]
    pub refresh_interval_secs: u64,

    /// Язык интерфейса: "ru" или "en"
    #[serde(default = "default_lang")]
    pub language: String,

    /// Удалять .torrent файл после добавления раздачи в клиент
    /// true = удалять (поведение по умолчанию как в Transmission GTK)
    #[serde(default = "default_true")]
    pub delete_torrent_after_add: bool,

    /// Автозапуск: создаёт/удаляет ~/.config/autostart/transmission-remote-slint.desktop
    #[serde(default = "default_false")]
    pub autostart: bool,

    /// Действие при закрытии окна: 0 = спрашивать, 1 = свернуть в трей, 2 = закрыть
    #[serde(default = "default_on_close")]
    pub on_close_action: u32,

    /// Уведомление при добавлении торрента
    #[serde(default = "default_true")]
    pub notify_on_add: bool,

    /// Уведомление при завершении загрузки
    #[serde(default = "default_true")]
    pub notify_on_complete: bool,

    /// Звук при завершении загрузки
    #[serde(default = "default_true")]
    pub notify_sound: bool,

    /// Тема: 0 = тёмная, 1 = светлая, 2 = системная
    #[serde(default = "default_theme")]
    pub theme: u32,

    /// Видимость кнопок тулбара
    #[serde(default = "default_tb_add")]
    pub tb_add: bool,
    #[serde(default = "default_true")]
    pub tb_magnet: bool,
    #[serde(default = "default_true")]
    pub tb_create: bool,
    #[serde(default = "default_true")]
    pub tb_rehash: bool,
    #[serde(default = "default_false")]
    pub tb_start_sel: bool,
    #[serde(default = "default_false")]
    pub tb_pause_sel: bool,
    #[serde(default = "default_false")]
    pub tb_start_all: bool,
    #[serde(default = "default_false")]
    pub tb_pause_all: bool,

    /// Видимость секций левой панели
    #[serde(default = "default_true")]
    pub lp_status: bool,
    #[serde(default = "default_true")]
    pub lp_disks: bool,
    #[serde(default = "default_true")]
    pub lp_trackers: bool,
    #[serde(default = "default_true")]
    pub lp_webtorrents: bool,
    #[serde(default = "default_true")]
    pub lp_tags: bool,
    #[serde(default = "default_true")]
    pub lp_created: bool,

    /// Показывать скорости в битах (kbit/s, Mbit/s) вместо байт
    #[serde(default = "default_false")]
    pub speed_in_bits: bool,

    /// Показывать большие объёмы трафика в ТБ (иначе максимум ГБ)
    #[serde(default = "default_false")]
    pub traffic_in_tb: bool,

    /// Показывать диалог выбора папки при добавлении .torrent
    /// false = добавлять сразу в папку демона по умолчанию
    #[serde(default = "default_true")]
    pub dl_show_dialog: bool,

    /// Автообновление blocklist раз в сутки (RPC blocklist-update)
    #[serde(default = "default_false")]
    pub blocklist_auto_update: bool,

    /// Последнее известное число записей blocklist (для отображения в UI)
    #[serde(default)]
    pub blocklist_entries: i64,

    /// Unix-время последнего успешного blocklist-update
    #[serde(default)]
    pub blocklist_last_update: u64,

    /// Пользовательские хосты (вкладка Remote → Добавить)
    #[serde(default)]
    pub custom_hosts: Vec<HostProfile>,
}

fn default_false() -> bool { false }
fn default_refresh() -> u64 { 2 }
fn default_lang() -> String { "ru".to_string() }
fn default_true() -> bool { true }
fn default_on_close() -> u32 { 0 }
fn default_theme() -> u32 { 0 }
fn default_tb_add() -> bool { true }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            suspend_on_hide: false,
            start_minimized: false,
            refresh_interval_secs: 2,
            delete_torrent_after_add: true,
            autostart: false,
            on_close_action: 0,
            notify_on_add: true,
            notify_on_complete: true,
            notify_sound: true,
            theme: 0,
            tb_add: true,
            tb_magnet: true,
            tb_create: true,
            tb_rehash: true,
            tb_start_sel: false,
            tb_pause_sel: false,
            tb_start_all: false,
            tb_pause_all: false,
            lp_status: true,
            lp_disks: true,
            lp_trackers: true,
            lp_webtorrents: true,
            lp_tags: true,
            lp_created: true,
            speed_in_bits: false,
            traffic_in_tb: false,
            dl_show_dialog: true,
            blocklist_auto_update: false,
            blocklist_entries: 0,
            blocklist_last_update: 0,
            custom_hosts: Vec::new(),
        }
    }
}

pub fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut h = dirs_home();
            h.push(".config");
            h
        });
    base.join("transmission-remote-slint").join("config.toml")
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

pub fn load() -> AppConfig {
    let path = config_path();
    if let Ok(text) = std::fs::read_to_string(&path) {
        match toml::from_str(&text) {
            Ok(cfg) => {
                eprintln!("[app_config] Loaded: {}", path.display());
                return cfg;
            }
            Err(e) => eprintln!("[app_config] Parse error: {e}"),
        }
    }
    let cfg = AppConfig::default();
    // Сохраняем дефолт чтобы пользователь видел файл
    save(&cfg);
    cfg
}

/// Атомарная запись: скрытый tmp-файл + rename (атомарно на POSIX).
/// При сбое (power loss, OOM) файл либо старый, либо новый — никогда не битый.
fn atomic_write(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    let fname = path.file_name().map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let tmp = path.with_file_name(format!(".{fname}.tmp"));
    std::fs::write(&tmp, data)?;
    match std::fs::File::open(&tmp) {
        Ok(f) => {
            if let Err(e) = f.sync_all() {
                eprintln!("[app_config] fsync {} error: {e}", tmp.display());
            }
        }
        Err(e) => eprintln!("[app_config] reopen {} error: {e}", tmp.display()),
    }
    std::fs::rename(&tmp, path)
}

pub fn save(cfg: &AppConfig) {
    let path = config_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match toml::to_string_pretty(cfg) {
        Ok(text) => {
            if let Err(e) = atomic_write(&path, text.as_bytes()) {
                eprintln!("[app_config] Save error: {e}");
            }
        }
        Err(e) => eprintln!("[app_config] Serialize error: {e}"),
    }
}

/// Устанавливает иконку и .desktop файл при первом запуске
/// Иконки ресайзятся из встроенного PNG — критично для XFCE (нужен 22x22)
pub fn install_icon() {
    let home = dirs_home();
    let icon_src = include_bytes!("../ui/app-icon.png");
    let img = match image::load_from_memory(icon_src) {
        Ok(i) => i.to_rgba8(),
        Err(e) => { eprintln!("[icon] Failed to decode app-icon.png: {e}"); return; }
    };
    let mut needs_cache_update = false;

    for size in &[16u32, 22, 32, 48, 128, 256] {
        let icon_dir = home.join(format!(".local/share/icons/hicolor/{size}x{size}/apps"));
        let icon_path = icon_dir.join("transmission-remote-slint.png");
        let should_write = !icon_path.exists();
        if should_write {
            let _ = std::fs::create_dir_all(&icon_dir);
            let resized = if *size as u32 == img.width() {
                img.clone()
            } else {
                image::imageops::resize(&img, *size, *size, image::imageops::FilterType::Lanczos3)
            };
            let mut buf = std::io::Cursor::new(Vec::new());
            if let Err(e) = image::codecs::png::PngEncoder::new(&mut buf)
                .write_image(&resized, *size, *size, image::ExtendedColorType::Rgba8)
            {
                eprintln!("[icon] PNG encode {size}x{size} error: {e}");
            } else if let Err(e) = atomic_write(&icon_path, buf.get_ref()) {
                eprintln!("[icon] Write {size}x{size} error: {e}");
            } else {
                eprintln!("[icon] Installed {size}x{size} icon (resized)");
                needs_cache_update = true;
            }
        }
    }

    // .desktop файл — всегда перезаписываем (актуальный Exec + StartupWMClass)
    let desktop_dir = home.join(".local/share/applications");
    let desktop_path = desktop_dir.join("transmission-remote-slint.desktop");
    let _ = std::fs::create_dir_all(&desktop_dir);
    let exe = std::env::current_exe()
        .unwrap_or_else(|_| std::path::PathBuf::from("transmission-remote-slint"));
    let content = format!(
        "[Desktop Entry]\n\
        Type=Application\n\
        Name=Transmission Remote\n\
        GenericName=BitTorrent Client\n\
        Comment=Lightweight Transmission GUI (Slint, no GTK)\n\
        Exec={}\n\
        Icon=transmission-remote-slint\n\
        Categories=Network;FileTransfer;P2P;\n\
        MimeType=application/x-bittorrent;x-scheme-handler/magnet;\n\
        StartupWMClass=transmission-remote-slint\n\
        Terminal=false\n",
        exe.display()
    );
    if let Err(e) = atomic_write(&desktop_path, content.as_bytes()) {
        eprintln!("[icon] .desktop write error: {e}");
    } else {
        eprintln!("[icon] Installed/updated .desktop");
        needs_cache_update = true;
    }

    // Обновляем кэш иконок
    let hicolor = home.join(".local/share/icons/hicolor");
    let out = std::process::Command::new("gtk-update-icon-cache")
        .args(["-f", "-t", hicolor.to_str().unwrap_or("")])
        .output();
    let ok = out.as_ref().map(|o| o.status.success()).unwrap_or(false);
    if ok {
        eprintln!("[icon] Icon cache updated");
    } else {
        if let Err(e) = out {
            eprintln!("[icon] gtk-update-icon-cache spawn error: {e}");
        } else {
            eprintln!("[icon] gtk-update-icon-cache failed: {}",
                String::from_utf8_lossy(&out.unwrap().stderr).trim());
        }
        if needs_cache_update {
            let _ = std::process::Command::new("xdg-desktop-menu").arg("forceupdate").status();
            let _ = std::process::Command::new("update-desktop-database")
                .arg(desktop_dir.to_str().unwrap_or(""))
                .status();
        }
    }
}
pub fn sync_autostart(enabled: bool) {
    let autostart_dir = dirs_home().join(".config").join("autostart");
    let desktop = autostart_dir.join("transmission-remote-slint.desktop");
    if enabled {
        let _ = std::fs::create_dir_all(&autostart_dir);
        let exe = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("transmission-remote-slint"));
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=Transmission Remote\nExec={}\nHidden=false\nNoDisplay=false\nX-GNOME-Autostart-enabled=true\n",
            exe.display()
        );
        let _ = std::fs::write(&desktop, content);
        eprintln!("[app_config] Autostart enabled");
    } else {
        let _ = std::fs::remove_file(&desktop);
        eprintln!("[app_config] Autostart disabled");
    }
}
