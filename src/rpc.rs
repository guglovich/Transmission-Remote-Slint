// src/rpc.rs

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

const SESSION_HEADER: &str = "X-Transmission-Session-Id";

#[derive(Debug, Serialize)]
struct RpcRequest<'a> {
    method: &'a str,
    arguments: Value,
}

#[derive(Debug, Deserialize)]
struct RpcResponse {
    result: String,
    arguments: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTorrent {
    pub id:            i64,
    pub name:          String,
    pub status:        i64,
    pub percent_done:  f64,
    pub rate_download: i64,
    pub rate_upload:   i64,
    #[serde(default)]
    pub download_dir:  String,
    #[serde(default)]
    pub error:         i64,
    /// Текстовое описание ошибки от демона (трекер, HTTP, etc.)
    #[serde(default)]
    pub error_string:  String,
}

impl RawTorrent {
    pub fn status_label(&self) -> &'static str {
        use crate::i18n::*;
        if self.error == 3 { return err_missing();      }
        if self.error == 2 { return err_tracker_err();  }
        if self.error == 1 { return err_tracker_warn(); }
        if self.error > 0  { return err_generic();      }
        match self.status {
            0 => status_stopped(),
            1 => status_check_wait(),
            2 => status_checking(),
            3 => status_dl_queue(),
            4 => status_downloading(),
            5 => status_seed_queue(),
            6 => status_seeding(),
            _ => status_unknown(),
        }
    }
    pub fn is_paused(&self) -> bool { self.status == 0 }
    pub fn is_error(&self) -> bool  { self.error > 0 }
}

#[derive(Clone)]
pub struct TransmissionClient {
    http:           Client,
    url:            String,
    pub user:       Option<String>,
    pub password:   Option<String>,
    session_id:     Arc<Mutex<String>>,
    use_snake_case: Arc<std::sync::atomic::AtomicBool>,
}

impl TransmissionClient {
    pub fn with_auth(
        url: impl Into<String>,
        user: Option<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            http: Client::builder()
                .timeout(Duration::from_secs(30))
                .tcp_keepalive(Duration::from_secs(10))
                .build()
                .expect("HTTP client"),
            url: url.into(),
            user,
            password,
            session_id:     Arc::new(Mutex::new(String::new())),
            use_snake_case: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Определяем версию RPC — 4.1.0+ использует snake_case для имён методов
    pub async fn detect_rpc_version(&self) {
        // Пробуем session-get (старый стиль)
        match self.call("session-get", json!({"fields":["rpc-version-semver","version"]})).await {
            Ok(v) => {
                let semver = v["rpc-version-semver"].as_str().unwrap_or("");
                let ver    = v["version"].as_str().unwrap_or("");
                eprintln!("[rpc] version={ver} rpc-version-semver={semver}");
                // 4.1.0 → rpc_version_semver "6.0.0" — используем snake_case
                let snake = semver >= "6.0.0";
                self.use_snake_case.store(snake, std::sync::atomic::Ordering::Relaxed);
                eprintln!("[rpc] snake_case={snake}");
            }
            Err(_) => {
                // session-get упал — пробуем snake_case session_get
                if self.call("session_get", json!({"fields":["rpc-version-semver","version"]})).await.is_ok() {
                    eprintln!("[rpc] snake_case detected via session_get");
                    self.use_snake_case.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }

    /// Конвертирует имя метода в нужный формат (dash или snake)
    fn method_name<'a>(&self, dash_name: &'a str) -> std::borrow::Cow<'a, str> {
        if self.use_snake_case.load(std::sync::atomic::Ordering::Relaxed) {
            std::borrow::Cow::Owned(dash_name.replace('-', "_"))
        } else {
            std::borrow::Cow::Borrowed(dash_name)
        }
    }

    async fn call(&self, method: &str, args: Value) -> Result<Value> {
        let body = serde_json::to_string(&RpcRequest { method, arguments: args })?;
        #[cfg(debug_assertions)]
        eprintln!("[rpc_call] method={method:?} body={body}");

        for attempt in 0..3u8 {
            let sid = self.session_id.lock().await.clone();

            // Парсим хост:порт из URL для Host header (нужен для rpc-host-whitelist)
            // Не перезаписываем Host — reqwest сам ставит правильный из URL
            // Ручной Host ломал rpc-host-whitelist проверку в Transmission
            let mut req = self.http
                .post(&self.url)
                .header("Content-Type", "application/json")
                .header(SESSION_HEADER, &sid)
                .body(body.clone());

            // Basic auth если включена в конфиге
            if let Some(ref u) = self.user {
                req = req.basic_auth(u, self.password.as_deref());
            }

            let resp = req.send().await
                .map_err(|e| anyhow!("Connection refused — daemon running? ({e})"))?;

            let code = resp.status();

            if code == 409 {
                if let Some(v) = resp.headers().get(SESSION_HEADER) {
                    *self.session_id.lock().await =
                        v.to_str().unwrap_or_default().to_owned();
                }
                if attempt < 2 { continue; }
                return Err(anyhow!("Session negotiation failed"));
            }

            let text = resp.text().await
                .map_err(|e| anyhow!("Body read: {e}"))?;

            if !code.is_success() {
                let t = text.trim();
                return Err(match code.as_u16() {
                    403 => anyhow!(
                        "403 Forbidden — add 127.0.0.1 to rpc-whitelist in Transmission settings"
                    ),
                    401 => anyhow!(
                        "401 Unauthorized — enable auth in Transmission settings or enter credentials"
                    ),
                    _ if t.starts_with('<') => anyhow!(
                        "Wrong RPC path — check rpc-url in Transmission settings.json"
                    ),
                    _ => anyhow!("HTTP {code}: {}", &t[..t.len().min(80)]),
                });
            }

            let rpc: RpcResponse = serde_json::from_str(&text).map_err(|e| {
                let t = text.trim();
                if t.starts_with('<') {
                    anyhow!("Got HTML — rpc-url in settings.json is wrong")
                } else {
                    anyhow!("JSON parse ({e}): {}", &t[..t.len().min(80)])
                }
            })?;

            if rpc.result != "success" {
                return Err(anyhow!("RPC: {}", rpc.result));
            }
            return Ok(rpc.arguments.unwrap_or(Value::Null));
        }
        Err(anyhow!("RPC exhausted retries"))
    }

    const FIELDS: &'static [&'static str] = &[
        "id","name","status","percentDone","rateDownload","rateUpload",
        "downloadDir","error","errorString"
    ];

    /// Полный список — вызывается один раз при старте
    pub async fn get_all_torrents(&self) -> Result<Vec<RawTorrent>> {
        let val = self.call(&self.method_name("torrent-get"), json!({
            "fields": Self::FIELDS
        })).await?;
        serde_json::from_value(val["torrents"].clone())
            .map_err(|e| anyhow!("Deserializing all: {e}"))
    }

    /// Только изменившиеся с последнего опроса — "recently-active"
    pub async fn get_recently_active(&self) -> Result<(Vec<RawTorrent>, Vec<i64>)> {
        let val = self.call(&self.method_name("torrent-get"), json!({
            "ids":    "recently-active",
            "fields": Self::FIELDS
        })).await?;
        let changed: Vec<RawTorrent> = serde_json::from_value(val["torrents"].clone())
            .unwrap_or_default();
        let removed: Vec<i64> = val["removed"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();
        Ok((changed, removed))
    }

    pub async fn start_torrent(&self, id: i64) -> Result<()> {
        self.call(&self.method_name("torrent-start"), json!({ "ids": [id] })).await.map(|_| ())
    }
    pub async fn stop_torrent(&self, id: i64) -> Result<()> {
        self.call(&self.method_name("torrent-stop"), json!({ "ids": [id] })).await.map(|_| ())
    }
    pub async fn stop_torrents(&self, ids: &[i64]) -> Result<()> {
        if ids.is_empty() { return Ok(()); }
        self.call(&self.method_name("torrent-stop"), json!({ "ids": ids })).await.map(|_| ())
    }
    pub async fn start_torrents(&self, ids: &[i64]) -> Result<()> {
        if ids.is_empty() { return Ok(()); }
        self.call(&self.method_name("torrent-start"), json!({ "ids": ids })).await.map(|_| ())
    }
    pub async fn start_all(&self) -> Result<()> {
        self.call(&self.method_name("torrent-start"), json!({})).await.map(|_| ())
    }
    pub async fn stop_all(&self) -> Result<()> {
        self.call(&self.method_name("torrent-stop"), json!({})).await.map(|_| ())
    }
    pub async fn remove_torrent(&self, id: i64, remove_data: bool) -> Result<()> {
        self.call(&self.method_name("torrent-remove"),
            json!({ "ids": [id], "delete-local-data": remove_data }),
        ).await.map(|_| ())
    }
    pub async fn add_torrent_url(&self, url: &str, download_dir: Option<&str>) -> Result<()> {
        let mut args = json!({ "filename": url });
        if let Some(dir) = download_dir {
            args["download-dir"] = json!(dir);
        }
        self.call(&self.method_name("torrent-add"), args).await.map(|_| ())
    }
    pub async fn add_torrent_file(&self, path: &str, download_dir: Option<&str>) -> Result<()> {
        use base64::Engine;
        let bytes = std::fs::read(path)
            .map_err(|e| anyhow!("Read {path}: {e}"))?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let mut args = json!({ "metainfo": b64 });
        if let Some(dir) = download_dir {
            args["download-dir"] = json!(dir);
        }
        self.call(&self.method_name("torrent-add"), args).await.map(|_| ())
    }
    pub async fn recheck_torrent(&self, id: i64) -> Result<()> {
        self.call(&self.method_name("torrent-verify"), json!({ "ids": [id] })).await.map(|_| ())
    }
    pub async fn set_location(&self, id: i64, location: &str, do_move: bool) -> Result<()> {
        self.call(&self.method_name("torrent-set-location"),
            json!({ "ids": [id], "location": location, "move": do_move }),
        ).await.map(|_| ())
    }
}

// ── Session settings ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DaemonSettings {
    pub speed_limit_up_enabled: bool,
    pub speed_limit_up: i64,
    pub speed_limit_down_enabled: bool,
    pub speed_limit_down: i64,
    pub alt_speed_enabled: bool,
    pub alt_speed_up: i64,
    pub alt_speed_down: i64,
    pub alt_speed_time_enabled: bool,
    pub alt_speed_time_begin: i64,
    pub alt_speed_time_end: i64,
    pub alt_speed_time_day: i64,
    pub download_dir: String,
    pub download_queue_enabled: bool,
    pub download_queue_size: i64,
    pub queue_stalled_enabled: bool,
    pub queue_stalled_minutes: i64,
    pub start_added_torrents: bool,
    pub trash_original_torrent_files: bool,
    pub rename_partial_files: bool,
    pub incomplete_dir_enabled: bool,
    pub incomplete_dir: String,
    pub script_torrent_done_enabled: bool,
    pub script_torrent_done_filename: String,
    pub script_torrent_done_seeding_enabled: bool,
    pub script_torrent_done_seeding_filename: String,
    pub seed_ratio_limited: bool,
    pub seed_ratio_limit: f64,
    pub idle_seeding_limit_enabled: bool,
    pub idle_seeding_limit: i64,
    pub peer_port: i64,
    pub peer_port_random_on_start: bool,
    pub port_forwarding_enabled: bool,
    pub peer_limit_per_torrent: i64,
    pub peer_limit_global: i64,
    pub utp_enabled: bool,
    pub pex_enabled: bool,
    pub dht_enabled: bool,
    pub lpd_enabled: bool,
    pub default_trackers: String,
    pub encryption: i64,
    pub blocklist_enabled: bool,
    pub blocklist_url: String,
    pub watch_dir_enabled: bool,
    pub watch_dir: String,
    pub rpc_enabled: bool,
    pub rpc_port: i64,
    pub rpc_authentication_required: bool,
    pub rpc_username: String,
    pub rpc_password: String,
    pub rpc_whitelist_enabled: bool,
    pub rpc_whitelist: String,
}

impl TransmissionClient {
    pub async fn session_get_settings(&self) -> Result<DaemonSettings> {
        let val = self.call(&self.method_name("session-get"), json!({
            "fields": [
                "speed-limit-up-enabled","speed-limit-up",
                "speed-limit-down-enabled","speed-limit-down",
                "alt-speed-enabled","alt-speed-up","alt-speed-down",
                "alt-speed-time-enabled","alt-speed-time-begin","alt-speed-time-end","alt-speed-time-day",
                "download-dir",
                "download-queue-enabled","download-queue-size",
                "queue-stalled-enabled","queue-stalled-minutes",
                "start-added-torrents","trash-original-torrent-files","rename-partial-files",
                "incomplete-dir-enabled","incomplete-dir",
                "script-torrent-done-enabled","script-torrent-done-filename",
                "script-torrent-done-seeding-enabled","script-torrent-done-seeding-filename",
                "seedRatioLimited","seedRatioLimit",
                "idle-seeding-limit-enabled","idle-seeding-limit",
                "peer-port","peer-port-random-on-start","port-forwarding-enabled",
                "peer-limit-per-torrent","peer-limit-global",
                "utp-enabled","pex-enabled","dht-enabled","lpd-enabled",
                "default-trackers",
                "encryption",
                "blocklist-enabled","blocklist-url",
                "watch-dir-enabled","watch-dir",
                "rpc-enabled","rpc-port",
                "rpc-authentication-required","rpc-username","rpc-password",
                "rpc-whitelist-enabled","rpc-whitelist"
            ]
        })).await?;

        Ok(DaemonSettings {
            speed_limit_up_enabled: val["speed-limit-up-enabled"].as_bool().unwrap_or(false),
            speed_limit_up: val["speed-limit-up"].as_i64().unwrap_or(0),
            speed_limit_down_enabled: val["speed-limit-down-enabled"].as_bool().unwrap_or(true),
            speed_limit_down: val["speed-limit-down"].as_i64().unwrap_or(500),
            alt_speed_enabled: val["alt-speed-enabled"].as_bool().unwrap_or(false),
            alt_speed_up: val["alt-speed-up"].as_i64().unwrap_or(50),
            alt_speed_down: val["alt-speed-down"].as_i64().unwrap_or(50),
            alt_speed_time_enabled: val["alt-speed-time-enabled"].as_bool().unwrap_or(false),
            alt_speed_time_begin: val["alt-speed-time-begin"].as_i64().unwrap_or(540),
            alt_speed_time_end: val["alt-speed-time-end"].as_i64().unwrap_or(1020),
            alt_speed_time_day: val["alt-speed-time-day"].as_i64().unwrap_or(31),
            download_dir: val["download-dir"].as_str().unwrap_or("").to_string(),
            download_queue_enabled: val["download-queue-enabled"].as_bool().unwrap_or(true),
            download_queue_size: val["download-queue-size"].as_i64().unwrap_or(5),
            queue_stalled_enabled: val["queue-stalled-enabled"].as_bool().unwrap_or(true),
            queue_stalled_minutes: val["queue-stalled-minutes"].as_i64().unwrap_or(30),
            start_added_torrents: val["start-added-torrents"].as_bool().unwrap_or(true),
            trash_original_torrent_files: val["trash-original-torrent-files"].as_bool().unwrap_or(true),
            rename_partial_files: val["rename-partial-files"].as_bool().unwrap_or(true),
            incomplete_dir_enabled: val["incomplete-dir-enabled"].as_bool().unwrap_or(false),
            incomplete_dir: val["incomplete-dir"].as_str().unwrap_or("").to_string(),
            script_torrent_done_enabled: val["script-torrent-done-enabled"].as_bool().unwrap_or(false),
            script_torrent_done_filename: val["script-torrent-done-filename"].as_str().unwrap_or("").to_string(),
            script_torrent_done_seeding_enabled: val["script-torrent-done-seeding-enabled"].as_bool().unwrap_or(false),
            script_torrent_done_seeding_filename: val["script-torrent-done-seeding-filename"].as_str().unwrap_or("").to_string(),
            seed_ratio_limited: val["seedRatioLimited"].as_bool().unwrap_or(false),
            seed_ratio_limit: val["seedRatioLimit"].as_f64().unwrap_or(2.0),
            idle_seeding_limit_enabled: val["idle-seeding-limit-enabled"].as_bool().unwrap_or(false),
            idle_seeding_limit: val["idle-seeding-limit"].as_i64().unwrap_or(30),
            peer_port: val["peer-port"].as_i64().unwrap_or(51413),
            peer_port_random_on_start: val["peer-port-random-on-start"].as_bool().unwrap_or(true),
            port_forwarding_enabled: val["port-forwarding-enabled"].as_bool().unwrap_or(true),
            peer_limit_per_torrent: val["peer-limit-per-torrent"].as_i64().unwrap_or(50),
            peer_limit_global: val["peer-limit-global"].as_i64().unwrap_or(200),
            utp_enabled: val["utp-enabled"].as_bool().unwrap_or(true),
            pex_enabled: val["pex-enabled"].as_bool().unwrap_or(true),
            dht_enabled: val["dht-enabled"].as_bool().unwrap_or(true),
            lpd_enabled: val["lpd-enabled"].as_bool().unwrap_or(true),
            default_trackers: val["default-trackers"].as_str().unwrap_or("").to_string(),
            encryption: val["encryption"].as_i64().unwrap_or(1),
            blocklist_enabled: val["blocklist-enabled"].as_bool().unwrap_or(false),
            blocklist_url: val["blocklist-url"].as_str().unwrap_or("").to_string(),
            watch_dir_enabled: val["watch-dir-enabled"].as_bool().unwrap_or(false),
            watch_dir: val["watch-dir"].as_str().unwrap_or("").to_string(),
            rpc_enabled: val["rpc-enabled"].as_bool().unwrap_or(true),
            rpc_port: val["rpc-port"].as_i64().unwrap_or(9091),
            rpc_authentication_required: val["rpc-authentication-required"].as_bool().unwrap_or(false),
            rpc_username: val["rpc-username"].as_str().unwrap_or("").to_string(),
            rpc_password: val["rpc-password"].as_str().unwrap_or("").to_string(),
            rpc_whitelist_enabled: val["rpc-whitelist-enabled"].as_bool().unwrap_or(true),
            rpc_whitelist: val["rpc-whitelist"].as_str().unwrap_or("127.0.0.1").to_string(),
        })
    }

    pub async fn session_set_settings(&self, s: &DaemonSettings) -> Result<()> {
        let args = json!({
            "speed-limit-up-enabled": s.speed_limit_up_enabled,
            "speed-limit-up": s.speed_limit_up,
            "speed-limit-down-enabled": s.speed_limit_down_enabled,
            "speed-limit-down": s.speed_limit_down,
            "alt-speed-enabled": s.alt_speed_enabled,
            "alt-speed-up": s.alt_speed_up,
            "alt-speed-down": s.alt_speed_down,
            "alt-speed-time-enabled": s.alt_speed_time_enabled,
            "alt-speed-time-begin": s.alt_speed_time_begin,
            "alt-speed-time-end": s.alt_speed_time_end,
            "alt-speed-time-day": s.alt_speed_time_day,
            "download-dir": s.download_dir,
            "download-queue-enabled": s.download_queue_enabled,
            "download-queue-size": s.download_queue_size,
            "queue-stalled-enabled": s.queue_stalled_enabled,
            "queue-stalled-minutes": s.queue_stalled_minutes,
            "start-added-torrents": s.start_added_torrents,
            "trash-original-torrent-files": s.trash_original_torrent_files,
            "rename-partial-files": s.rename_partial_files,
            "incomplete-dir-enabled": s.incomplete_dir_enabled,
            "incomplete-dir": s.incomplete_dir,
            "script-torrent-done-enabled": s.script_torrent_done_enabled,
            "script-torrent-done-filename": s.script_torrent_done_filename,
            "script-torrent-done-seeding-enabled": s.script_torrent_done_seeding_enabled,
            "script-torrent-done-seeding-filename": s.script_torrent_done_seeding_filename,
            "seedRatioLimited": s.seed_ratio_limited,
            "seedRatioLimit": s.seed_ratio_limit,
            "idle-seeding-limit-enabled": s.idle_seeding_limit_enabled,
            "idle-seeding-limit": s.idle_seeding_limit,
            "peer-port": s.peer_port,
            "peer-port-random-on-start": s.peer_port_random_on_start,
            "port-forwarding-enabled": s.port_forwarding_enabled,
            "peer-limit-per-torrent": s.peer_limit_per_torrent,
            "peer-limit-global": s.peer_limit_global,
            "utp-enabled": s.utp_enabled,
            "pex-enabled": s.pex_enabled,
            "dht-enabled": s.dht_enabled,
            "lpd-enabled": s.lpd_enabled,
            "default-trackers": s.default_trackers,
            "encryption": s.encryption,
            "blocklist-enabled": s.blocklist_enabled,
            "blocklist-url": s.blocklist_url,
            "watch-dir-enabled": s.watch_dir_enabled,
            "watch-dir": s.watch_dir,
            "rpc-enabled": s.rpc_enabled,
            "rpc-port": s.rpc_port,
            "rpc-authentication-required": s.rpc_authentication_required,
            "rpc-username": s.rpc_username,
            "rpc-password": s.rpc_password,
            "rpc-whitelist-enabled": s.rpc_whitelist_enabled,
            "rpc-whitelist": s.rpc_whitelist
        });
        let _ = self.call(&self.method_name("session-set"), args).await?;
        Ok(())
    }

    /// Проверка открыт ли порт (RPC port-test) — демон стучится к своим серверам,
    /// может занять 10-30 секунд.
    pub async fn port_test(&self) -> Result<bool> {
        let val = self.call(&self.method_name("port-test"), json!({})).await?;
        Ok(val["port-is-open"].as_bool().unwrap_or(false))
    }

    /// Обновляет blocklist на демоне (RPC blocklist-update).
    /// Возвращает число записей в обновлённом списке (blocklist-size).
    /// Внимание: демон скачивает список — вызов может занять десятки секунд.
    pub async fn blocklist_update(&self) -> Result<i64> {
        let val = self.call(&self.method_name("blocklist-update"), json!({})).await?;
        Ok(val["blocklist-size"].as_i64().unwrap_or(0))
    }

    /// Число трекеров, которым демон отправит `event=stopped` при завершении:
    /// не-backup трекеры всех ЗАПУЩЕННЫХ торрентов (остановленные не анонсятся).
    pub async fn count_active_trackers(&self) -> Result<i64> {
        Ok(self.close_summary().await?.trackers)
    }

    /// Снимок для анимации закрытия: что именно уйдёт трекерам.
    /// Раздачи/загрузки, сессионные итоги (↑/↓), трекеры и топ-торренты для тикера.
    pub async fn close_summary(&self) -> Result<CloseSummary> {
        let val = self.call(&self.method_name("torrent-get"), json!({
            "fields": ["status", "percentDone", "rateUpload", "rateDownload", "name", "leftUntilComplete", "trackerStats"]
        })).await?;
        let mut s = CloseSummary::default();
        if let Some(torrents) = val["torrents"].as_array() {
            for t in torrents {
                let status = t["status"].as_i64().unwrap_or(0);
                if status == 0 { continue; } // stopped — не анонсируется
                s.running += 1;
                // Классификация как в сайдбаре: Раздаются = 6, Загружаются = 4
                match status {
                    6 => s.seeding += 1,
                    4 => s.downloading += 1,
                    _ => {} // 1..=3, 5 — промежуточные состояния, в сайдбаре не показываются
                }
                // left уходит в stopped-announce как есть (поле left=)
                s.left_bytes += t["leftUntilComplete"].as_i64().unwrap_or(0).max(0);
                if let Some(trackers) = t["trackerStats"].as_array() {
                    for tr in trackers {
                        if tr["isBackup"].as_bool().unwrap_or(false) { continue; }
                        s.trackers += 1;
                        if let Some(h) = tr["host"].as_str() {
                            if !h.is_empty() { s.hosts.push(h.to_string()); }
                        }
                    }
                }
                s.items.push(CloseItem {
                    name: t["name"].as_str().unwrap_or("").to_string(),
                    percent: t["percentDone"].as_f64().unwrap_or(0.0) * 100.0,
                    up: t["rateUpload"].as_i64().unwrap_or(0),
                    down: t["rateDownload"].as_i64().unwrap_or(0),
                });
            }
        }
        // Уникальные хосты, по алфавиту — это адресаты stopped-announce
        s.hosts.sort();
        s.hosts.dedup();
        // Для тикера — самые живые (по скорости) сверху
        s.items.sort_by(|a, b| (b.up + b.down).cmp(&(a.up + a.down)));
        s.items.truncate(40);
        Ok(s)
    }
}

// ── Session stats ─────────────────────────────────────────────────────────────

/// Строка тикера закрытия: один активный торрент
#[derive(Debug, Clone, Default)]
pub struct CloseItem {
    pub name: String,
    pub percent: f64,
    pub up: i64,
    pub down: i64,
}

/// Снимок состояния на момент закрытия — показывается в анимации.
/// ВАЖНО: stopped-announce несёт per-tier счётчики (up/down/corrupt с момента
/// последнего успешного stop по ЭТОМУ трекеру) — RPC их НЕ отдаёт, поэтому
/// объёмы ↑/↓ здесь не показываем. Честно показываем: статусы, `left`
/// (поле leftUntilComplete — уходит в announce как есть) и хосты трекеров.
#[derive(Debug, Clone, Default)]
pub struct CloseSummary {
    pub running: i64,
    pub seeding: i64,
    pub downloading: i64,
    pub trackers: i64,
    pub left_bytes: i64,
    pub hosts: Vec<String>,
    pub items: Vec<CloseItem>,
}

#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub down_speed:   i64,
    pub up_speed:     i64,
    pub downloaded:   i64,
    pub uploaded:     i64,
    pub ratio:        f64,
    pub active_count: i64,
    pub dht_nodes:    i64,
    pub peer_count:   i64,
}

impl TransmissionClient {
    pub async fn get_session_stats(&self) -> Result<SessionStats> {
        let val = self.call(&self.method_name("session-stats"), json!({})).await?;
        let cum = &val["cumulative-stats"];
        let ul  = cum["uploadedBytes"].as_i64().unwrap_or(0);
        let dl  = cum["downloadedBytes"].as_i64().unwrap_or(0);
        Ok(SessionStats {
            down_speed:   val["downloadSpeed"].as_i64().unwrap_or(0),
            up_speed:     val["uploadSpeed"].as_i64().unwrap_or(0),
            downloaded:   dl,
            uploaded:     ul,
            ratio:        if dl > 0 { ul as f64 / dl as f64 } else { 0.0 },
            active_count: val["activeTorrentCount"].as_i64().unwrap_or(0),
            dht_nodes:    val["dhtNodeCount"].as_i64().unwrap_or(0),
            peer_count:   val["peerCount"].as_i64().unwrap_or(0),
        })
    }
}
