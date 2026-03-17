// src/config.rs

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RpcConfig {
    pub url:      String,
    pub user:     Option<String>,
    pub password: Option<String>,
    #[allow(dead_code)]
    pub source:   String,
}

impl RpcConfig {
    pub fn default_config() -> Self {
        Self {
            url:      "http://127.0.0.1:9091/transmission/rpc".into(),
            user:     None,
            password: None,
            source:   "built-in default".into(),
        }
    }
}

fn default_rpc_enabled() -> bool { true }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
struct SettingsJson {
    #[serde(default = "default_rpc_enabled")]
    rpc_enabled:                 bool,
    rpc_port:                    Option<u16>,
    rpc_bind_address:            Option<String>,
    rpc_authentication_required: bool,
    rpc_username:                Option<String>,
    rpc_host_whitelist_enabled:  bool,
    rpc_whitelist_enabled:       bool,
    rpc_whitelist:               Option<String>,
}

impl Default for SettingsJson {
    fn default() -> Self {
        Self {
            rpc_enabled: true,
            rpc_port: None,
            rpc_bind_address: None,
            rpc_authentication_required: false,
            rpc_username: None,
            rpc_host_whitelist_enabled: false,
            rpc_whitelist_enabled: false,
            rpc_whitelist: None,
        }
    }
}

/// Возвращает список URL для probe в порядке приоритета.
/// Включает все найденные конфиги + стандартные порты.
pub fn detect_rpc_candidates() -> Vec<RpcConfig> {
    let mut candidates: Vec<RpcConfig> = Vec::new();

    // Читаем все известные settings.json
    for path in config_search_paths() {
        if !path.exists() { continue; }
        eprintln!("[config] Found: {}", path.display());
        match read_settings(&path) {
            Ok(cfg) => {
                eprintln!("[config] → {}", cfg.url);
                candidates.push(cfg);
            }
            Err(e) => eprintln!("[config] Parse error: {e}"),
        }
    }

    // Всегда добавляем стандартные fallback-адреса если их ещё нет
    for fallback_url in &[
        "http://127.0.0.1:9091/transmission/rpc",
        "http://localhost:9091/transmission/rpc",
    ] {
        if !candidates.iter().any(|c| &c.url == fallback_url) {
            candidates.push(RpcConfig {
                url: fallback_url.to_string(),
                user: None, password: None,
                source: "fallback".into(),
            });
        }
    }

    candidates
}

/// Для обратной совместимости — возвращает первый кандидат как основной конфиг
pub fn detect_rpc_config() -> RpcConfig {
    detect_rpc_candidates().into_iter().next()
        .unwrap_or_else(RpcConfig::default_config)
}

fn read_settings(path: &PathBuf) -> anyhow::Result<RpcConfig> {
    let text = std::fs::read_to_string(path)?;
    let s: SettingsJson = serde_json::from_str(&text)?;
    let raw: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();

    if !s.rpc_enabled {
        return Err(anyhow::anyhow!("RPC disabled in settings"));
    }

    let port = s.rpc_port.unwrap_or(9091);

    // rpc-url задаёт базовый путь, RPC = rpc-url + "rpc"
    // "/transmission/" → "/transmission/rpc"
    // "/transmission/web/" → "/transmission/web/rpc"
    let rpc_url_raw = raw.get("rpc-url").and_then(|v| v.as_str()).unwrap_or("/transmission/");
    let rpc_path = format!("{}rpc", if rpc_url_raw.ends_with('/') { rpc_url_raw.to_string() } else { format!("{rpc_url_raw}/") });
    eprintln!("[config] rpc-url={rpc_url_raw:?} → {rpc_path}");

    let host = match s.rpc_bind_address.as_deref() {
        Some("0.0.0.0") | Some("::") | None => "127.0.0.1".to_string(),
        Some(addr) => addr.to_string(),
    };

    let url = format!("http://{host}:{port}{rpc_path}");
    eprintln!("[config] → {url}");

    if s.rpc_host_whitelist_enabled {
        eprintln!("[config] NOTE: rpc-host-whitelist-enabled=true");
    }
    if s.rpc_whitelist_enabled {
        eprintln!("[config] NOTE: rpc-whitelist={:?}", s.rpc_whitelist);
    }

    // Диагностика торрентов
    if let Some(dl) = raw.get("download-dir").and_then(|v| v.as_str()) {
        eprintln!("[config] download-dir={dl:?}");
    }
    let torrents_dir = path.parent().unwrap_or(std::path::Path::new(".")).join("torrents");
    if torrents_dir.exists() {
        let n = std::fs::read_dir(&torrents_dir).map(|d| d.count()).unwrap_or(0);
        eprintln!("[config] torrents: {n} files in {}", torrents_dir.display());
    }

    let user = if s.rpc_authentication_required {
        s.rpc_username.filter(|u| !u.is_empty())
    } else { None };

    Ok(RpcConfig {
        url,
        user,
        password: None,
        source: path.display().to_string(),
    })
}

fn config_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(h) = std::env::var("TRANSMISSION_HOME") {
        paths.push(PathBuf::from(h).join("settings.json"));
    }

    let xdg = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config")
        });

    paths.push(xdg.join("transmission-daemon/settings.json"));
    paths.push(xdg.join("transmission/settings.json"));

    // Системный сервис — пользователь 'transmission'
    paths.push(PathBuf::from("/var/lib/transmission/.config/transmission-daemon/settings.json"));
    paths.push(PathBuf::from("/var/lib/transmission/.config/transmission/settings.json"));
    // Arch: systemd сервис может использовать эту папку
    paths.push(PathBuf::from("/var/lib/transmission/settings.json"));

    paths
}
