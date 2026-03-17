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

use rpc::{SessionStats as RpcStats, TransmissionClient};
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
                eprintln!("transmission-gui [--gl|--vk|--sw|--wl]");
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

fn fmt_speed(bps: i64) -> SharedString {
    match bps {
        b if b <= 0        => "—".into(),
        b if b < 1_024     => format!("{b} B/s").into(),
        b if b < 1_048_576 => format!("{:.1} KB/s", b as f64 / 1_024.0).into(),
        b                  => format!("{:.1} MB/s", b as f64 / 1_048_576.0).into(),
    }
}

fn fmt_bytes(bytes: i64) -> SharedString {
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

#[derive(Debug)]
enum Command {
    StartTorrent(i64),
    StopTorrent(i64),
    StartAll,
    StopAll,
    StopDisk(Vec<i64>),
    StartDisk(Vec<i64>),
    RecheckTorrent(i64),
    CreateTorrent(String, Vec<String>),  // path, trackers
    RemoveTorrent(i64, bool),
    AddTorrentUrl(String, Option<String>),
    AddTorrentFile(String, Option<String>, bool), // path, download_dir, delete_after
}

struct Update {
    torrents: Vec<rpc::RawTorrent>,
    stats:    RpcStats,
}

// ── Async backend ─────────────────────────────────────────────────────────────

async fn backend_task(
    client: TransmissionClient,
    mut cmd_rx: mpsc::UnboundedReceiver<Command>,
    update_tx: std::sync::mpsc::SyncSender<Update>,
    status_tx: std::sync::mpsc::SyncSender<String>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut fail_count = 0u32;

    // Кэш всех торрентов — обновляем только изменившиеся
    let mut cache: std::collections::HashMap<i64, rpc::RawTorrent> = std::collections::HashMap::new();
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
    sorted.sort_by(|a, b| torrent_sort_key(a).cmp(&torrent_sort_key(b)));

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

    // Игнорируем SIGHUP — закрытие терминала не убивает приложение
    unsafe { libc::signal(libc::SIGHUP, libc::SIG_IGN) };

    // Режим стража — запущены с --watcher <pid>, не инициализируем UI
    if suspend::maybe_run_as_watcher(&args[1..]) {
        return Ok(());
    }

    // Торрент-файл переданный из проводника: transmission-gui /path/to/file.torrent
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
    // Пишем в ~/transmission-gui.log чтобы видеть крэши при запуске без терминала
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let log_path = format!("{home}/transmission-gui.log");
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

    // ── Инициализируем язык ────────────────────────────────────────────────────
    i18n::init(&app_cfg.language);
    eprintln!("[i18n] language={}", app_cfg.language);

    eprintln!("[app_config] suspend_on_hide={} start_minimized={} refresh={}s autostart={}",
        app_cfg.suspend_on_hide, app_cfg.start_minimized,
        app_cfg.refresh_interval_secs, app_cfg.autostart);
    app_config::sync_autostart(app_cfg.autostart);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    // Каналы создаём до UI — статус-сообщения от daemon нужны сразу
    let (cmd_tx, cmd_rx)       = mpsc::unbounded_channel::<Command>();
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
    rt.spawn(backend_task(client, cmd_rx, update_tx, status_tx.clone()));

    // ── Трей ─────────────────────────────────────────────────────────────────
    let tray = if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_ok() {
        match tray::AppTray::build(&rt) {
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
                match filepicker::pick_directory() {
                    Ok(path) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui2.upgrade() {
                                ui.set_pending_create_path(path.into());
                                ui.set_create_dialog_visible(true);
                            }
                        });
                    }
                    Err(e) => eprintln!("[create] pick dir: {e}"),
                }
            });
        });
    }
    {
        let tx = cmd_tx.clone();
        ui.on_add_torrent_url(move |url| {
            let s = url.trim().to_string();
            if !s.is_empty() { let _ = tx.send(Command::AddTorrentUrl(s, None)); }
        });
    }
    {
        let tx = cmd_tx.clone();
        let ui_weak = ui.as_weak();
        let delete_after = app_cfg.delete_torrent_after_add;
        ui.on_pick_torrent_file(move || {
            let tx2 = tx.clone(); let ui2 = ui_weak.clone();
            std::thread::spawn(move || {
                // Шаг 1: выбрать .torrent файл
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
                // Шаг 2: выбрать папку назначения (обязательно)
                match filepicker::pick_directory() {
                    Ok(dir) => { let _ = tx2.send(Command::AddTorrentFile(path, Some(dir), delete_after)); }
                    Err(_)  => { /* отмена — не добавляем */ }
                }
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
    let handle_arc = std::sync::Arc::new(std::sync::Mutex::new(Some(daemon_handle)));
    let cfg_arc    = std::sync::Arc::new(active_cfg.clone());

    // do_quit — останавливаем демон в отдельном треде, потом quit event loop
    let do_quit: std::sync::Arc<dyn Fn() + Send + Sync> = {
        let ha = handle_arc.clone();
        let ca = cfg_arc.clone();
        std::sync::Arc::new(move || {
            let ha2 = ha.clone();
            let ca2 = ca.clone();
            std::thread::spawn(move || {
                if let Ok(mut lock) = ha2.lock() {
                    if let Some(h) = lock.take() {
                        daemon::stop_daemon(h, &ca2);
                    }
                }
                slint::quit_event_loop().ok();
            });
        })
    };

    {
        let ui_weak  = ui.as_weak();
        let dq = do_quit.clone();
        ui.on_do_quit(move || {
            let ui2  = ui_weak.clone();
            let dq2  = dq.clone();
            slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui2.upgrade() { ui.window().hide().ok(); }
                dq2();
            }).ok();
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
        move || {
            while let Ok(msg) = status_rx.try_recv() {
                if let Some(ui) = ui_h.upgrade() {
                    let ok = msg.starts_with("Connected");
                    ui.set_connected(ok);
                    ui.set_status_bar_text(msg.into());
                }
            }
            if !tray_ready.load(std::sync::atomic::Ordering::Relaxed) {
                // Дренируем события пока не готовы — игнорируем буферизованные клики
                if let Some(ref tray) = tray { tray.poll_events(); }
                return;
            }
            if let Some(ref tray) = tray {
                let (toggle, quit) = tray.poll_events();
                if toggle {
                    if let Some(ui) = ui_h.upgrade() {
                        let win = ui.window();
                        if win.is_visible() {
                            // Скрываем через callback — он обработает suspend
                            ui.invoke_do_minimize_tray();
                        } else {
                            // Показываем напрямую — suspend не нужен
                            win.show().ok();
                        }
                    }
                }
                if quit {
                    if let Some(ui) = ui_h.upgrade() { ui.window().hide().ok(); }
                    do_quit_tmr();
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

    // Кэш: download_dir → /dev/sdX — заполняется в фоне, читается в event loop
    let path_disk_cache: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Option<String>>>>
        = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    _tmr_data.start(slint::TimerMode::Repeated, Duration::from_millis(500), move || {
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

            // Применяем поиск поверх disk_filtered
            let query = ui_h.upgrade()
                .map(|u| u.get_search_text().to_string().to_lowercase())
                .unwrap_or_default();
            let filtered_refs: Vec<&rpc::RawTorrent> = if query.is_empty() {
                after_disk.iter().collect()
            } else {
                after_disk.iter().filter(|t| t.name.to_lowercase().contains(&query)).collect()
            };

            apply_torrent_update(&mdl, &filtered_refs);

            // Сохраняем disk_filtered для мгновенного поиска (после apply чтобы не конфликтовать с borrow)
            *disk_filtered.lock().unwrap() = after_disk;
            if let Some(ui) = ui_h.upgrade() {
                let s = &upd.stats;
                ui.set_stats(SessionStats {
                    down_speed:  fmt_speed(s.down_speed),
                    up_speed:    fmt_speed(s.up_speed),
                    downloaded:  fmt_bytes(s.downloaded),
                    uploaded:    fmt_bytes(s.uploaded),
                    ratio:       fmt_ratio(s.ratio),
                    active:      s.active_count as i32,
                });
            }
        }
    });

    if app_cfg.start_minimized {
        ui.show()?;
        ui.window().hide().ok();
    } else {
        ui.show()?;
    }

    // Открытие .torrent из проводника — запускаем выбор папки сразу после старта UI
    if let Some(torrent_path) = pending_torrent {
        let tx = cmd_tx.clone();
        let delete_after = app_cfg.delete_torrent_after_add;
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            match filepicker::pick_directory() {
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
        single_instance::start_listener(listener, move |torrent_path| {
            let tx2 = tx.clone();
            let ui2 = ui_weak.clone();
            let delete_after2 = delete_after;
            // Поднимаем окно
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui2.upgrade() { ui.show().ok(); }
            });
            if torrent_path.is_empty() { return; }
            // Выбор папки и добавление
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(300));
                match filepicker::pick_directory() {
                    Ok(dir) => {
                        eprintln!("[open] Adding {torrent_path} → {dir}");
                        let _ = tx2.send(Command::AddTorrentFile(torrent_path, Some(dir), delete_after2));
                    }
                    Err(_) => eprintln!("[open] Directory selection cancelled"),
                }
            });
        });
    }

    slint::run_event_loop_until_quit()?;

    // daemon stopped in do_quit() before event loop exit
    Ok(())
}
