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
    user:           Option<String>,
    password:       Option<String>,
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
    pub async fn create_torrent(&self, path: &str, trackers: &[&str]) -> Result<Vec<u8>> {
        use base64::Engine;
        let method = self.method_name("torrent-create");
        eprintln!("[create] calling {method} RPC, path={path}");
        let result = self.call(&method, json!({
            "path":         path,
            "tracker-list": trackers,
        })).await;
        eprintln!("[create] RPC result: {:?}", result.as_ref().map(|v| v.to_string()).map_err(|e| e.to_string()));
        let result = result?;
        eprintln!("[create] raw response: {}", serde_json::to_string_pretty(&result).unwrap_or_default());
        let b64 = result["torrent"]["metainfo"]
            .as_str()
            .ok_or_else(|| anyhow!("torrent-create: no metainfo in response. Full: {result}"))?;
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64)
            .map_err(|e| anyhow!("torrent-create: base64 decode: {e}"))?;
        Ok(bytes)
    }
}

// ── Session stats ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub down_speed:   i64,
    pub up_speed:     i64,
    pub downloaded:   i64,
    pub uploaded:     i64,
    pub ratio:        f64,
    pub active_count: i64,
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
        })
    }
}
