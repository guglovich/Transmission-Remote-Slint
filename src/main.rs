#![recursion_limit = "256"]
// src/main.rs

mod app_config;
mod config;
mod daemon;
mod disks;
mod filepicker;
mod i18n;
mod notify;
mod rpc;
mod single_instance;
mod suspend;
mod tray;
mod wm_icon;

use rpc::{DaemonSettings, SessionStats as RpcStats, TransmissionClient};
use slint::{Model, ModelRc, SharedString, VecModel};
use std::rc::Rc;
use std::time::Duration;
use tokio::sync::mpsc;

slint::include_modules!();

// ── Render backend ────────────────────────────────────────────────────────────

fn apply_render_backend(args: &[String]) {
    for arg in args {
        match arg.as_str() {
            "--gl"        => { set_renderer("skia-opengl"); return; }
            "--vk"        => { set_renderer("skia-vulkan"); return; }
            "--sw"|"--cpu"=> { set_renderer("software");    return; }
            "--wl"        => { std::env::remove_var("WINIT_UNIX_BACKEND"); return; }
            "--help"|"-h" => {
                eprintln!("transmission-remote-slint [--gl|--vk|--sw|--wl]");
                std::process::exit(0);
            }
            _ => {}
        }
    }
    if std::env::var("DISPLAY").is_ok() {
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
        if std::env::var("SLINT_RENDERER").is_err() {
            let has_vulkan = std::process::Command::new("vulkaninfo")
                .arg("--summary")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            std::env::set_var("SLINT_RENDERER",
                if has_vulkan { "skia-vulkan" } else { "skia-opengl" });
        }
    }
}

fn set_renderer(r: &str) {
    std::env::set_var("SLINT_BACKEND", "winit");
    std::env::set_var("SLINT_RENDERER", r);
    eprintln!("[render] {r}");
}

// ── Форматирование ────────────────────────────────────────────────────────────

/// Текущий полный URL RPC — для active-флагов профилей
static ACTIVE_RPC_URL: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

fn active_rpc_url() -> String {
    ACTIVE_RPC_URL.lock().unwrap().clone().unwrap_or_default()
}

/// url → "host:port" для отображения
fn url_endpoint(url: &str) -> String {
    url.trim_start_matches("http://").trim_start_matches("https://")
        .split('/').next().unwrap_or(url).to_string()
}

fn is_local_url(url: &str) -> bool {
    let ep = url_endpoint(url);
    ep.starts_with("127.") || ep.starts_with("localhost")
}

/// Перестраивает модель профилей (вкладка Remote) и кандидатов попапа.
/// Local Host присутствует только если реально обнаружен локальный конфиг/демон.
fn refresh_profiles(ui: &MainWindow) {
    let cfg = config_lock().lock().unwrap().clone();
    let active = active_rpc_url();
    let mut items: Vec<ProfileItem> = Vec::new();
    let mut displays: Vec<SharedString> = Vec::new();

    let local_detected = {
        let cands = config::detect_rpc_candidates();
        let has_real = cands.iter().any(|c| is_local_url(&c.url) && c.source != "fallback");
        let active_is_local = !active.is_empty() && is_local_url(&active);
        has_real || active_is_local
    };

    if local_detected {
        let (ep, url) = if !active.is_empty() && is_local_url(&active) {
            (url_endpoint(&active), active.clone())
        } else {
            ("127.0.0.1:9091".to_string(), "http://127.0.0.1:9091/transmission/rpc".to_string())
        };
        items.push(ProfileItem {
            name: "Local Host".into(),
            host: ep.split(':').next().unwrap_or("").into(),
            endpoint: ep.clone().into(),
            url: url.into(),
            active: !active.is_empty() && url_endpoint(&active) == ep,
            is_custom: false,
        });
        displays.push(ep.into());
    }

    for h in &cfg.custom_hosts {
        let ep = url_endpoint(&h.url);
        if items.iter().any(|i| i.endpoint == ep) { continue; }
        items.push(ProfileItem {
            name: h.name.clone().into(),
            host: ep.split(':').next().unwrap_or("").into(),
            endpoint: ep.clone().into(),
            url: h.url.clone().into(),
            active: !active.is_empty() && url_endpoint(&active) == ep,
            is_custom: true,
        });
        displays.push(ep.into());
    }

    ui.set_profiles(ModelRc::from(Rc::new(VecModel::from(items))));
    ui.set_rpc_candidates(ModelRc::from(Rc::new(VecModel::from(displays))));
    ui.set_is_local_config_detected(local_detected);
    ui.set_local_host_active(
        active.is_empty() || is_local_url(&active),
    );
}

/// Единицы скорости: true = биты (kbit/s), false = байты (KB/s)
static SPEED_IN_BITS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Единицы трафика: true = показывать ТБ для больших объёмов
static TRAFFIC_IN_TB: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn fmt_speed(bps: i64) -> SharedString {
    if SPEED_IN_BITS.load(std::sync::atomic::Ordering::Relaxed) {
        let bits = bps * 8;
        return match bits {
            b if b <= 0            => "—".into(),
            b if b < 1_000         => format!("{b} bit/s").into(),
            b if b < 1_000_000     => format!("{:.1} kbit/s", b as f64 / 1_000.0).into(),
            b                      => format!("{:.1} Mbit/s", b as f64 / 1_000_000.0).into(),
        };
    }
    match bps {
        b if b <= 0        => "—".into(),
        b if b < 1_024     => format!("{b} B/s").into(),
        b if b < 1_048_576 => format!("{:.1} KB/s", b as f64 / 1_024.0).into(),
        b                  => format!("{:.1} MB/s", b as f64 / 1_048_576.0).into(),
    }
}

fn fmt_bytes(bytes: i64) -> SharedString {
    if TRAFFIC_IN_TB.load(std::sync::atomic::Ordering::Relaxed) {
        if bytes >= 1_099_511_627_776 {
            return format!("{:.2} TB", bytes as f64 / 1_099_511_627_776.0).into();
        }
    }
    match bytes {
        b if b <= 0            => "0 B".into(),
        b if b < 1_048_576     => format!("{:.1} KB", b as f64 / 1_024.0).into(),
        b if b < 1_073_741_824 => format!("{:.1} MB", b as f64 / 1_048_576.0).into(),
        b                      => format!("{:.2} GB", b as f64 / 1_073_741_824.0).into(),
    }
}

fn fmt_ratio(r: f64) -> SharedString {
    if r <= 0.0 { "—".into() } else { format!("{r:.2}").into() }
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Глобальный доступ к конфигу из любых потоков/хендлеров.
/// Единственный источник истины: main() инициализирует из app_config::load().
/// DaemonSettings из свойств UI (для автосохранения настроек)
fn build_daemon_settings(ui: &MainWindow) -> DaemonSettings {
    let mut day_mask: i64 = 0;
    if ui.get_cfg_speed_day_mon() { day_mask |= 1; }
    if ui.get_cfg_speed_day_tue() { day_mask |= 2; }
    if ui.get_cfg_speed_day_wed() { day_mask |= 4; }
    if ui.get_cfg_speed_day_thu() { day_mask |= 8; }
    if ui.get_cfg_speed_day_fri() { day_mask |= 16; }
    if ui.get_cfg_speed_day_sat() { day_mask |= 32; }
    if ui.get_cfg_speed_day_sun() { day_mask |= 64; }

    let s = DaemonSettings {
        speed_limit_up_enabled: ui.get_cfg_speed_up_enabled(),
        speed_limit_up: ui.get_cfg_speed_up_kbs() as i64 * 125, // Мбит/с → KB/s (×1000/8)
        speed_limit_down_enabled: ui.get_cfg_speed_down_enabled(),
        speed_limit_down: ui.get_cfg_speed_down_kbs() as i64 * 125,
        alt_speed_enabled: ui.get_cfg_alt_speed_enabled(),
        alt_speed_up: ui.get_cfg_alt_speed_up_kbs() as i64 * 125,
        alt_speed_down: ui.get_cfg_alt_speed_down_kbs() as i64 * 125,
        alt_speed_time_enabled: ui.get_cfg_alt_speed_schedule(),
        alt_speed_time_begin: ui.get_cfg_alt_speed_time_begin() as i64,
        alt_speed_time_end: ui.get_cfg_alt_speed_time_end() as i64,
        alt_speed_time_day: day_mask,
        download_dir: ui.get_cfg_dl_dir().to_string(),
        download_queue_enabled: ui.get_cfg_dl_queue_enabled(),
        download_queue_size: ui.get_cfg_dl_queue_max() as i64,
        queue_stalled_enabled: ui.get_cfg_dl_seed_ratio_limit_min() > 0,
        queue_stalled_minutes: ui.get_cfg_dl_seed_ratio_limit_min() as i64,
        start_added_torrents: ui.get_cfg_dl_start_added(),
        trash_original_torrent_files: ui.get_cfg_dl_trash_torrent(),
        rename_partial_files: ui.get_cfg_dl_part_ext(),
        incomplete_dir_enabled: ui.get_cfg_dl_incomplete_dir_enabled(),
        incomplete_dir: ui.get_cfg_dl_incomplete_dir().to_string(),
        script_torrent_done_enabled: ui.get_cfg_dl_done_script_enabled(),
        script_torrent_done_filename: ui.get_cfg_dl_done_script().to_string(),
        script_torrent_done_seeding_enabled: ui.get_cfg_seed_done_script_enabled(),
        script_torrent_done_seeding_filename: ui.get_cfg_seed_done_script().to_string(),
        seed_ratio_limited: ui.get_cfg_seed_ratio_enabled(),
        seed_ratio_limit: ui.get_cfg_seed_ratio() as f64 / 100.0,
        idle_seeding_limit_enabled: ui.get_cfg_seed_idle_enabled(),
        idle_seeding_limit: ui.get_cfg_seed_idle_min() as i64,
        peer_port: ui.get_cfg_net_port() as i64,
        peer_port_random_on_start: ui.get_cfg_net_random_port(),
        port_forwarding_enabled: ui.get_cfg_net_port_forward(),
        peer_limit_per_torrent: ui.get_cfg_net_max_peers_torrent() as i64,
        peer_limit_global: ui.get_cfg_net_max_peers_total() as i64,
        utp_enabled: ui.get_cfg_net_utp(),
        pex_enabled: ui.get_cfg_net_pex(),
        dht_enabled: ui.get_cfg_net_dht(),
        lpd_enabled: ui.get_cfg_net_lpd(),
        default_trackers: ui.get_cfg_net_default_trackers().to_string(),
        encryption: ui.get_cfg_priv_encryption() as i64,
        blocklist_enabled: ui.get_cfg_priv_blocklist_enabled(),
        blocklist_url: ui.get_cfg_priv_blocklist_url().to_string(),
        watch_dir_enabled: ui.get_cfg_dl_watch_dir_enabled(),
        watch_dir: ui.get_cfg_dl_watch_dir().to_string(),
        rpc_enabled: ui.get_cfg_remote_enabled(),
        rpc_port: ui.get_cfg_remote_port() as i64,
        rpc_authentication_required: ui.get_cfg_remote_auth(),
        rpc_username: ui.get_cfg_remote_username().to_string(),
        rpc_password: ui.get_cfg_remote_password().to_string(),
        rpc_whitelist_enabled: ui.get_cfg_remote_whitelist_enabled(),
        rpc_whitelist: ui.get_cfg_remote_whitelist().to_string(),
    };
    s
}


/// AppConfig из свойств UI + сохранение (возвращает blocklist флаги)
fn apply_app_config_from_ui(ui: &MainWindow) -> (bool, u64) {
    let mut g = config_lock().lock().unwrap();
    g.autostart = ui.get_cfg_autostart();
    g.suspend_on_hide = ui.get_cfg_suspend();
    g.start_minimized = ui.get_cfg_start_minimized();
    g.delete_torrent_after_add = ui.get_cfg_delete_torrent();
    g.refresh_interval_secs = ui.get_cfg_refresh_interval() as u64;
    g.on_close_action = ui.get_cfg_on_close() as u32;
    g.notify_on_add = ui.get_cfg_notify_add();
    g.notify_on_complete = ui.get_cfg_notify_complete();
    g.notify_sound = ui.get_cfg_notify_sound();
    g.theme = ui.get_cfg_theme() as u32;
    g.tb_add = ui.get_cfg_tb_add();
    g.tb_magnet = ui.get_cfg_tb_magnet();
    g.tb_create = ui.get_cfg_tb_create();
    g.tb_rehash = ui.get_cfg_tb_rehash();
    g.tb_start_sel = ui.get_cfg_tb_start_sel();
    g.tb_pause_sel = ui.get_cfg_tb_pause_sel();
    g.tb_start_all = ui.get_cfg_tb_start_all();
    g.tb_pause_all = ui.get_cfg_tb_pause_all();
    g.lp_status = ui.get_cfg_lp_status();
    g.lp_disks = ui.get_cfg_lp_disks();
    g.lp_trackers = ui.get_cfg_lp_trackers();
    g.lp_webtorrents = ui.get_cfg_lp_webtorrents();
    g.lp_tags = ui.get_cfg_lp_tags();
    g.lp_created = ui.get_cfg_lp_created();
    g.dl_show_dialog = ui.get_cfg_dl_show_dialog();
    g.blocklist_auto_update = ui.get_cfg_priv_blocklist_auto_update();
    g.speed_in_bits = ui.get_speed_in_bits();
    g.traffic_in_tb = ui.get_traffic_in_tb();
    let r = (g.blocklist_auto_update, g.blocklist_last_update);
    app_config::save(&*g);
    r
}


fn config_lock() -> &'static std::sync::Mutex<app_config::AppConfig> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<app_config::AppConfig>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(app_config::AppConfig::default()))
}

#[derive(Debug)]
enum Command {
    StartTorrent(i64),
    StopTorrent(i64),
    StartAll,
    StopAll,
    StopDisk(Vec<i64>),
    StartDisk(Vec<i64>),
    RecheckTorrent(i64),
    SetLocation(i64, String, bool), // id, new_location, do_move
    CreateTorrent(String, Vec<String>),  // path, trackers
    RemoveTorrent(i64, bool),
    AddTorrentUrl(String, Option<String>),
    AddTorrentFile(String, Option<String>, bool), // path, download_dir, delete_after
    SwitchRpc(String),
    SaveDaemonSettings(DaemonSettings),
    LoadDaemonSettings,
    UpdateBlocklist,
    PortTest,
}

struct Update {
    torrents: Vec<rpc::RawTorrent>,
    stats: RpcStats,
}

enum SettingsResult {
    Loaded(DaemonSettings),
    Saved,
    Error(String),
}

// ── Async backend ─────────────────────────────────────────────────────────────

async fn backend_task(
    mut client: TransmissionClient,
    mut cmd_rx: mpsc::UnboundedReceiver<Command>,
    update_tx: std::sync::mpsc::SyncSender<Update>,
    status_tx: std::sync::mpsc::SyncSender<String>,
    settings_tx: std::sync::mpsc::SyncSender<SettingsResult>,
    ui_weak: slint::Weak<MainWindow>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut fail_count = 0u32;
    // Оверлей «переключение хоста» активен до первого успешного тика
    let mut switching = false;

    // Определяем версию RPC (dash vs snake_case методы)
    client.detect_rpc_version().await;

    // Кэш всех торрентов — обновляем только изменившиеся
    let mut cache: std::collections::HashMap<i64, rpc::RawTorrent> = std::collections::HashMap::with_capacity(256);
    let mut initialized = false;

    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                let res = match cmd {
                    Command::StartTorrent(id)     => client.start_torrent(id).await,
                    Command::StopTorrent(id)      => client.stop_torrent(id).await,
                    Command::StartAll             => client.start_all().await,
                    Command::StopAll              => client.stop_all().await,
                    Command::StopDisk(ids)        => client.stop_torrents(&ids).await,
                    Command::StartDisk(ids)       => client.start_torrents(&ids).await,
                    Command::RecheckTorrent(id)   => client.recheck_torrent(id).await,
                    Command::CreateTorrent(path, trackers) => {
                        // transmission-create CLI (torrent-create RPC не существует в 4.1.x)
                        let name = std::path::Path::new(&path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("created");
                        let save_dir = {
                            let home = std::env::var("HOME").unwrap_or_default();
                            let desktop = format!("{home}/Desktop");
                            std::fs::create_dir_all(&desktop).ok();
                            desktop
                        };
                        let out = format!("{save_dir}/{name}.torrent");

                        // Собираем аргументы: -o output [-t tracker …] path
                        let mut args = vec![
                            "-o".to_string(), out.clone(),
                        ];
                        for t in &trackers {
                            args.push("-t".to_string());
                            args.push(t.clone());
                        }
                        args.push(path.clone());

                        eprintln!("[create] running: transmission-create {}", args.join(" "));
                        match std::process::Command::new("transmission-create")
                            .args(&args)
                            .output()
                        {
                            Ok(output) => {
                                let stdout = String::from_utf8_lossy(&output.stdout);
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                if !stdout.is_empty() { eprintln!("[create] stdout: {stdout}"); }
                                if !stderr.is_empty() { eprintln!("[create] stderr: {stderr}"); }
                                if output.status.success() {
                                    eprintln!("[create] Saved: {out}");
                                    let _ = status_tx.try_send(format!("Torrent saved: {out}"));
                                    Ok(())
                                } else {
                                    Err(anyhow::anyhow!(
                                        "transmission-create failed ({}): {stderr}",
                                        output.status
                                    ))
                                }
                            }
                            Err(e) => Err(anyhow::anyhow!(
                                "transmission-create not found: {e}. Install transmission-cli package."
                            )),
                        }
                    },
                    Command::SetLocation(id, loc, mv) => client.set_location(id, &loc, mv).await,
                    Command::RemoveTorrent(id, d) => client.remove_torrent(id, d).await,
                    Command::AddTorrentUrl(u, dir)          => client.add_torrent_url(&u, dir.as_deref()).await,
                    Command::AddTorrentFile(p, dir, del) => {
                        let res = client.add_torrent_file(&p, dir.as_deref()).await;
                        if res.is_ok() && del {
                            if let Err(e) = std::fs::remove_file(&p) {
                                eprintln!("[add] Could not delete .torrent file: {e}");
                            } else {
                                eprintln!("[add] Deleted .torrent file: {p}");
                            }
                        }
                        res
                    },
                Command::SwitchRpc(url) => {
                    eprintln!("[rpc] Switching to {url}");
                    client = TransmissionClient::with_auth(url, client.user.clone(), client.password.clone());
                    cache.clear(); initialized = false; fail_count = 0;
                    switching = true;
                    let _ = status_tx.try_send("Connecting…".into());
                    client.detect_rpc_version().await;
                    interval.reset();
                    continue;
                },
                Command::SaveDaemonSettings(s) => {
                    match client.session_set_settings(&s).await {
                        Ok(()) => {
                            eprintln!("[settings] Daemon settings saved OK");
                            let _ = settings_tx.try_send(SettingsResult::Saved);
                            let _ = status_tx.try_send("Settings saved".into());
                        },
                        Err(e) => {
                            eprintln!("[settings] Save error: {e}");
                            let _ = settings_tx.try_send(SettingsResult::Error(e.to_string()));
                        },
                    }
                    interval.reset();
                    continue;
                },
                Command::LoadDaemonSettings => {
                    match client.session_get_settings().await {
                        Ok(s) => {
                            eprintln!("[settings] Daemon settings loaded OK");
                            let _ = settings_tx.try_send(SettingsResult::Loaded(s));
                        },
                        Err(e) => {
                            eprintln!("[settings] Load error: {e}");
                            let _ = settings_tx.try_send(SettingsResult::Error(e.to_string()));
                        },
                    }
                    continue;
                },
                Command::PortTest => {
                    let cl = client.clone();
                    let ui_w = ui_weak.clone();
                    tokio::spawn(async move {
                        match cl.port_test().await {
                            Ok(open) => {
                                eprintln!("[port-test] open={open}");
                                let _ = slint::invoke_from_event_loop(move || {
                                    if let Some(ui) = ui_w.upgrade() {
                                        ui.set_port_test_state(if open { 2 } else { 3 });
                                    }
                                });
                            },
                            Err(e) => {
                                eprintln!("[port-test] failed: {e}");
                                let _ = slint::invoke_from_event_loop(move || {
                                    if let Some(ui) = ui_w.upgrade() {
                                        ui.set_port_test_state(0);
                                    }
                                });
                            },
                        }
                    });
                    continue;
                },
                Command::UpdateBlocklist => {
                    // Может занять минуты (демон качает список) — не блокируем цикл
                    let cl = client.clone();
                    let st = status_tx.clone();
                    let ui_w = ui_weak.clone();
                    tokio::spawn(async move {
                        let _ = st.try_send("Updating blocklist…".into());
                        match cl.blocklist_update().await {
                            Ok(n) => {
                                let _ = st.try_send(format!("Blocklist updated: {n} entries"));
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs()).unwrap_or(0);
                                if let Ok(mut cfg) = config_lock().lock() {
                                    cfg.blocklist_entries = n;
                                    cfg.blocklist_last_update = now;
                                    app_config::save(&cfg);
                                }
                                let _ = slint::invoke_from_event_loop(move || {
                                    if let Some(ui) = ui_w.upgrade() {
                                        ui.set_cfg_priv_blocklist_entries(n as i32);
                                    }
                                });
                            },
                            Err(e) => {
                                let _ = st.try_send(format!("Blocklist update failed: {e}"));
                            },
                        }
                    });
                    continue;
                },
                };
                if let Err(e) = res {
                    eprintln!("[cmd] Error: {e}");
                    let _ = status_tx.try_send(format!("Error: {e}"));
                }
                interval.reset();
            }
            _ = interval.tick() => {
                // Первый раз — полный список. Далее — только delta
                let tor_res = if !initialized {
                    client.get_all_torrents().await.map(|list| (list, vec![]))
                } else {
                    client.get_recently_active().await
                };
                let stat_res = client.get_session_stats().await;

                match tor_res {
                    Ok((changed, removed)) => {
                        fail_count = 0;

                        // Применяем delta к кэшу
                        for t in changed { cache.insert(t.id, t); }
                        for id in removed { cache.remove(&id); }
                        initialized = true;

                        let list: Vec<rpc::RawTorrent> = cache.values().cloned().collect();
                        let n = list.len();
                        let active = list.iter()
                            .filter(|t| t.rate_upload > 0 || t.rate_download > 0)
                            .count();
                        eprintln!("[rpc] OK: {n} torrents, {active} active");
                        let _ = status_tx.try_send(format!("Connected — {n} torrent(s)"));
                        // Успешный тик после смены хоста — гасим оверлей «переключение»
                        if switching {
                            switching = false;
                            let ui_w = ui_weak.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(ui) = ui_w.upgrade() {
                                    ui.set_is_switching_host(false);
                                }
                            });
                        }
                        // Ежедневный автоапдейт blocklist (если включён)
                        if let Ok(cfg) = config_lock().lock() {
                            if cfg.blocklist_auto_update {
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs()).unwrap_or(0);
                                if now.saturating_sub(cfg.blocklist_last_update) > 86_400 {
                                    drop(cfg);
                                    let cl = client.clone();
                                    let st = status_tx.clone();
                                    let ui_w = ui_weak.clone();
                                    tokio::spawn(async move {
                                        eprintln!("[blocklist] auto-update started");
                                        match cl.blocklist_update().await {
                                            Ok(cnt) => {
                                                let _ = st.try_send(format!("Blocklist updated: {cnt} entries"));
                                                let now = std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .map(|d| d.as_secs()).unwrap_or(0);
                                                if let Ok(mut cfg) = config_lock().lock() {
                                                    cfg.blocklist_entries = cnt;
                                                    cfg.blocklist_last_update = now;
                                                    app_config::save(&*cfg);
                                                }
                                                let _ = slint::invoke_from_event_loop(move || {
                                                    if let Some(ui) = ui_w.upgrade() {
                                                        ui.set_cfg_priv_blocklist_entries(cnt as i32);
                                                    }
                                                });
                                            },
                                            Err(e) => eprintln!("[blocklist] auto-update failed: {e}"),
                                        }
                                    });
                                }
                            }
                        }
                        let mut stats = stat_res.unwrap_or_default();
                        stats.active_count = active as i64;
                        let _ = update_tx.try_send(Update { torrents: list, stats });
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        let is_fatal = msg.contains("Connection refused")
                            || msg.contains("os error 111");
                        let is_transient = msg.contains("Body read")
                            || msg.contains("error decoding")
                            || msg.contains("connection reset")
                            || msg.contains("unexpected EOF")
                            || msg.contains("broken pipe");

                        if is_transient {
                            // Временная ошибка под нагрузкой — не считаем, просто ждём
                            eprintln!("[rpc] transient error (ignored): {}", &msg[..msg.len().min(80)]);
                            initialized = false; // перезапросим полный список
                            cache.clear();
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            interval.reset();
                            continue;
                        }

                        fail_count += 1;
                        initialized = false;
                        cache.clear();
                        eprintln!("[rpc] error #{fail_count}: {}", &msg[..msg.len().min(80)]);
                        if is_fatal {
                            let _ = status_tx.try_send("Connecting…".into());
                        } else {
                            let _ = status_tx.try_send(format!("Disconnected: {msg}"));
                        }
                        // Backoff: 5s, 10s, 20s, 30s
                        if fail_count > 1 {
                            let wait = Duration::from_secs(match fail_count {
                                2 =>  5,
                                3 => 10,
                                4 => 20,
                                _ => 30,
                            });
                            tokio::time::sleep(wait).await;
                            interval.reset();
                        }
                        // После 5 настоящих ошибок — выходим
                        if fail_count >= 5 {
                            eprintln!("[rpc] Daemon lost after {} failures, quitting", fail_count);
                            let _ = status_tx.try_send("Daemon lost — closing".into());
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            slint::quit_event_loop().ok();
                            return;
                        }
                    }
                }
            }
        }
    }
}

// ── Model diff ────────────────────────────────────────────────────────────────

fn torrent_sort_key(t: &rpc::RawTorrent) -> (u8, i64, String) {
    let speed = t.rate_upload + t.rate_download;
    let priority = if t.is_error() {
        6 // ошибки — в самый конец
    } else if speed > 0 {
        0 // активные — наверху
    } else {
        match t.status {
            3 | 5 => 1, // Queued
            2     => 2, // Checking
            4 | 6 => 3, // Stalled
            0     => 4, // Stopped
            _     => 5,
        }
    };
    (priority, -speed, t.name.to_lowercase())
}

fn apply_torrent_update(model: &Rc<VecModel<TorrentItem>>, torrents: &[&rpc::RawTorrent]) {
    let mut sorted: Vec<&rpc::RawTorrent> = torrents.to_vec();
    sorted.sort_by_key(|t| torrent_sort_key(t));

    let old_len = model.row_count();
    let new_len = sorted.len();

    for (i, t) in sorted.iter().enumerate() {
        let new = TorrentItem {
            id:           t.id as i32,
            name:         t.name.as_str().into(),
            status_label: t.status_label().into(),
            progress:     t.percent_done as f32,
            down_speed:   fmt_speed(t.rate_download),
            up_speed:     fmt_speed(t.rate_upload),
            is_paused:    t.is_paused(),
            is_error:     t.is_error(),
            is_checking:  t.status == 1 || t.status == 2,
            download_dir: t.download_dir.as_str().into(),
            error_string: t.error_string.as_str().into(),
        };
        if i < old_len {
            let old = model.row_data(i).unwrap();
            let changed = old.id           != new.id
                || old.status_label != new.status_label
                || old.is_paused    != new.is_paused
                || old.is_error     != new.is_error
                || old.is_checking  != new.is_checking
                || old.error_string != new.error_string
                || (old.progress - new.progress).abs() > 0.001
                || old.down_speed   != new.down_speed
                || old.up_speed     != new.up_speed
                || old.name         != new.name;
            if changed { model.set_row_data(i, new); }
        } else {
            model.push(new);
        }
    }
    for _ in new_len..old_len { model.remove(new_len); }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // TLS: reqwest собран с rustls-no-provider — явно ставим ring (чистая сборка без cmake)
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Игнорируем SIGHUP — закрытие терминала не убивает приложение
    unsafe { libc::signal(libc::SIGHUP, libc::SIG_IGN) };

    // Режим стража — запущены с --watcher <pid>, не инициализируем UI
    if suspend::maybe_run_as_watcher(&args[1..]) {
        return Ok(());
    }

    // Торрент-файл переданный из проводника: transmission-remote-slint /path/to/file.torrent
    let pending_torrent: Option<String> = args[1..].iter()
        .find(|a| !a.starts_with("--") && a.ends_with(".torrent"))
        .cloned();
    if let Some(ref p) = pending_torrent {
        eprintln!("[open] Torrent file from args: {p}");
    }

    // ── Single instance ────────────────────────────────────────────────────────
    // Если уже запущен другой экземпляр — передаём ему файл и выходим
    let listener = match single_instance::acquire(pending_torrent.as_deref()) {
        single_instance::InstanceRole::Secondary => return Ok(()),
        single_instance::InstanceRole::Primary(l) => l,
    };
    // Лог в /tmp (обычно tmpfs): запись в ОЗУ бережёт SSD, логи не копятся месяцами.
    // Суффикс UID — защита от коллизий/симлинк-атак в world-writable /tmp.
    {
        let log_path = format!("/tmp/transmission-remote-slint-{}.log", unsafe { libc::getuid() });
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true).append(true).open(&log_path)
        {
            use std::io::Write;
            // Дата/время в начале сессии
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let _ = writeln!(&file, "\n=== Started at unix={ts} ===");
            // Дублируем stderr в файл через pipe trick (только Unix)
            // Используем простой подход: dup2 fd лог-файла на stderr
            use std::os::unix::io::IntoRawFd;
            let fd = file.into_raw_fd();
            unsafe {
                libc::dup2(fd, libc::STDERR_FILENO);
                libc::close(fd);
            }
            eprintln!("[log] Writing to {log_path}");
        }
    }

    // ── Panic hook — пишет стектрейс в лог ────────────────────────────────────
    std::panic::set_hook(Box::new(|info| {
        eprintln!("[PANIC] {info}");
        // Принудительно сбрасываем буфер stderr
        use std::io::Write;
        let _ = std::io::stderr().flush();
    }));

    apply_render_backend(&args[1..]);

    // Загружаем конфиг приложения (создаёт дефолт если нет)
    let app_cfg = app_config::load();
    *config_lock().lock().unwrap() = app_cfg.clone();
    app_config::install_icon();

    // ── Инициализируем язык ────────────────────────────────────────────────────
    i18n::init(&app_cfg.language);
    eprintln!("[i18n] language={}", app_cfg.language);

    eprintln!("[app_config] suspend_on_hide={} start_minimized={} refresh={}s autostart={}",
        app_cfg.suspend_on_hide, app_cfg.start_minimized,
        app_cfg.refresh_interval_secs, app_cfg.autostart);
    app_config::sync_autostart(app_cfg.autostart);

    let rt = std::sync::Arc::new(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?);

    // Каналы создаём до UI — статус-сообщения от daemon нужны сразу
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
    let (settings_tx, settings_rx) = std::sync::mpsc::sync_channel::<SettingsResult>(8);
    // update: буфер 1 — UI всегда берёт последнее состояние, промежуточные не нужны
    // буфер 8 — backend не блокируется, UI дренирует все и берёт последнее
    let (update_tx, update_rx) = std::sync::mpsc::sync_channel::<Update>(8);
    let (status_tx, status_rx) = std::sync::mpsc::sync_channel::<String>(32);

    // ── Конфиг + демон ────────────────────────────────────────────────────────
    let (daemon_handle, active_cfg, probe_result) = rt.block_on(async {
        daemon::ensure_daemon(&status_tx).await
    });
    eprintln!("[main] Using RPC: {}", active_cfg.url);

    let client = TransmissionClient::with_auth(
        active_cfg.url.clone(),
        active_cfg.user.clone(),
        active_cfg.password.clone(),
    );
    // backend_task запускается после создания UI (нужен ui.as_weak() для оверлея переключения)
    let client_for_task = client;

    // ── Трей ─────────────────────────────────────────────────────────────────
    let tray = if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_ok() {
        match tray::AppTray::build() {
            Ok(t) => {
                eprintln!("[tray] StatusNotifierItem registered");
                Some(t)
            }
            Err(e) => { eprintln!("[tray] Failed: {e}"); None }
        }
    } else {
        eprintln!("[tray] No D-Bus — tray unavailable");
        None
    };

    // ── UI ────────────────────────────────────────────────────────────────────
    let ui = MainWindow::new()?;
    ui.set_status_bar_text(probe_result.status_msg.as_str().into());
    ui.set_connected(probe_result.ok);
    ui.set_selected_language(app_cfg.language.as_str().into());

    // Новые настройки UI из конфига
    SPEED_IN_BITS.store(app_cfg.speed_in_bits, std::sync::atomic::Ordering::Relaxed);
    TRAFFIC_IN_TB.store(app_cfg.traffic_in_tb, std::sync::atomic::Ordering::Relaxed);
    ui.set_speed_in_bits(app_cfg.speed_in_bits);
    ui.set_traffic_in_tb(app_cfg.traffic_in_tb);
    ui.set_cfg_dl_show_dialog(app_cfg.dl_show_dialog);
    ui.set_cfg_priv_blocklist_auto_update(app_cfg.blocklist_auto_update);
    ui.set_cfg_priv_blocklist_entries(app_cfg.blocklist_entries as i32);

    // Запускаем асинхронный бэкенд (после создания UI)
    rt.spawn(backend_task(client_for_task, cmd_rx, update_tx, status_tx.clone(), settings_tx, ui.as_weak()));

    // RPC URL для отображения: "127.0.0.1:9091" → "localhost" если локальный
    fn rpc_display(url: &str) -> String {
        let host_port = url.trim_start_matches("http://").trim_start_matches("https://")
            .split('/').next().unwrap_or(url);
        if host_port.starts_with("127.") || host_port.starts_with("localhost") {
            "localhost".to_string()
        } else {
            host_port.to_string()
        }
    }
    ui.set_rpc_url(rpc_display(&active_cfg.url).into());

    // Список кандидатов для переключения
    let candidates = config::detect_rpc_candidates();
    let candidate_urls: Vec<slint::SharedString> = candidates.iter()
        .map(|c| rpc_display(&c.url).into())
        .collect::<std::collections::HashSet<_>>()  // дедупликация
        .into_iter().collect();
    ui.set_rpc_candidates(std::rc::Rc::new(slint::VecModel::from(candidate_urls)).into());

    // Активный URL для active-флагов профилей
    {
        *ACTIVE_RPC_URL.lock().unwrap() = Some(active_cfg.url.clone());
    }
    refresh_profiles(&ui);
    push_dialog_tr(&ui);
    
    // Обновляем UI переводы
    ui.set_tr_toolbar_open(i18n::toolbar_open().into());
    ui.set_tr_toolbar_magnet(i18n::toolbar_magnet().into());
    ui.set_tr_toolbar_create(i18n::toolbar_create().into());
    ui.set_tr_toolbar_rehash(i18n::toolbar_rehash().into());
    ui.set_tr_sidebar_status(i18n::sidebar_status().into());
    ui.set_tr_sidebar_all(i18n::sidebar_all().into());
    ui.set_tr_sidebar_downloading(i18n::sidebar_downloading().into());
    ui.set_tr_sidebar_seeding(i18n::sidebar_seeding().into());
    ui.set_tr_sidebar_completed(i18n::sidebar_completed().into());
    ui.set_tr_sidebar_stopped(i18n::sidebar_stopped().into());
    ui.set_tr_sidebar_active(i18n::sidebar_active().into());
    ui.set_tr_sidebar_inactive(i18n::sidebar_inactive().into());
    ui.set_tr_sidebar_checking(i18n::sidebar_checking().into());
    ui.set_tr_sidebar_error(i18n::sidebar_error().into());
    ui.set_tr_sidebar_disks(i18n::sidebar_disks().into());
    ui.set_tr_column_name(i18n::column_name().into());
    ui.set_tr_column_status(i18n::column_status().into());
    ui.set_tr_column_done(i18n::column_done().into());
    ui.set_tr_column_down(i18n::column_down().into());
    ui.set_tr_column_up(i18n::column_up().into());
    ui.set_tr_menu_start(i18n::menu_start().into());
    ui.set_tr_menu_pause(i18n::menu_pause().into());
    ui.set_tr_menu_recheck(i18n::menu_recheck().into());
    ui.set_tr_menu_open_folder(i18n::menu_open_folder().into());
    ui.set_tr_menu_set_location(i18n::menu_set_location().into());
    ui.set_tr_menu_remove(i18n::menu_remove().into());
    ui.set_tr_menu_delete(i18n::menu_delete().into());
    ui.set_tr_statusbar_dht(i18n::statusbar_dht().into());
    ui.set_tr_statusbar_conn(i18n::statusbar_conn().into());

    // Переводы диалога удаления
    ui.set_tr_dlg_remove_title(i18n::dlg_remove_confirm().into());
    ui.set_tr_dlg_remove_subtitle(i18n::dlg_remove_subtitle().into());
    ui.set_tr_dlg_delete_title(i18n::dlg_delete_confirm().into());
    ui.set_tr_dlg_delete_warning(i18n::dlg_delete_warning().into());
    ui.set_tr_dlg_irreversible(i18n::dlg_irreversible().into());
    ui.set_tr_dlg_confirm(i18n::dlg_ok().into());
    ui.set_tr_dlg_cancel(i18n::dlg_cancel().into());

    // Определяем диски
    eprintln!("[disks] Detecting physical disks...");
    let physical_disks = disks::detect_physical_disks();
    eprintln!("[disks] Building majmin map...");
    let majmin_map = std::sync::Arc::new(disks::build_majmin_map());
    eprintln!("[disks] majmin map: {} entries", majmin_map.len());
    let disk_model: Rc<VecModel<DiskItem>> = Rc::new(VecModel::default());
    for d in &physical_disks {
        eprintln!("[disks] Adding disk: {} = {}", d.label, d.dev);
        disk_model.push(DiskItem {
            label:         d.label.as_str().into(),
            kind:          "disk".into(),
            mountpoints:   d.dev.as_str().into(),
            torrent_count: 0,
        });
    }
    eprintln!("[disks] disk_model has {} rows", disk_model.row_count());
    ui.set_disk_groups(ModelRc::from(disk_model.clone()));
    eprintln!("[disks] set_disk_groups done");
    let physical_disks_arc = std::sync::Arc::new(physical_disks);

    let torrent_model: Rc<VecModel<TorrentItem>> = Rc::new(VecModel::default());
    ui.set_torrents(ModelRc::from(torrent_model.clone()));

    // Shared snapshot всех торрентов — обновляется slow timer, читается disk callbacks
    let torrent_snapshot: std::sync::Arc<std::sync::Mutex<Vec<rpc::RawTorrent>>> =
        std::sync::Arc::new(std::sync::Mutex::new(vec![]));

    // Callbacks
    { let tx = cmd_tx.clone(); ui.on_start_all(move || { let _ = tx.send(Command::StartAll); }); }
    { let tx = cmd_tx.clone(); ui.on_stop_all(move || { let _ = tx.send(Command::StopAll); }); }
    { let tx = cmd_tx.clone(); ui.on_start_torrent(move |id| { let _ = tx.send(Command::StartTorrent(id as i64)); }); }
    { let tx = cmd_tx.clone(); ui.on_stop_torrent(move |id| { let _ = tx.send(Command::StopTorrent(id as i64)); }); }
    { let tx = cmd_tx.clone(); ui.on_remove_torrent(move |id, del| { let _ = tx.send(Command::RemoveTorrent(id as i64, del)); }); }
    { let tx = cmd_tx.clone(); ui.on_recheck_torrent(move |id| { let _ = tx.send(Command::RecheckTorrent(id as i64)); }); }

    // Кэш последней папки для "Указать расположение"
    let last_location: std::sync::Arc<std::sync::Mutex<String>> =
        std::sync::Arc::new(std::sync::Mutex::new(String::new()));

    // request-set-location: XDG диалог выбора папки
    {
        let tx = cmd_tx.clone();
        let loc = last_location.clone();
        ui.on_request_set_location(move |id: i32, _name: slint::SharedString, current_dir: slint::SharedString| {
            let id = id as i64;
            let default_dir = current_dir.to_string();
            // Пробуем XDG диалог
            if let Ok(new_dir) = filepicker::pick_directory(&default_dir) {
                // Сохраняем для следующего раза
                *loc.lock().unwrap() = new_dir.clone();
                // Сразу перемещаем (do_move = true)
                eprintln!("[set-location] id={} → {} (move)", id, new_dir);
                let _ = tx.send(Command::SetLocation(id, new_dir, true));
            }
        });
    }

    // pick-context-menu-set-location: из контекстного меню → открыть диалог
    {
        let ui_weak = ui.as_weak();
        let loc = last_location.clone();
        ui.on_pick_context_menu_set_location(move |id: i32, name: slint::SharedString, current_dir: slint::SharedString| {
            if let Some(ui) = ui_weak.upgrade() {
                let loc_guard = loc.lock().unwrap();
                let path = if loc_guard.is_empty() {
                    current_dir.to_string()
                } else {
                    loc_guard.clone()
                };
                drop(loc_guard);

                ui.set_set_location_torrent_id(id);
                ui.set_set_location_torrent_name(name);
                ui.set_set_location_path(path.into());
                ui.set_set_location_do_move(true);
                ui.set_set_location_dialog_visible(true);
                ui.set_context_menu_visible(false);
            }
        });
    }

    // get-last-location: вернуть последнюю использованную папку
    {
        let loc = last_location.clone();
        ui.on_get_last_location(move || -> slint::SharedString {
            let loc_guard = loc.lock().unwrap();
            if loc_guard.is_empty() {
                "".into()
            } else {
                loc_guard.clone().into()
            }
        });
    }

    // Закрытие контекстного меню по клику вне
    {
        let ui_weak = ui.as_weak();
        ui.on_click_outside(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_context_menu_visible(false);
            }
        });
    }

    // set-location-xdg-dialog: XDG диалог → сразу действие (переместить или обновить путь)
    {
        let tx = cmd_tx.clone();
        let loc = last_location.clone();
        ui.on_set_location_xdg_dialog(move |_id: i32, _name: slint::SharedString, current_dir: slint::SharedString, do_move: bool| {
            let default_dir = current_dir.to_string();
            if let Ok(new_dir) = filepicker::pick_directory(&default_dir) {
                *loc.lock().unwrap() = new_dir.clone();
                eprintln!("[set-location-xdg] picked: {} (move={})", new_dir, do_move);
                let _ = tx.send(Command::SetLocation(_id as i64, new_dir, do_move));
            }
        });
    }

    // set-location-cmd: прямой вызов без диалога
    {
        let tx = cmd_tx.clone();
        let loc = last_location.clone();
        ui.on_set_location_cmd(move |id: i32, path: slint::SharedString, do_move: bool| {
            let id = id as i64;
            let path = path.to_string();
            if !do_move {
                *loc.lock().unwrap() = path.clone();
            }
            eprintln!("[set-location] id={} → {} (move={})", id, path, do_move);
            let _ = tx.send(Command::SetLocation(id, path, do_move));
        });
    }
    {
        let tx = cmd_tx.clone();
        ui.on_create_torrent(move |path, trackers| {
            let path = path.trim().to_string();
            if path.is_empty() { return; }
            // Разбираем трекеры по строкам
            let tr: Vec<String> = trackers.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            eprintln!("[create] Path: {path}, trackers: {}", tr.len());
            let _ = tx.send(Command::CreateTorrent(path, tr));
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.on_pick_create_path(move || {
            let ui2 = ui_weak.clone();
            std::thread::spawn(move || {
                match filepicker::pick_directory("") {
                    Ok(path) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui2.upgrade() {
                                ui.set_pending_create_path(path.into());
                                ui.set_create_dialog_visible(true);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("[create] pick dir: {e}");
                        // При отмене — закрываем диалог
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui2.upgrade() {
                                ui.set_create_dialog_visible(false);
                            }
                        });
                    }
                }
            });
        });
    }
    // Переключение единиц скорости (байты ↔ биты)
    {
        let ui_weak = ui.as_weak();
        ui.on_toggle_speed_units(move || {
            let new_val = {
                let mut cfg = config_lock().lock().unwrap();
                cfg.speed_in_bits = !cfg.speed_in_bits;
                cfg.speed_in_bits
            };
            SPEED_IN_BITS.store(new_val, std::sync::atomic::Ordering::Relaxed);
            app_config::save(&config_lock().lock().unwrap());
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_speed_in_bits(new_val);
            }
        });
    }

    // Переключение единиц трафика (ГБ ↔ ТБ)
    {
        let ui_weak = ui.as_weak();
        ui.on_toggle_traffic_units(move || {
            let new_val = {
                let mut cfg = config_lock().lock().unwrap();
                cfg.traffic_in_tb = !cfg.traffic_in_tb;
                cfg.traffic_in_tb
            };
            TRAFFIC_IN_TB.store(new_val, std::sync::atomic::Ordering::Relaxed);
            app_config::save(&config_lock().lock().unwrap());
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_traffic_in_tb(new_val);
            }
        });
    }

    // Выбор файла скрипта «после загрузки» (download done-script)
    {
        let ui_weak = ui.as_weak();
        ui.on_pick_dl_script_path(move || {
            let ui2 = ui_weak.clone();
            std::thread::spawn(move || {
                match filepicker::pick_file("Select done-script", "Scripts", "*.sh") {
                    Ok(path) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui2.upgrade() {
                                ui.set_cfg_dl_done_script(path.into());
                            }
                        });
                    }
                    Err(_) => {} // отмена выбора — ничего не делаем
                }
            });
        });
    }

    // Выбор файла скрипта «после раздачи» (seed done-script)
    {
        let ui_weak = ui.as_weak();
        ui.on_pick_seed_script_path(move || {
            let ui2 = ui_weak.clone();
            std::thread::spawn(move || {
                match filepicker::pick_file("Select seeding done-script", "Scripts", "*.sh") {
                    Ok(path) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui2.upgrade() {
                                ui.set_cfg_seed_done_script(path.into());
                            }
                        });
                    }
                    Err(_) => {} // отмена выбора — ничего не делаем
                }
            });
        });
    }

    {
        let ui_weak = ui.as_weak();
        let uw1 = ui_weak.clone();
        ui.on_pick_dl_dir(move || {
            let ui2 = uw1.clone();
            std::thread::spawn(move || {
                if let Ok(path) = filepicker::pick_directory("Select download directory") {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui2.upgrade() {
                            ui.set_cfg_dl_dir(path.into());
                        }
                    });
                }
            });
        });
        let uw2 = ui_weak.clone();
        ui.on_pick_incomplete_dir(move || {
            let ui2 = uw2.clone();
            std::thread::spawn(move || {
                if let Ok(path) = filepicker::pick_directory("Select incomplete directory") {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui2.upgrade() {
                            ui.set_cfg_dl_incomplete_dir(path.into());
                        }
                    });
                }
            });
        });
        let uw3 = ui_weak.clone();
        ui.on_pick_watch_dir(move || {
            let ui2 = uw3.clone();
            std::thread::spawn(move || {
                if let Ok(path) = filepicker::pick_directory("Select watch directory") {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui2.upgrade() {
                            ui.set_cfg_dl_watch_dir(path.into());
                        }
                    });
                }
            });
        });
    }

    {
        let tx = cmd_tx.clone();
        ui.on_add_torrent_url(move |url| {
            let s = url.trim().to_string();
            if !s.is_empty() { let _ = tx.send(Command::AddTorrentUrl(s, None)); }        });
    }
    // Magnet кнопка
    {
        ui.on_magnet_submitted(move |uri: slint::SharedString| {
            let uri = uri.to_string();
            let _ = std::process::Command::new("xdg-open")
                .arg(&uri)
                .spawn();
            eprintln!("[magnet] Opening: {}", uri);
        });
    }
    // Смена языка
    {
        let ui_weak = ui.as_weak();
        ui.on_language_changed(move |lang: slint::SharedString| {
            let lang = lang.to_string();
            eprintln!("[i18n] Language changed to: {}", lang);
            i18n::set_language(&lang);
            // Обновляем все строки диалогов/вкладок
            if let Some(ui) = ui_weak.upgrade() {
                push_dialog_tr(&ui);
            }
            // Сохраняем в конфиг (применится после перезапуска)
            {
                let mut cfg = config_lock().lock().unwrap();
                cfg.language = lang.clone();
                app_config::save(&*cfg);
            }
            eprintln!("[i18n] Config saved. Locale set to: {}", i18n::get_language());
            
            // Обновляем UI переводы
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_tr_toolbar_open(i18n::toolbar_open().into());
                ui.set_tr_toolbar_magnet(i18n::toolbar_magnet().into());
                ui.set_tr_toolbar_create(i18n::toolbar_create().into());
                ui.set_tr_toolbar_rehash(i18n::toolbar_rehash().into());
                ui.set_tr_sidebar_status(i18n::sidebar_status().into());
                ui.set_tr_sidebar_all(i18n::sidebar_all().into());
                ui.set_tr_sidebar_downloading(i18n::sidebar_downloading().into());
                ui.set_tr_sidebar_seeding(i18n::sidebar_seeding().into());
                ui.set_tr_sidebar_completed(i18n::sidebar_completed().into());
                ui.set_tr_sidebar_stopped(i18n::sidebar_stopped().into());
                ui.set_tr_sidebar_active(i18n::sidebar_active().into());
                ui.set_tr_sidebar_inactive(i18n::sidebar_inactive().into());
                ui.set_tr_sidebar_checking(i18n::sidebar_checking().into());
                ui.set_tr_sidebar_error(i18n::sidebar_error().into());
                ui.set_tr_sidebar_disks(i18n::sidebar_disks().into());
                ui.set_tr_column_name(i18n::column_name().into());
                ui.set_tr_column_status(i18n::column_status().into());
                ui.set_tr_column_done(i18n::column_done().into());
                ui.set_tr_column_down(i18n::column_down().into());
                ui.set_tr_column_up(i18n::column_up().into());
                ui.set_tr_menu_start(i18n::menu_start().into());
                ui.set_tr_menu_pause(i18n::menu_pause().into());
                ui.set_tr_menu_recheck(i18n::menu_recheck().into());
                ui.set_tr_menu_open_folder(i18n::menu_open_folder().into());
                ui.set_tr_menu_set_location(i18n::menu_set_location().into());
                ui.set_tr_menu_remove(i18n::menu_remove().into());
                ui.set_tr_menu_delete(i18n::menu_delete().into());
                ui.set_tr_statusbar_dht(i18n::statusbar_dht().into());
                ui.set_tr_statusbar_conn(i18n::statusbar_conn().into());
                eprintln!("[i18n] UI translations updated.");
            }
        });
    }
    // Rehash кнопка — проверка торрентов с hash error (error == 1)
    {
        let tx = cmd_tx.clone();
        let snap = torrent_snapshot.clone();
        ui.on_rehash_errors(move || {
            let snapshot = snap.lock().unwrap();
            // Только торренты с hash error (error == 1)
            for t in snapshot.iter() {
                if t.error == 1 {
                    eprintln!("[rehash] Rechecking torrent {}: {}", t.id, t.name);
                    let _ = tx.send(Command::RecheckTorrent(t.id));
                }
            }
        });
    }
    // Кнопка Открыть — xdg-open для .torrent файла
    {
        let ui_weak = ui.as_weak();
        ui.on_pick_torrent_file(move || {
            let ui2 = ui_weak.clone();
            std::thread::spawn(move || {
                // Выбрать .torrent файл и открыть через xdg-open
                let path = match filepicker::pick_torrent_file() {
                    Ok(p) => p,
                    Err(e) => {
                        let msg = format!("File picker: {e}");
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui2.upgrade() { ui.set_status_bar_text(msg.into()); }
                        });
                        return;
                    }
                };
                // Открыть через xdg-open
                let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
                eprintln!("[open] xdg-open {}", path);
            });
        });
    }
    {
        ui.on_open_folder(move |dir| {
            let d = dir.to_string();
            std::thread::spawn(move || {
                eprintln!("[open_folder] xdg-open {d}");
                let _ = std::process::Command::new("xdg-open").arg(&d).spawn();
            });
        });
    }
    {
        let ui_weak = ui.as_weak();
        let suspend = app_cfg.suspend_on_hide;
        let started_at = std::time::Instant::now();
        ui.on_do_minimize_tray(move || {
            eprintln!("[minimize] called at +{}ms, suspend={}", started_at.elapsed().as_millis(), suspend);
            if let Some(ui) = ui_weak.upgrade() { ui.window().hide().ok(); }
            if suspend && started_at.elapsed().as_secs() >= 3 {
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    suspend::suspend_self();
                });
            } else if suspend {
                eprintln!("[minimize] suspend skipped — too early after start");
            }
        });
    }
    // daemon_handle и cfg перемещаем в Arc для доступа из нескольких замыканий
    let _handle_arc = std::sync::Arc::new(std::sync::Mutex::new(Some(daemon_handle)));
    let cfg_arc    = std::sync::Arc::new(active_cfg.clone());

    // do_quit: анимация закрытия → SIGTERM демону → GUI закрывается ПОСЛЕ смерти демона
    let quitting = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let do_quit: std::sync::Arc<dyn Fn() + Send + Sync> = {
        let ca = cfg_arc.clone();
        let rt_arc = rt.clone();
        let ui_weak0 = ui.as_weak();
        let quitting0 = quitting.clone();
        std::sync::Arc::new(move || {
            // one-shot: повторные вызовы (трей + диалог) игнорируем
            if quitting0.swap(true, std::sync::atomic::Ordering::SeqCst) { return; }
            let ca2 = ca.clone();
            let rt2 = rt_arc.clone();
            let ui_w = ui_weak0.clone();
            std::thread::spawn(move || {
                // 1) Снимок статистики, пока RPC жив: что уйдёт трекерам.
                //    stopped-announce несёт per-tier up/down с последнего stop —
                //    RPC этого не отдаёт, поэтому показываем честное: статусы,
                //    leftUntilComplete и хосты трекеров (адресаты announce).
                let (summary, lines) = {
                    let cfg = ca2.clone();
                    rt2.block_on(async move {
                        match TransmissionClient::with_auth(
                            cfg.url.clone(), cfg.user.clone(), cfg.password.clone())
                            .close_summary().await
                        {
                            Ok(s) => {
                                let lines: Vec<SharedString> = s.hosts.iter().map(|h| {
                                    SharedString::from(format!("→ {h}"))
                                }).collect();
                                (s, lines)
                            }
                            Err(e) => {
                                eprintln!("[quit] close_summary failed: {e}");
                                (rpc::CloseSummary::default(), Vec::new())
                            }
                        }
                    })
                };
                eprintln!("[quit] torrents: {} (seed {} / leech {}), trackers: {}, left: {}",
                    summary.running, summary.seeding, summary.downloading,
                    summary.trackers, fmt_bytes(summary.left_bytes));
                // 2) Показываем анимацию (окно НЕ прячем — GUI живёт до смерти демона)
                let n_tr = summary.trackers as i32;
                let s_left: SharedString = fmt_bytes(summary.left_bytes);
                let ui_w1 = ui_w.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_w1.upgrade() {
                        ui.set_close_anim_trackers(n_tr);
                        ui.set_close_anim_left(s_left);
                        let model: slint::ModelRc<SharedString> =
                            std::rc::Rc::new(slint::VecModel::from(lines)).into();
                        ui.set_close_anim_ticker(model);
                        ui.set_close_anim_visible(true);
                    }
                });
                std::thread::sleep(Duration::from_millis(350)); // даём диалогу отрисоваться
                // 3) Просим демона завершиться: SIGTERM → stopped-announce трекерам
                daemon::shutdown_initiate(&ca2);
                // 4) Ждём завершения процесса демона (замер на 593 трекерах: ~16с)
                let deadline = std::time::Instant::now() + Duration::from_secs(20);
                let mut done = false;
                while std::time::Instant::now() < deadline {
                    if !daemon::daemon_alive(&ca2) { done = true; break; }
                    std::thread::sleep(Duration::from_millis(200));
                }
                if done {
                    eprintln!("[quit] daemon exited cleanly");
                    let ui_w2 = ui_w.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_w2.upgrade() { ui.set_close_anim_phase(1); }
                    });
                    std::thread::sleep(Duration::from_millis(400));
                } else {
                    eprintln!("[quit] daemon did not exit in 10s — force killing");
                    daemon::force_kill_local();
                    std::thread::sleep(Duration::from_millis(300));
                }
                // 5) Только теперь прячем окно и выходим
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_w.upgrade() { ui.window().hide().ok(); }
                });
                slint::quit_event_loop().ok();
            });
        })
    };

    // Форс-выход из анимации: SIGKILL демону, мгновенное скрытие и выход
    {
        let ui_weak = ui.as_weak();
        let quitting2 = quitting.clone();
        ui.on_close_anim_force(move || {
            quitting2.store(true, std::sync::atomic::Ordering::SeqCst);
            daemon::force_kill_local();
            if let Some(ui) = ui_weak.upgrade() { ui.window().hide().ok(); }
            slint::quit_event_loop().ok();
        });
    }

    {
        let dq = do_quit.clone();
        ui.on_do_quit(move || {
            dq();
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.window().on_close_requested(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_close_dialog_visible(true);
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
    }

    // ── Автосохранение настроек: пока открыт диалог, раз в 800мс сравниваем
    // снапшот из UI с последним отправленным; отличие → session-set + конфиг
    let last_settings: std::sync::Arc<std::sync::Mutex<Option<DaemonSettings>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let last_app_auto = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    // Юзер редактировал настройки с момента открытия диалога → поздний Loaded игнорируем
    let edits_since_open = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    // Немедленный flush настроек при закрытии диалога (автосейв 800мс мог не успеть)
    {
        let ui_c = ui.as_weak();
        let tx_c = cmd_tx.clone();
        let ls_c = last_settings.clone();
        ui.on_settings_close(move || {
            if let Some(ui) = ui_c.upgrade() {
                let s = build_daemon_settings(&ui);
                let changed = match ls_c.lock().unwrap().as_ref() {
                    Some(prev) => *prev != s,
                    None => false,
                };
                if changed {
                    *ls_c.lock().unwrap() = Some(s.clone());
                    eprintln!("[settings] close flush: begin={} end={} → session-set", s.alt_speed_time_begin, s.alt_speed_time_end);
                    let _ = tx_c.send(Command::SaveDaemonSettings(s));
                    let _ = apply_app_config_from_ui(&ui);
                    app_config::sync_autostart(config_lock().lock().unwrap().autostart);
                }
            }
        });
    }

    // ── Насос: статус и трей 20Hz, данные торрентов 2Hz ─────────────────────
    let ui_h        = ui.as_weak();
    let mdl         = torrent_model.clone();
    let _tmr_fast   = slint::Timer::default(); // статус + трей — 50ms
    let _tmr_data   = slint::Timer::default(); // торренты + stats — 500ms
    let do_quit_tmr = do_quit.clone();

    // Быстрый таймер: только статус и трей
    let tray_ready = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        // Дренируем буферизованные события трея перед стартом (snixembed может послать activate при регистрации)
        if let Some(ref t) = tray { t.poll_events(); }
        let tr = tray_ready.clone();
        slint::Timer::single_shot(Duration::from_millis(1500), move || {
            tr.store(true, std::sync::atomic::Ordering::Relaxed);
        });
    }
    _tmr_fast.start(slint::TimerMode::Repeated, Duration::from_millis(50), {
        let ui_h = ui_h.clone();
        let do_quit_tmr = do_quit_tmr.clone();
        let cmd_tx_fast = cmd_tx.clone();
        let last_settings = last_settings.clone();
        let eso = edits_since_open.clone();
        move || {
        while let Ok(msg) = status_rx.try_recv() {
            if let Some(ui) = ui_h.upgrade() {
                let ok = msg.starts_with("Connected");
                ui.set_connected(ok);
                ui.set_status_bar_text(msg.into());
            }
        }
        while let Ok(result) = settings_rx.try_recv() {
            match result {
                SettingsResult::Loaded(s) => {
                    // Гонка: юзер уже редактировал настройки с момента открытия диалога —
                    // поздний ответ Load перезаписал бы его правки. Пропускаем.
                    if eso.load(std::sync::atomic::Ordering::Relaxed) {
                        eprintln!("[settings] Loaded skipped — user edited since open (race guard)");
                        continue;
                    }
                    eprintln!("[settings] Loaded from daemon: begin={} end={} enabled={}", s.alt_speed_time_begin, s.alt_speed_time_end, s.alt_speed_time_enabled);
                    if let Some(ui2) = ui_h.upgrade() {
                        let cur = build_daemon_settings(&ui2);
                        let user_edited = match last_settings.lock().unwrap().as_ref() {
                            Some(prev) => cur != *prev,
                            None => false,
                        };
                        if user_edited {
                            eprintln!("[settings] Loaded skipped — user already edited (race guard)");
                            continue;
                        }
                    }
                    if let Some(ui) = ui_h.upgrade() {
                    ui.set_cfg_speed_up_enabled(s.speed_limit_up_enabled);
                    ui.set_cfg_speed_up_kbs((s.speed_limit_up * 8 / 1000) as i32); // KB/s → Мбит/с
                    ui.set_cfg_speed_down_enabled(s.speed_limit_down_enabled);
                    ui.set_cfg_speed_down_kbs((s.speed_limit_down * 8 / 1000) as i32);
                    ui.set_cfg_alt_speed_enabled(s.alt_speed_enabled);
                    ui.set_cfg_alt_speed_up_kbs((s.alt_speed_up * 8 / 1000) as i32);
                    ui.set_cfg_alt_speed_down_kbs((s.alt_speed_down * 8 / 1000) as i32);
                        ui.set_cfg_alt_speed_schedule(s.alt_speed_time_enabled);
                        ui.set_cfg_alt_speed_time_begin(s.alt_speed_time_begin as i32);
                        ui.set_cfg_alt_speed_time_end(s.alt_speed_time_end as i32);
                        ui.set_cfg_speed_day_mon(s.alt_speed_time_day & 1 != 0);
                        ui.set_cfg_speed_day_tue(s.alt_speed_time_day & 2 != 0);
                        ui.set_cfg_speed_day_wed(s.alt_speed_time_day & 4 != 0);
                        ui.set_cfg_speed_day_thu(s.alt_speed_time_day & 8 != 0);
                        ui.set_cfg_speed_day_fri(s.alt_speed_time_day & 16 != 0);
                        ui.set_cfg_speed_day_sat(s.alt_speed_time_day & 32 != 0);
                        ui.set_cfg_speed_day_sun(s.alt_speed_time_day & 64 != 0);
                        ui.set_cfg_dl_watch_dir_enabled(s.watch_dir_enabled);
                        ui.set_cfg_dl_watch_dir(s.watch_dir.as_str().into());
                        ui.set_cfg_dl_start_added(s.start_added_torrents);
                        ui.set_cfg_dl_trash_torrent(s.trash_original_torrent_files);
                        ui.set_cfg_dl_dir(s.download_dir.as_str().into());
                        ui.set_cfg_dl_queue_max(s.download_queue_size as i32);
                        ui.set_cfg_dl_queue_enabled(s.download_queue_enabled);
                        ui.set_cfg_dl_seed_ratio_limit_min(s.queue_stalled_minutes as i32);
                        ui.set_cfg_dl_part_ext(s.rename_partial_files);
                        ui.set_cfg_dl_incomplete_dir_enabled(s.incomplete_dir_enabled);
                        ui.set_cfg_dl_incomplete_dir(s.incomplete_dir.as_str().into());
                        ui.set_cfg_dl_done_script_enabled(s.script_torrent_done_enabled);
                        ui.set_cfg_dl_done_script(s.script_torrent_done_filename.as_str().into());
                        ui.set_cfg_seed_done_script_enabled(s.script_torrent_done_seeding_enabled);
                        ui.set_cfg_seed_done_script(s.script_torrent_done_seeding_filename.as_str().into());
                        ui.set_cfg_seed_ratio_enabled(s.seed_ratio_limited);
                        ui.set_cfg_seed_ratio((s.seed_ratio_limit * 100.0) as i32);
                        ui.set_cfg_seed_idle_enabled(s.idle_seeding_limit_enabled);
                        ui.set_cfg_seed_idle_min(s.idle_seeding_limit as i32);
                        ui.set_cfg_net_port(s.peer_port as i32);
                        ui.set_cfg_net_random_port(s.peer_port_random_on_start);
                        ui.set_cfg_net_port_forward(s.port_forwarding_enabled);
                        ui.set_cfg_net_max_peers_torrent(s.peer_limit_per_torrent as i32);
                        ui.set_cfg_net_max_peers_total(s.peer_limit_global as i32);
                        ui.set_cfg_net_utp(s.utp_enabled);
                        ui.set_cfg_net_pex(s.pex_enabled);
                        ui.set_cfg_net_dht(s.dht_enabled);
                        ui.set_cfg_net_lpd(s.lpd_enabled);
                        ui.set_cfg_net_default_trackers(s.default_trackers.as_str().into());
                        ui.set_cfg_priv_encryption(s.encryption as i32);
                        ui.set_cfg_priv_blocklist_enabled(s.blocklist_enabled);
                        ui.set_cfg_priv_blocklist_url(s.blocklist_url.as_str().into());
                        ui.set_cfg_remote_enabled(s.rpc_enabled);
                        ui.set_cfg_remote_port(s.rpc_port as i32);
                        ui.set_cfg_remote_auth(s.rpc_authentication_required);
                        ui.set_cfg_remote_username(s.rpc_username.as_str().into());
                        ui.set_cfg_remote_password(s.rpc_password.as_str().into());
                        ui.set_cfg_remote_whitelist_enabled(s.rpc_whitelist_enabled);
                        ui.set_cfg_remote_whitelist(s.rpc_whitelist.as_str().into());
                        eprintln!("[settings] Daemon settings applied to UI");
                        // Базовая калибровка автосейва: то, что пришло, считаем отправленным
                        *last_settings.lock().unwrap() = Some(s.clone());
                    }
                }
                SettingsResult::Saved => {
                    // Автосохранение — без спама статус-бара
                    eprintln!("[settings] Daemon settings saved (auto)");
                }
                SettingsResult::Error(e) => {
                    if let Some(ui) = ui_h.upgrade() {
                        ui.set_status_bar_text(format!("Settings error: {e}").into());
                    }
                }
            }
        }
            if !tray_ready.load(std::sync::atomic::Ordering::Relaxed) {
                // Дренируем события пока не готовы — игнорируем буферизованные клики
                if let Some(ref tray) = tray { tray.poll_events(); }
                return;
            }
            if let Some(ref tray) = tray {
                let (toggle, quit, start_all, stop_all) = tray.poll_events();
                if toggle {
                    if let Some(ui) = ui_h.upgrade() {
                        let win = ui.window();
                        if win.is_visible() {
                            // Скрываем через callback — он обработает suspend
                            ui.invoke_do_minimize_tray();
                        } else {
                            // Показываем напрямую — suspend не нужен
                            win.show().ok();
                            wm_icon::set_wm_icon_by_pid();
                        }
                    }
                }
                if quit {
                    if let Some(ui) = ui_h.upgrade() { ui.window().hide().ok(); }
                    do_quit_tmr();
                }
                if start_all {
                    eprintln!("[tray] Resume All");
                    let _ = cmd_tx_fast.send(Command::StartAll);
                }
                if stop_all {
                    eprintln!("[tray] Pause All");
                    let _ = cmd_tx_fast.send(Command::StopAll);
                }
            }
        }
    });

    // stop/start disk torrents callbacks
    {
        let tx  = cmd_tx.clone();
        let mm  = majmin_map.clone();
        let pd  = physical_disks_arc.clone();
        let snap = torrent_snapshot.clone();
        ui.on_stop_disk_torrents(move |idx| {
            if let Some(disk) = pd.get(idx as usize) {
                let ids: Vec<i64> = snap.lock().unwrap().iter()
                    .filter(|t| {
                        disks::disk_for_path(&t.download_dir, &mm)
                            .as_deref() == Some(&disk.dev)
                    })
                    .filter(|t| !t.is_paused()) // только активные
                    .map(|t| t.id)
                    .collect();
                eprintln!("[disk] Pause disk {}: {} torrents", disk.dev, ids.len());
                if !ids.is_empty() {
                    let _ = tx.send(Command::StopDisk(ids));
                }
            }
        });
    }
    {
        let tx  = cmd_tx.clone();
        let mm  = majmin_map.clone();
        let pd  = physical_disks_arc.clone();
        let snap = torrent_snapshot.clone();
        ui.on_start_disk_torrents(move |idx| {
            if let Some(disk) = pd.get(idx as usize) {
                let ids: Vec<i64> = snap.lock().unwrap().iter()
                    .filter(|t| {
                        disks::disk_for_path(&t.download_dir, &mm)
                            .as_deref() == Some(&disk.dev)
                    })
                    .filter(|t| t.is_paused()) // только остановленные
                    .map(|t| t.id)
                    .collect();
                eprintln!("[disk] Resume disk {}: {} torrents", disk.dev, ids.len());
                if !ids.is_empty() {
                    let _ = tx.send(Command::StartDisk(ids));
                }
            }
        });
    }

    // Медленный таймер: обновление списка торрентов и статистики
    let majmin_arc = majmin_map.clone();
    let disks_arc  = physical_disks_arc.clone();
    let snap_w     = torrent_snapshot.clone();
    let mut notify_tracker = notify::NotifyTracker::default();

    // Кэш отфильтрованного по диску списка — поиск применяется поверх мгновенно
    let disk_filtered: std::sync::Arc<std::sync::Mutex<Vec<rpc::RawTorrent>>> =
        std::sync::Arc::new(std::sync::Mutex::new(vec![]));
    // Активный фильтр статуса — сохраняется для применения в таймере
    let active_filter: std::sync::Arc<std::sync::Mutex<String>> =
        std::sync::Arc::new(std::sync::Mutex::new("all".to_string()));
    let sort_col: std::sync::Arc<std::sync::Mutex<(String, bool)>> =
        std::sync::Arc::new(std::sync::Mutex::new((String::new(), false)));
    {
        let df  = disk_filtered.clone();
        let mdl = torrent_model.clone();
        ui.on_search_changed(move |query| {
            let q = query.to_string().to_lowercase();
            let src = df.lock().unwrap();
            let filtered: Vec<&rpc::RawTorrent> = if q.is_empty() {
                src.iter().collect()
            } else {
                src.iter().filter(|t| t.name.to_lowercase().contains(&q)).collect()
            };
            apply_torrent_update(&mdl, &filtered);
        });
    }
    // Фильтр по статусу
    {
        let df = disk_filtered.clone();
        let mdl = torrent_model.clone();
        let ui_h_f = ui_h.clone();
        let filter_clone = active_filter.clone();
        ui.on_filter_clicked(move |status: slint::SharedString| {
            let status = status.to_string();
            *filter_clone.lock().unwrap() = status.clone();
            // Обновляем active-filter в UI
            if let Some(ui) = ui_h_f.upgrade() {
                ui.set_active_filter(status.clone().into());
            }
            let src = df.lock().unwrap();
            let filtered: Vec<&rpc::RawTorrent> = match status.as_str() {
                "downloading" => src.iter().filter(|t: &&rpc::RawTorrent| t.status == 4 && !t.is_paused()).collect(),
                "seeding" => src.iter().filter(|t: &&rpc::RawTorrent| t.status == 6 && !t.is_paused()).collect(),
                "completed" => src.iter().filter(|t: &&rpc::RawTorrent| t.percent_done >= 1.0).collect(),
                "stopped" => src.iter().filter(|t: &&rpc::RawTorrent| t.is_paused()).collect(),
                "active" => src.iter().filter(|t: &&rpc::RawTorrent| t.rate_upload > 0 || t.rate_download > 0).collect(),
                "error" => src.iter().filter(|t: &&rpc::RawTorrent| t.status == 3 || t.error != 0).collect(),
                _ => src.iter().collect(),
            };
            apply_torrent_update(&mdl, &filtered);
        });
    }

    // Сортировка по колонке
    {
        let sc = sort_col.clone();
        ui.on_sort_changed(move |col: slint::SharedString, asc: bool| {
            *sc.lock().unwrap() = (col.to_string(), asc);
        });
    }

    // Переключение RPC
    {
        let ui_weak = ui.as_weak();
        let candidates_full = config::detect_rpc_candidates();
        let tx = cmd_tx.clone();
        ui.on_switch_rpc(move |display: slint::SharedString| {
            let display_str = display.to_string();
            // Клик по текущему хосту — no-op (раньше запускал reconnect с битым URL и валил клиент)
            let active = ACTIVE_RPC_URL.lock().unwrap().clone().unwrap_or_default();
            if url_endpoint(&active) == url_endpoint(&display_str) {
                eprintln!("[switch-rpc] {display_str} is already active — no-op");
                if let Some(ui) = ui_weak.upgrade() { ui.set_is_switching_host(false); }
                return;
            }
            // Находим полный URL по display-имени (host:port)
            let full_url = candidates_full.iter().find(|c| {
                let d = c.url.trim_start_matches("https://").trim_start_matches("http://")
                    .split('/').next().unwrap_or("").to_string();
                d == display_str
            }).map(|c| c.url.clone()).unwrap_or(display_str.clone());
            // Повторная no-op проверка уже по полному URL
            if url_endpoint(&full_url) == url_endpoint(&active) {
                eprintln!("[switch-rpc] {full_url} is already active — no-op");
                if let Some(ui) = ui_weak.upgrade() { ui.set_is_switching_host(false); }
                return;
            }

            eprintln!("[switch-rpc] {} → {}", display_str, full_url);
            *ACTIVE_RPC_URL.lock().unwrap() = Some(full_url.clone());
            let _ = tx.send(Command::SwitchRpc(full_url));
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_rpc_url(display);
                refresh_profiles(&ui);
            }
        });
    }

    // Проверка открытости порта (RPC port-test, долго — до 30с)
    {
        let tx = cmd_tx.clone();
        ui.on_port_test_start(move || {
            let _ = tx.send(Command::PortTest);
        });
    }

    // Смена хоста из списка профилей (full URL напрямую)
    {
        let tx = cmd_tx.clone();
        let ui_weak = ui.as_weak();
        ui.on_switch_host(move |url: slint::SharedString| {
            let full = url.to_string();
            // Клик по текущему хосту — no-op
            let active = ACTIVE_RPC_URL.lock().unwrap().clone().unwrap_or_default();
            if url_endpoint(&full) == url_endpoint(&active) {
                eprintln!("[switch-host] {full} is already active — no-op");
                if let Some(ui) = ui_weak.upgrade() { ui.set_is_switching_host(false); }
                return;
            }
            eprintln!("[switch-host] → {full}");
            *ACTIVE_RPC_URL.lock().unwrap() = Some(full.clone());
            let _ = tx.send(Command::SwitchRpc(full.clone()));
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_is_switching_host(true);
                ui.set_switching_host_name(url_endpoint(&full).into());
                ui.set_rpc_url(url_endpoint(&full).into());
                refresh_profiles(&ui);
            }
        });
    }

    // Сохранение хоста (добавление/редактирование) — persists в конфиг
    {
        let ui_weak = ui.as_weak();
        ui.on_save_host(move |name: slint::SharedString, host: slint::SharedString,
                             port: i32, user: slint::SharedString,
                             pass: slint::SharedString, orig: slint::SharedString| {
            eprintln!("[save-host] called: name={name:?} host={host:?} port={port} orig={orig:?}");
            let name_s = name.trim().to_string();
            let host_s = host.trim().to_string();
            if name_s.is_empty() || host_s.is_empty() { return; }
            let url = format!("http://{host_s}:{port}/transmission/rpc");
            {
                let mut cfg = config_lock().lock().unwrap();
                let orig_s = orig.to_string();
                if !orig_s.is_empty() {
                    if let Some(h) = cfg.custom_hosts.iter_mut().find(|h| h.name == orig_s) {
                        h.name = name_s.clone();
                        h.url = url.clone();
                        h.user = user.to_string();
                        h.password = pass.to_string();
                    }
                } else if !cfg.custom_hosts.iter().any(|h| h.name == name_s) {
                    cfg.custom_hosts.push(app_config::HostProfile {
                        name: name_s, url, user: user.to_string(), password: pass.to_string(),
                    });
                }
                app_config::save(&*cfg);
            }
            if let Some(ui) = ui_weak.upgrade() { refresh_profiles(&ui); }
        });
    }

    // Удаление пользовательского хоста
    {
        let ui_weak = ui.as_weak();
        ui.on_delete_host(move |name: slint::SharedString| {
            eprintln!("[delete-host] called: {name:?}");
            let name_s = name.to_string();
            let mut removed = false;
            {
                let mut cfg = config_lock().lock().unwrap();
                let before = cfg.custom_hosts.len();
                cfg.custom_hosts.retain(|h| h.name != name_s);
                removed = cfg.custom_hosts.len() != before;
                if removed { app_config::save(&*cfg); }
            }
            if removed {
                if let Some(ui) = ui_weak.upgrade() { refresh_profiles(&ui); }
            }
        });
    }

    // Settings dialog: Load daemon settings when opened
    {
        let tx = cmd_tx.clone();
        let edits = edits_since_open.clone();
        ui.on_settings_open(move || {
            edits.store(false, std::sync::atomic::Ordering::Relaxed);
            eprintln!("[settings] dialog open → Load sent");
            let _ = tx.send(Command::LoadDaemonSettings);
        });
    }
    {
        let edits = edits_since_open.clone();
        ui.on_settings_touched(move || {
            edits.store(true, std::sync::atomic::Ordering::Relaxed);
        });
    }

    let _tmr_auto = slint::Timer::default();
    {
        let ui_w = ui.as_weak();
        let tx = cmd_tx.clone();
        let ls = last_settings.clone();
        let prev_auto = last_app_auto.clone();
        let ls2 = last_settings.clone();
    let pa2 = last_app_auto.clone();
    let tx2 = cmd_tx.clone();
    let ui_w2 = ui.as_weak();
    _tmr_auto.start(slint::TimerMode::Repeated, Duration::from_millis(800), move || {
            let Some(ui) = ui_w2.upgrade() else { return; };;
            if !ui.get_settings_dialog_visible() { return; }
            let s = build_daemon_settings(&ui);
            let changed = match ls.lock().unwrap().as_ref() {
                Some(prev) => *prev != s,
                None => false, // ждём первый Loaded — базовая калибровка
            };
            if !changed { return; }
            *ls.lock().unwrap() = Some(s.clone());
            eprintln!("[settings] auto-save: change detected begin={} end={} → session-set", s.alt_speed_time_begin, s.alt_speed_time_end);
            let _ = tx.send(Command::SaveDaemonSettings(s));

            // AppConfig (не-демонские настройки) — онлайн в конфиг
            let (auto_now, last_upd) = apply_app_config_from_ui(&ui);
            app_config::sync_autostart(config_lock().lock().unwrap().autostart);

            // Blocklist: включили автообновление и список устарел → обновить сейчас
            let was = prev_auto.swap(auto_now, std::sync::atomic::Ordering::Relaxed);
            if auto_now && !was {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs()).unwrap_or(0);
                if now.saturating_sub(last_upd) > 86_400 {
                    let _ = tx.send(Command::UpdateBlocklist);
                }
            }
        });
    }

    // Кэш: download_dir → /dev/sdX — заполняется в фоне, читается в event loop
    let path_disk_cache: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Option<String>>>>
        = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let sort_col_tmr = sort_col.clone();
    _tmr_data.start(slint::TimerMode::Repeated, Duration::from_millis(1000), move || {
        let mut last_update: Option<Update> = None;
        while let Ok(upd) = update_rx.try_recv() { last_update = Some(upd); }
        if let Some(upd) = last_update {
            // Обновляем shared snapshot для disk callbacks
            *snap_w.lock().unwrap() = upd.torrents.clone();

            // Уведомления: завершение, ошибки, рехеш
            {
                let events: Vec<(i64, &str, f64, i64, i64)> = upd.torrents.iter()
                    .map(|t| (t.id, t.name.as_str(), t.percent_done, t.status, t.error))
                    .collect();
                notify_tracker.update(&events);
            }

            let selected = ui_h.upgrade().map(|u| u.get_selected_disk()).unwrap_or(-1);

            // Заполняем кэш в фоне для новых путей (не блокируем event loop)
            {
                let cache = path_disk_cache.clone();
                let mm    = majmin_arc.clone();
                let dirs: Vec<String> = upd.torrents.iter()
                    .filter_map(|t| {
                        let c = cache.lock().unwrap();
                        if c.contains_key(&t.download_dir) { None }
                        else { Some(t.download_dir.clone()) }
                    })
                    .collect::<std::collections::HashSet<_>>() // только уникальные
                    .into_iter().collect();
                if !dirs.is_empty() {
                    std::thread::spawn(move || {
                        for dir in dirs {
                            let result = disks::disk_for_path(&dir, &mm);
                            cache.lock().unwrap().insert(dir, result);
                        }
                    });
                }
            }

            // Читаем кэш (только уже готовые записи)
            let cache_snap = path_disk_cache.lock().unwrap().clone();

            // Считаем торренты на каждом диске
            let counts: Vec<usize> = disks_arc.iter().map(|disk| {
                upd.torrents.iter().filter(|t|
                    cache_snap.get(&t.download_dir).and_then(|d| d.as_deref())
                        == Some(&disk.dev)
                ).count()
            }).collect();

            // Обновляем счётчики (slint скроет кнопки с count=0 через visible)
            for (idx, count) in counts.iter().enumerate() {
                if let Some(mut item) = disk_model.row_data(idx) {
                    if item.torrent_count != *count as i32 {
                        item.torrent_count = *count as i32;
                        disk_model.set_row_data(idx, item);
                    }
                }
            }

            // Фильтруем по диску и сохраняем в кэш
            let after_disk: Vec<rpc::RawTorrent> = if selected < 0 {
                upd.torrents.clone()
            } else if let Some(disk) = disks_arc.get(selected as usize) {
                upd.torrents.iter().filter(|t|
                    cache_snap.get(&t.download_dir).and_then(|d| d.as_deref())
                        == Some(&disk.dev)
                ).cloned().collect()
            } else {
                upd.torrents.clone()
            };

            // Применяем фильтр статуса
            let filter_status = active_filter.lock().unwrap().clone();
            let after_status: Vec<rpc::RawTorrent> = match filter_status.as_str() {
                "downloading" => after_disk.iter().filter(|t| t.status == 4 && !t.is_paused()).cloned().collect(),
                "seeding" => after_disk.iter().filter(|t| t.status == 6 && !t.is_paused()).cloned().collect(),
                "completed" => after_disk.iter().filter(|t| t.percent_done >= 1.0).cloned().collect(),
                "stopped" => after_disk.iter().filter(|t| t.is_paused()).cloned().collect(),
                "active" => after_disk.iter().filter(|t| t.rate_upload > 0 || t.rate_download > 0).cloned().collect(),
                "error" => after_disk.iter().filter(|t| t.status == 3 || t.error != 0).cloned().collect(),
                _ => after_disk,
            };

            // Применяем поиск поверх status filter
            let query = ui_h.upgrade()
                .map(|u| u.get_search_text().to_string().to_lowercase())
                .unwrap_or_default();
            let mut filtered_refs: Vec<&rpc::RawTorrent> = if query.is_empty() {
                after_status.iter().collect()
            } else {
                after_status.iter().filter(|t| t.name.to_lowercase().contains(&query)).collect()
            };

            // Применяем сортировку по колонке
            let (sc, sa) = sort_col_tmr.lock().unwrap().clone();
            match sc.as_str() {
                "down" => {
                    // нулевые в конец, активные сортируются по скорости
                    if sa {
                        filtered_refs.sort_by_key(|t| if t.rate_download > 0 { t.rate_download } else { i64::MAX });
                    } else {
                        filtered_refs.sort_by_key(|t| if t.rate_download > 0 { -t.rate_download } else { i64::MAX });
                    }
                }
                "up" => {
                    if sa {
                        filtered_refs.sort_by_key(|t| if t.rate_upload > 0 { t.rate_upload } else { i64::MAX });
                    } else {
                        filtered_refs.sort_by_key(|t| if t.rate_upload > 0 { -t.rate_upload } else { i64::MAX });
                    }
                }
                "done" => {
                    let factor = if sa { 1i64 } else { -1i64 };
                    filtered_refs.sort_by_key(|t| factor * (t.percent_done * 1_000_000.0) as i64);
                }
                _ => {}
            }

            apply_torrent_update(&mdl, &filtered_refs);

            // Сохраняем status-filtered список для мгновенного поиска
            *disk_filtered.lock().unwrap() = after_status.clone();

            // Подсчитываем counts из ВСЕХ торрентов (не filtered)
            let all = upd.torrents.len() as i32;
            let downloading = upd.torrents.iter().filter(|t| t.status == 4 && !t.is_paused()).count() as i32;
            let seeding = upd.torrents.iter().filter(|t| t.status == 6 && !t.is_paused()).count() as i32;
            let completed = upd.torrents.iter().filter(|t| t.percent_done >= 1.0).count() as i32;
            let stopped = upd.torrents.iter().filter(|t| t.is_paused()).count() as i32;
            let active = upd.torrents.iter().filter(|t| t.rate_upload > 0 || t.rate_download > 0).count() as i32;
            let error = upd.torrents.iter().filter(|t| t.status == 3 || t.error != 0).count() as i32;
            
            if let Some(ui) = ui_h.upgrade() {
                let s = &upd.stats;
                ui.set_stats(SessionStats {
                    down_speed:  fmt_speed(s.down_speed),
                    up_speed:    fmt_speed(s.up_speed),
                    downloaded:  fmt_bytes(s.downloaded),
                    uploaded:    fmt_bytes(s.uploaded),
                    ratio:       fmt_ratio(s.ratio),
                    active:      s.active_count as i32,
                    dht:         s.dht_nodes as i32,
                    peers:       s.peer_count as i32,
                });
                ui.set_count_all(all);
                ui.set_count_downloading(downloading);
                ui.set_count_seeding(seeding);
                ui.set_count_completed(completed);
                ui.set_count_stopped(stopped);
                ui.set_count_active(active);
                ui.set_count_error(error);
            }
        }
    });

    if app_cfg.start_minimized {
        ui.show()?;
        ui.window().hide().ok();
    } else {
        ui.show()?;
    }

    // Инициализируем фильтр "all" при старте
    ui.invoke_filter_clicked("all".into());

    // Устанавливаем иконку в докбар через _NET_WM_ICON (только X11)
    if std::env::var("DISPLAY").is_ok() {
        wm_icon::set_wm_icon_by_pid();
    }

    // Открытие .torrent из проводника — выбор папки (или сразу добавление при dl-show-dialog=false)
    if let Some(torrent_path) = pending_torrent {
        let tx = cmd_tx.clone();
        let delete_after = app_cfg.delete_torrent_after_add;
        let show_dialog = app_cfg.dl_show_dialog;
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            if !show_dialog {
                eprintln!("[open] Adding {torrent_path} → default dir (dialog disabled)");
                let _ = tx.send(Command::AddTorrentFile(torrent_path, None, delete_after));
                return;
            }
            match filepicker::pick_directory("") {
                Ok(dir) => {
                    eprintln!("[open] Adding {torrent_path} → {dir}");
                    let _ = tx.send(Command::AddTorrentFile(torrent_path, Some(dir), delete_after));
                }
                Err(_) => eprintln!("[open] Directory selection cancelled"),
            }
        });
    }

    // Слушаем входящие файлы от вторичных экземпляров
    {
        let tx = cmd_tx.clone();
        let ui_weak = ui.as_weak();
        let delete_after = app_cfg.delete_torrent_after_add;
        let show_dialog = app_cfg.dl_show_dialog;
        single_instance::start_listener(listener, move |torrent_path| {
            let tx2 = tx.clone();
            let ui2 = ui_weak.clone();
            let delete_after2 = delete_after;
            let show_dialog2 = show_dialog;
            // Поднимаем окно
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui2.upgrade() { ui.show().ok(); }
            });
            if torrent_path.is_empty() { return; }
            // Выбор папки и добавление (или сразу при dl-show-dialog=false)
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(300));
                if !show_dialog2 {
                    eprintln!("[open] Adding {torrent_path} → default dir (dialog disabled)");
                    let _ = tx2.send(Command::AddTorrentFile(torrent_path, None, delete_after2));
                    return;
                }
                match filepicker::pick_directory("") {
                    Ok(dir) => {
                        eprintln!("[open] Adding {torrent_path} → {dir}");
                        let _ = tx2.send(Command::AddTorrentFile(torrent_path, Some(dir), delete_after2));
                    }
                    Err(_) => eprintln!("[open] Directory selection cancelled"),
                }
            });
        });
    }

    // Форс логического размера окна: WM игнорирует preferred и открывает по min-width (740),
    // причём применяет свою геометрию ПОСЛЕ маппинга — поэтому принуждаем цепочкой ~4 сек.
    fn enforce_window_size(ui0: slint::Weak<MainWindow>, n: u32) {
        if n == 0 { return; }
        slint::Timer::single_shot(Duration::from_millis(300), move || {
            if let Some(ui) = ui0.upgrade() {
                let w = ui.window();
                let sz = w.size();
                let sf = w.scale_factor() as f32;
                let min_w = (1060.0 * sf) as u32;
                let min_h = (700.0 * sf) as u32;
                if sz.width < min_w || sz.height < min_h {
                    w.set_size(slint::WindowSize::Logical(slint::LogicalSize::new(1060.0, 700.0)));
                    eprintln!("[window] #{n} was {sz:?} → forced 1060x700");
                }
                enforce_window_size(ui0, n - 1);
            }
        });
    }
    {
        let ui0 = ui.as_weak();
        enforce_window_size(ui0, 14);
    }

    slint::run_event_loop_until_quit()?;

    // daemon stopped in do_quit() before event loop exit
    Ok(())
}

/// Проталкивает все строки диалогов/вкладок в глобаль Tr (5 языков из i18n)
fn push_dialog_tr(ui: &MainWindow) {
    let g = Tr::get(ui);
    g.set_about_desc(i18n::d_about_desc().into());
    g.set_about_title(i18n::d_about_title().into());
    g.set_add_connection(i18n::d_add_connection().into());
    g.set_alt_dl_lbl(i18n::d_alt_dl_lbl().into());
    g.set_alt_override(i18n::d_alt_override().into());
    g.set_alt_up_lbl(i18n::d_alt_up_lbl().into());
    g.set_append_part(i18n::d_append_part().into());
    g.set_at_add_url(i18n::d_at_add_url().into());
    g.set_at_browse(i18n::d_at_browse().into());
    g.set_at_title(i18n::d_at_title().into());
    g.set_at_url_ph(i18n::d_at_url_ph().into());
    g.set_auto_import(i18n::d_auto_import().into());
    g.set_autostart_lbl(i18n::d_autostart_lbl().into());
    g.set_blocklist_auto(i18n::d_blocklist_auto().into());
    g.set_blocklist_enable(i18n::d_blocklist_enable().into());
    g.set_blocklist_entries(i18n::d_blocklist_entries().into());
    g.set_blocklist_entries2(i18n::d_blocklist_entries2().into());
    g.set_blocklist_url(i18n::d_blocklist_url().into());
    g.set_by_schedule(i18n::d_by_schedule().into());
    g.set_ca_done(i18n::d_ca_done().into());
    g.set_ca_done_closing(i18n::d_ca_done_closing().into());
    g.set_ca_left(i18n::d_ca_left().into());
    g.set_ca_notifying(i18n::d_ca_notifying().into());
    g.set_ca_sending(i18n::d_ca_sending().into());
    g.set_ca_shutting(i18n::d_ca_shutting().into());
    g.set_ca_units(i18n::d_ca_units().into());
    g.set_cancel_btn(i18n::d_cancel_btn().into());
    g.set_cd_min_tray(i18n::d_cd_min_tray().into());
    g.set_cd_quit(i18n::d_cd_quit().into());
    g.set_cd_what(i18n::d_cd_what().into());
    g.set_check_port(i18n::d_check_port().into());
    g.set_config_found(i18n::d_config_found().into());
    g.set_config_not_found(i18n::d_config_not_found().into());
    g.set_conn_host(i18n::d_conn_host().into());
    g.set_conn_profiles(i18n::d_conn_profiles().into());
    g.set_ct_browse(i18n::d_ct_browse().into());
    g.set_ct_create(i18n::d_ct_create().into());
    g.set_ct_path_ph(i18n::d_ct_path_ph().into());
    g.set_ct_title(i18n::d_ct_title().into());
    g.set_ct_trackers(i18n::d_ct_trackers().into());
    g.set_day_fr(i18n::d_day_fr().into());
    g.set_day_mo(i18n::d_day_mo().into());
    g.set_day_sa(i18n::d_day_sa().into());
    g.set_day_su(i18n::d_day_su().into());
    g.set_day_th(i18n::d_day_th().into());
    g.set_day_tu(i18n::d_day_tu().into());
    g.set_day_we(i18n::d_day_we().into());
    g.set_default_dir_lbl(i18n::d_default_dir_lbl().into());
    g.set_del_btn(i18n::d_del_btn().into());
    g.set_del_confirm(i18n::d_del_confirm().into());
    g.set_del_profile_q(i18n::d_del_profile_q().into());
    g.set_del_secs(i18n::d_del_secs().into());
    g.set_del_word(i18n::d_del_word().into());
    g.set_dev_with(i18n::d_dev_with().into());
    g.set_dht(i18n::d_dht().into());
    g.set_dlimit_lbl(i18n::d_dlimit_lbl().into());
    g.set_done_script_dl(i18n::d_done_script_dl().into());
    g.set_done_script_seed(i18n::d_done_script_seed().into());
    g.set_edit_btn(i18n::d_edit_btn().into());
    g.set_enc_disabled(i18n::d_enc_disabled().into());
    g.set_enc_prefer(i18n::d_enc_prefer().into());
    g.set_enc_require(i18n::d_enc_require().into());
    g.set_encryption_mode(i18n::d_encryption_mode().into());
    g.set_enforce_auth(i18n::d_enforce_auth().into());
    g.set_enforce_auth_short(i18n::d_enforce_auth_short().into());
    g.set_exit_now(i18n::d_exit_now().into());
    g.set_free_disk_note(i18n::d_free_disk_note().into());
    g.set_host_lbl(i18n::d_host_lbl().into());
    g.set_incomplete_dir_lbl(i18n::d_incomplete_dir_lbl().into());
    g.set_lang_lbl(i18n::d_lang_lbl().into());
    g.set_lang_section(i18n::d_lang_section().into());
    g.set_license_lbl(i18n::d_license_lbl().into());
    g.set_local_host(i18n::d_local_host().into());
    g.set_login_ph(i18n::d_login_ph().into());
    g.set_lpd(i18n::d_lpd().into());
    g.set_manage_profiles(i18n::d_manage_profiles().into());
    g.set_mg_title(i18n::d_mg_title().into());
    g.set_no_profiles(i18n::d_no_profiles().into());
    g.set_notify_add(i18n::d_notify_add().into());
    g.set_notify_complete(i18n::d_notify_complete().into());
    g.set_notify_sound(i18n::d_notify_sound().into());
    g.set_on_close_ask(i18n::d_on_close_ask().into());
    g.set_on_close_lbl(i18n::d_on_close_lbl().into());
    g.set_on_close_quit(i18n::d_on_close_quit().into());
    g.set_on_close_tray(i18n::d_on_close_tray().into());
    g.set_password_ph(i18n::d_password_ph().into());
    g.set_path_to_script(i18n::d_path_to_script().into());
    g.set_peer_port_lbl(i18n::d_peer_port_lbl().into());
    g.set_peers_max_global(i18n::d_peers_max_global().into());
    g.set_peers_max_torrent(i18n::d_peers_max_torrent().into());
    g.set_pex(i18n::d_pex().into());
    g.set_port_checking(i18n::d_port_checking().into());
    g.set_port_checking_cap(i18n::d_port_checking_cap().into());
    g.set_port_closed(i18n::d_port_closed().into());
    g.set_port_open(i18n::d_port_open().into());
    g.set_port_unknown(i18n::d_port_unknown().into());
    g.set_prevent_sleep(i18n::d_prevent_sleep().into());
    g.set_profile_name(i18n::d_profile_name().into());
    g.set_public_trackers_ph(i18n::d_public_trackers_ph().into());
    g.set_queue_enable(i18n::d_queue_enable().into());
    g.set_queue_max_lbl(i18n::d_queue_max_lbl().into());
    g.set_random_port(i18n::d_random_port().into());
    g.set_reload_note(i18n::d_reload_note().into());
    g.set_renderer_lbl(i18n::d_renderer_lbl().into());
    g.set_save_btn(i18n::d_save_btn().into());
    g.set_sched_to(i18n::d_sched_to().into());
    g.set_sec_additions(i18n::d_sec_additions().into());
    g.set_sec_alt_limits(i18n::d_sec_alt_limits().into());
    g.set_sec_blocklist_ip(i18n::d_sec_blocklist_ip().into());
    g.set_sec_dl_process(i18n::d_sec_dl_process().into());
    g.set_sec_encryption(i18n::d_sec_encryption().into());
    g.set_sec_features(i18n::d_sec_features().into());
    g.set_sec_iface_lang(i18n::d_sec_iface_lang().into());
    g.set_sec_iface_visual(i18n::d_sec_iface_visual().into());
    g.set_sec_lbl(i18n::d_sec_lbl().into());
    g.set_sec_network(i18n::d_sec_network().into());
    g.set_sec_notifications(i18n::d_sec_notifications().into());
    g.set_sec_peers(i18n::d_sec_peers().into());
    g.set_sec_privacy(i18n::d_sec_privacy().into());
    g.set_sec_public_trackers(i18n::d_sec_public_trackers().into());
    g.set_sec_queue(i18n::d_sec_queue().into());
    g.set_sec_remote(i18n::d_sec_remote().into());
    g.set_sec_seeding(i18n::d_sec_seeding().into());
    g.set_sec_speed_limits(i18n::d_sec_speed_limits().into());
    g.set_seed_idle_lbl(i18n::d_seed_idle_lbl().into());
    g.set_seed_ratio_lbl(i18n::d_seed_ratio_lbl().into());
    g.set_seed_ratio_min_lbl(i18n::d_seed_ratio_min_lbl().into());
    g.set_settings_title(i18n::d_settings_title().into());
    g.set_show_opts_dlg(i18n::d_show_opts_dlg().into());
    g.set_sl_pick(i18n::d_sl_pick().into());
    g.set_start_added(i18n::d_start_added().into());
    g.set_start_min_tray(i18n::d_start_min_tray().into());
    g.set_status_lbl(i18n::d_status_lbl().into());
    g.set_sync_state(i18n::d_sync_state().into());
    g.set_tab_downloading(i18n::d_tab_downloading().into());
    g.set_tab_interface(i18n::d_tab_interface().into());
    g.set_tab_network(i18n::d_tab_network().into());
    g.set_tab_privacy(i18n::d_tab_privacy().into());
    g.set_tab_remote(i18n::d_tab_remote().into());
    g.set_tab_seeding(i18n::d_tab_seeding().into());
    g.set_tab_speed(i18n::d_tab_speed().into());
    g.set_tab_system(i18n::d_tab_system().into());
    g.set_trash_torrents(i18n::d_trash_torrents().into());
    g.set_tray_lbl(i18n::d_tray_lbl().into());
    g.set_uifw_lbl(i18n::d_uifw_lbl().into());
    g.set_ulimit_lbl(i18n::d_ulimit_lbl().into());
    g.set_update_freq(i18n::d_update_freq().into());
    g.set_upnp(i18n::d_upnp().into());
    g.set_utp(i18n::d_utp().into());
}
