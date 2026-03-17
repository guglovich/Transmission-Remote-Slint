// src/daemon.rs

use std::process::{Command, Stdio};
use std::time::Duration;
use crate::config::{RpcConfig, detect_rpc_candidates};

#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub ok:         bool,
    pub status_msg: String,
}

pub enum DaemonHandle {
    Spawned,   // мы запустили — при закрытии --exit
    External,  // уже работал — при закрытии --exit
}

async fn probe_once(candidates: &[RpcConfig]) -> Option<RpcConfig> {
    for cfg in candidates {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2)).build().unwrap();
        let mut req = client.post(&cfg.url)
            .header("Content-Type", "application/json")
            .body(r#"{"method":"session-get","arguments":{}}"#);
        if let Some(ref u) = cfg.user {
            req = req.basic_auth(u, cfg.password.as_deref());
        }
        if let Ok(r) = req.send().await {
            let code = r.status().as_u16();
            if code == 409 || code == 401 {
                return Some(cfg.clone());
            }
            if code == 200 {
                let body = r.text().await.unwrap_or_default();
                if body.trim_start().starts_with('{') && body.contains("result") {
                    return Some(cfg.clone());
                }
            }
        }
    }
    None
}

fn port_bound() -> bool {
    Command::new("ss").args(["-tlnH", "sport = :9091"])
        .output()
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false)
}

pub async fn ensure_daemon(
    status_tx: &std::sync::mpsc::SyncSender<String>,
) -> (DaemonHandle, RpcConfig, ProbeResult) {

    // Читаем конфиг ОДИН раз
    let candidates = detect_rpc_candidates();
    let fallback = candidates.first().cloned()
        .unwrap_or_else(crate::config::detect_rpc_config);

    // Шаг 1: HTTP probe — нашли сразу?
    let _ = status_tx.try_send("Connecting to Transmission…".into());
    if let Some(cfg) = probe_once(&candidates).await {
        eprintln!("[daemon] Found via HTTP: {}", cfg.url);
        return (DaemonHandle::External, cfg, ProbeResult { ok: true, status_msg: "Connected".into() });
    }

    // Шаг 2: ss говорит порт занят — демон работает, просто HTTP не прошёл
    if port_bound() {
        eprintln!("[daemon] Port 9091 bound (ss) but HTTP probe failed — waiting...");
        let _ = status_tx.try_send("Daemon found, connecting…".into());
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Some(cfg) = probe_once(&candidates).await {
                eprintln!("[daemon] Connected after wait: {}", cfg.url);
                return (DaemonHandle::External, cfg, ProbeResult { ok: true, status_msg: "Connected".into() });
            }
        }
        let msg = "Daemon running but not accessible (check rpc-whitelist in settings.json)".into();
        eprintln!("[daemon] {msg}");
        let _ = status_tx.try_send(msg);
        return (DaemonHandle::External, fallback, ProbeResult { ok: false, status_msg: "Not accessible".into() });
    }

    // Шаг 3: порт точно свободен — запускаем
    eprintln!("[daemon] Port free, spawning transmission-daemon...");
    let _ = status_tx.try_send("Starting Transmission…".into());

    let Some(bin) = find_bin("transmission-daemon") else {
        let _ = status_tx.try_send("transmission-daemon not found".into());
        return (DaemonHandle::External, fallback, ProbeResult { ok: false, status_msg: "Not installed".into() });
    };

    if let Ok(mut child) = Command::new(&bin).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
        let _ = child.wait();
    }
    eprintln!("[daemon] Spawned {bin}");

    for i in 0..40 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        if let Some(cfg) = probe_once(&candidates).await {
            eprintln!("[daemon] Ready: {}", cfg.url);
            return (DaemonHandle::Spawned, cfg, ProbeResult { ok: true, status_msg: "Connected".into() });
        }
        if i % 8 == 7 {
            let _ = status_tx.try_send(format!("Waiting for daemon… ({}s)", (i+1)/4));
        }
    }

    (DaemonHandle::Spawned, fallback, ProbeResult { ok: false, status_msg: "Not responding".into() })
}

/// Всегда останавливаем через --exit — не зависит от PID
pub fn stop_daemon(handle: DaemonHandle, cfg: &RpcConfig) {
    let label = match handle { DaemonHandle::Spawned => "spawned", DaemonHandle::External => "external" };
    eprintln!("[daemon] Stopping {label} daemon");

    // Способ 1: SIGTERM по PID из pidof
    if let Ok(out) = Command::new("pidof").arg("transmission-daemon").output() {
        let pids = String::from_utf8_lossy(&out.stdout);
        for pid_str in pids.split_whitespace() {
            if let Ok(pid) = pid_str.parse::<libc::pid_t>() {
                eprintln!("[daemon] SIGTERM pid={pid}");
                #[cfg(unix)]
                unsafe { libc::kill(pid, libc::SIGTERM); }
            }
        }
        // Ждём завершения
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Ok(o) = Command::new("pidof").arg("transmission-daemon").output() {
                if o.stdout.is_empty() { 
                    eprintln!("[daemon] Daemon stopped");
                    return; 
                }
            }
        }
        // SIGKILL если не остановился
        for pid_str in pids.split_whitespace() {
            if let Ok(pid) = pid_str.parse::<libc::pid_t>() {
                #[cfg(unix)]
                unsafe { libc::kill(pid, libc::SIGKILL); }
            }
        }
    } else {
        // Fallback: transmission-remote с host:port (стандартный путь)
        let host_port = cfg.url.trim_start_matches("http://")
            .split('/').next().unwrap_or("127.0.0.1:9091");
        eprintln!("[daemon] Fallback: transmission-remote {host_port} --exit");
        let _ = Command::new("transmission-remote")
            .args([host_port, "--exit"])
            .status();
    }
}

fn find_bin(name: &str) -> Option<String> {
    for dir in &["/usr/bin", "/usr/local/bin", "/opt/bin"] {
        let p = format!("{dir}/{name}");
        if std::path::Path::new(&p).exists() { return Some(p); }
    }
    None
}
