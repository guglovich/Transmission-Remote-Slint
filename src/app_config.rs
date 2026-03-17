// src/app_config.rs
// Конфиг приложения: ~/.config/transmission-gui/config.toml

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

    /// Автозапуск: создаёт/удаляет ~/.config/autostart/transmission-gui.desktop
    #[serde(default = "default_false")]
    pub autostart: bool,
}

fn default_false() -> bool { false }
fn default_refresh() -> u64 { 2 }
fn default_lang() -> String { "ru".to_string() }
fn default_true() -> bool { true }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language:                  "ru".to_string(),
            suspend_on_hide:           false,
            start_minimized:           false,
            refresh_interval_secs:     2,
            delete_torrent_after_add:  true,
            autostart:                 false,
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
    base.join("transmission-gui").join("config.toml")
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

pub fn save(cfg: &AppConfig) {
    let path = config_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match toml::to_string_pretty(cfg) {
        Ok(text) => {
            if let Err(e) = std::fs::write(&path, text) {
                eprintln!("[app_config] Save error: {e}");
            }
        }
        Err(e) => eprintln!("[app_config] Serialize error: {e}"),
    }
}

/// Обновляет autostart .desktop файл
pub fn sync_autostart(enabled: bool) {
    let autostart_dir = dirs_home().join(".config").join("autostart");
    let desktop = autostart_dir.join("transmission-gui.desktop");
    if enabled {
        let _ = std::fs::create_dir_all(&autostart_dir);
        let exe = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("transmission-gui"));
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
